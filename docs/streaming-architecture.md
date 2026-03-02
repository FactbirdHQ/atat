# Streaming Response Architecture for atat — Implementation Plan

## Context

GitHub issue #89: atat buffers entire AT responses before parsing, forcing large static buffers. For commands like `+USORD`, `+UWSCAN`, memory cost is ~3x the max response size (`ResponseSlot(Vec<u8, N>)` + ingress buffer + client buf).

This plan implements a bbqueue-style framed ring buffer replacing `ResponseSlot`, enabling chunked response delivery, zero-copy parsing, and a new `send_streaming()` API. Target: ~60% memory reduction.

The plan incorporates 6 refinements over the original architecture doc (`docs/streaming-architecture.md`):
1. **Safe-point yielding** — Digester splits only at field delimiters (`,`, `:`, `\r\n`), guaranteeing serde_at never sees a field spanning the buffer boundary
2. **Greedy URC guard** — Digester holds back data when buffer ends with potential URC prefix (`\r\n+`)
3. **Atomic reset** — `reset()` atomically clears ring buffer, `in_response_flag`, and all grants
4. **Custom error payloads** — Error strings stored in ring buffer data region
5. **Dynamic streaming via GAT** — `Response<'a>` GAT supports both owned and streaming responses
6. **`Error::SpanningField`** — Dedicated serde_at error variant for boundary-spanning fields

## Phase 1: Response Ring Buffer Channel

**Create** `atat/src/response_channel.rs`
**Modify** `atat/src/lib.rs` — add `pub mod response_channel;`, re-exports

### Data structures

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FrameKind { Complete, Start, Partial, End }

#[derive(Clone, Copy)]
pub enum ResponseMeta {
    Data(FrameKind),
    Error(InternalErrorCode),  // header-only for non-Custom; data region for Custom payload
    Prompt(u8),
}

/// Owned error code — no lifetime. Custom error payload bytes live in the frame's data region.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InternalErrorCode {
    Error, CmeError(u16), CmsError(u16), ConnectionError(u8),
    Custom,  // R4: payload in ring buffer data region
}

struct FrameHeader {
    start: usize,
    len: u16,
    meta: ResponseMeta,
}
```

### Channel state

```rust
pub struct ResponseChannel<const BUF_SIZE: usize, const MAX_FRAMES: usize> {
    inner: Mutex<CriticalSectionRawMutex, RefCell<Inner<BUF_SIZE, MAX_FRAMES>>>,
    signal: Signal<CriticalSectionRawMutex, ()>,
    in_response_flag: AtomicBool,  // R3: shared with Digester, cleared by reset()
}

struct Inner<const BUF_SIZE: usize, const MAX_FRAMES: usize> {
    data: [u8; BUF_SIZE],
    headers: heapless::Deque<FrameHeader, MAX_FRAMES>,
    write_pos: usize,
    read_pos: usize,
}
```

### Publisher API (held by Ingress)

- `grant(max_sz) -> FrameGrantW` — reserves space, wraps to 0 if insufficient tail room
- `commit(grant, actual_len, meta)` — finalizes, wakes subscriber
- `signal_error(code)` — zero-data frame (header only)
- `signal_error_with_payload(code, payload: &[u8])` — R4: grants space, copies payload, commits with `Error(Custom)`
- `signal_prompt(p)` — zero-data frame
- `in_response_active() -> bool` — reads `in_response_flag` (R3: Ingress guard)

### Subscriber API (held by Client)

- `read() -> Option<FrameGrantR>` / `read_async() -> FrameGrantR`
- `FrameGrantR`: deref to `&[u8]`, `.meta()`, `.release()`. **Must impl `Drop`** — releases ring buffer space, wakes publisher
- `try_merge(self, other) -> Result<FrameGrantR, (FrameGrantR, FrameGrantR)>` — merge adjacent grants
- `reset()` — R3: atomically clears `in_response_flag`, ring buffer pointers, pending grants, wakes publisher

### Ring buffer wrapping

When grant doesn't fit at tail, mark remaining tail with sentinel, start grant at 0. Port pattern from embedded-mqtt `pubsub/publisher.rs:170-233`.

### Invariants

- **`FrameGrantR::Drop`**: Load-bearing. Without it, timeout-cancelled futures leak ring buffer space permanently.
- **R3 `reset()` atomicity**: Lock `inner`, clear headers + pointers, store `false` to `in_response_flag` (Release ordering). Single critical section.
- **R3 Ingress guard**: Publisher checks `in_response_flag` before `grant()`. If cleared by client reset, stop yielding partial data.

### Verification

- grant/commit/read/release cycle, buffer wrapping, back-pressure
- `signal_error_with_payload` stores payload, subscriber reads via deref
- `reset()` clears `in_response_flag`, reclaims all space
- `FrameGrantR::Drop` releases space
- `try_merge` success (adjacent) and failure (wrap sentinel)

---

## Phase 2: Digester Streaming Support

**Modify** `atat/src/digest.rs`

### DigestResult extension (breaking)

```rust
pub enum DigestResult<'a> {
    Urc(&'a [u8]),
    Response(Result<&'a [u8], InternalError<'a>>),
    ResponseData { data: &'a [u8], complete: bool },  // NEW
    Prompt(u8),
    None,
}
```

All match sites need updating: `ingress.rs`, `simple_client.rs`, downstream user code.

### AtDigester state

```rust
pub struct AtDigester<P: Parser> {
    _urc_parser: PhantomData<P>,
    custom_success: fn(&[u8]) -> Result<(&[u8], usize), ParseError>,
    custom_error: fn(&[u8]) -> Result<(&[u8], usize), ParseError>,
    custom_prompt: fn(&[u8]) -> Result<(u8, usize), ParseError>,
    in_response: *const AtomicBool,  // raw ptr avoids lifetime infection; valid for 'static channel
}
```

Constructor gains `&AtomicBool` param. Add `new_standalone()` for backward compat / testing (points to dummy always-false atomic).

### Digester trait extension

```rust
pub trait Digester {
    fn digest<'a>(&mut self, buf: &'a [u8]) -> (DigestResult<'a>, usize);
    fn digest_with_pressure<'a>(&mut self, buf: &'a [u8], pressure: bool) -> (DigestResult<'a>, usize) {
        let _ = pressure;
        self.digest(buf)  // default: ignore pressure
    }
}
```

### R1: Safe-point yielding in `digest_with_pressure`

When `pressure` is true and `self.digest(buf)` returns `None`:

**Case `!in_response`**: After all standard checks fail (echo, URC, success, prompt, error, garbage), the remaining buffer is response data without a terminator. Search backwards for last safe delimiter:

```rust
fn find_safe_yield_point(buf: &[u8]) -> Option<usize> {
    let mut pos = buf.len();
    while pos > 0 {
        pos -= 1;
        match buf[pos] {
            b',' | b':' => return Some(pos + 1),
            b'\n' if pos > 0 && buf[pos - 1] == b'\r' => {
                let candidate = pos + 1;  // after \r\n
                // R2: Greedy URC guard — if remaining looks like URC start, yield up to \r\n
                if candidate < buf.len() && buf[candidate] == b'+' {
                    return Some(pos - 1);  // yield up to before \r
                }
                return Some(candidate);
            }
            _ => {}
        }
    }
    None  // no safe point → SpanningField
}
```

- If safe point found: yield `ResponseData { data: &buf[..safe_point], complete: false }`, set `in_response = true`
- If no safe point: entire buffer is one huge field. Return `Response(Err(InternalError::Custom(b"SpanningField")))` — user must increase `INGRESS_BUF_SIZE` or use `Streaming<'a>`

**Case `in_response`**:
- Still run URC checks first (digest order preserved: echo → URC → ...)
- Check terminators: `\r\nOK\r\n`, `\r\nERROR\r\n`, `+CME ERROR`, `+CMS ERROR`, etc.
- If terminator found: yield `ResponseData { data: &buf[..terminator_start], complete: true }`, set `in_response = false`
- If no terminator + pressure: find safe yield point, yield as `Partial`
- If no terminator + no pressure: return `None`

**R3 guard at top of `digest_with_pressure`**: Check `in_response_flag.load(Acquire)`. If was true but now false (externally cleared by reset), return `None` — let Ingress handle cleanup.

### Verification

- All existing digest tests pass unchanged (no pressure = old behavior)
- Pressure-triggered chunking at `,`, `:`, `\r\n`
- No safe point → error
- R2: URC guard holds data when buffer ends with `\r\n+`
- Error mid-stream detection
- R3: reset clears `in_response`

---

## Phase 3: serde_at Two-Buffer Deserializer

**Modify** `serde_at/src/de/mod.rs`

R1 (safe-point yielding) makes this phase much simpler: the ring buffer wrap-around always falls between AT fields, so `subslice()` spanning errors are a safety net, not a normal path.

### Deserializer struct change

```rust
pub(crate) struct Deserializer<'a> {
    first: &'a [u8],
    second: &'a [u8],   // empty for single-buffer case
    index: usize,
    struct_size_hint: Option<usize>,
    is_trailing_parsing: bool,
}
```

### New public API

```rust
pub fn from_slice_pair<'a, T: de::Deserialize<'a>>(a: &'a [u8], b: &'a [u8]) -> Result<T>
pub fn from_slice<'a, T: de::Deserialize<'a>>(s: &'a [u8]) -> Result<T>  // delegates to from_slice_pair(s, &[])
```

### R6: SpanningField error

Add variant to `serde_at::de::Error`:
```rust
/// A field spans the ring buffer boundary. Increase INGRESS_BUF_SIZE or use Streaming.
SpanningField,
```

### Helper methods

```rust
fn len(&self) -> usize { self.first.len() + self.second.len() }
fn get(&self, idx: usize) -> Option<u8> { ... }  // index across both slices
fn subslice(&self, range: Range<usize>) -> Result<&'a [u8]> { ... }  // Err(SpanningField) if range spans boundary
```

### Access sites to modify (all in `serde_at/src/de/mod.rs`)

| Line | Current | Change |
|------|---------|--------|
| 77 | `fn new(slice)` | Add `fn new_pair(first, second)`. `new` delegates to `new_pair(slice, &[])` |
| 97-103 | `self.slice.get(self.index)` → `Option<&u8>` | `self.get(self.index)` → `Option<u8>`. `next_char` returns `Option<u8>`. Fix callers comparing `Some(c)` at line 115 |
| 126-127 | `self.slice.len()`, `&self.slice[start..]` | `self.len()`, `self.subslice(start..self.len())?` |
| 147 | `self.slice[index - count - 1]` | `self.get(index - count - 1).unwrap()` |
| 161 | `&self.slice[start..end]` | `self.subslice(start..end)?` |
| 178-179 | `self.slice.len()`, `&self.slice[start..]` | `self.len()`, `self.subslice(start..self.len())?` |
| 187 | `&self.slice[start..self.index]` | `self.subslice(start..self.index)?` |
| 232 | `self.slice.get(self.index).copied()` | `self.get(self.index)` |
| 320 | `&$self.slice[start..$self.index]` | `$self.subslice(start..$self.index)?` |
| 497-500 | `self.slice[self.index..].iter().position(...)` | Loop using `self.get()` |
| 503 | `&self.slice[self.index..self.index + idx]` | `self.subslice(self.index..self.index + idx)?` |
| 574 | `self.slice[self.index..].as_ref()` | `self.subslice(self.index..self.len())?` with `SpanningField` fallback |
| 576 | `self.index = self.slice.len()` | `self.index = self.len()` |
| 618 | `self.index == self.slice.len()` | `self.index == self.len()` |

Also need `trim_ascii_whitespace_pair(a, b) -> (&[u8], &[u8])` helper.

### Why SpanningField is rare

With R1, the Digester only splits at field delimiters. Numbers are parsed byte-by-byte (never need `subslice`). Quoted strings have delimiter (comma) before opening quote, so the field is in one buffer. `LengthDelimited` uses `deserialize_tuple` which gets all remaining bytes — large payloads should use streaming.

### Verification

- All existing `from_slice` / `from_str` tests pass unchanged
- `from_slice_pair` with split at comma (field delimiter)
- Numbers across boundary (byte-by-byte parsing works)
- `SpanningField` error on artificial spanning case

---

## Phase 4: Ingress Integration

**Modify** `atat/src/ingress.rs`

### Struct change

```rust
pub struct Ingress<'a, D: Digester, Urc: AtatUrc,
    const BUF_SIZE: usize, const MAX_FRAMES: usize,
    const URC_CAPACITY: usize, const URC_SUBSCRIBERS: usize,
> {
    digester: D,
    buf: &'a mut [u8],
    pos: usize,
    res_publisher: ResponsePublisher<'a, BUF_SIZE, MAX_FRAMES>,  // was: res_slot
    urc_publisher: UrcPublisher<'a, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    first_chunk: bool,
}
```

Constructor takes `&'a ResponseChannel<BUF_SIZE, MAX_FRAMES>` instead of `&'a ResponseSlot<RES_BUF_SIZE>`.

### try_advance / advance changes

```
pressure = self.pos > self.buf.len() * 3 / 4
// R3: Ingress guard — if in_response was cleared externally, discard partial state
if !self.res_publisher.in_response_active() { self.first_chunk = true; }
result = self.digester.digest_with_pressure(&self.buf[..self.pos], pressure)
```

Routing:
- `Response(Ok(data))` → grant, copy, commit as `FrameKind::Complete`
- `Response(Err(InternalError::Custom(payload)))` → R4: `signal_error_with_payload(Custom, payload)`
- `Response(Err(e))` → `signal_error(e.into())`
- `ResponseData { data, complete: true }` → grant, copy, commit as `End` (or `Complete` if `first_chunk`)
- `ResponseData { data, complete: false }` → grant, copy, commit as `Start` (if `first_chunk`) or `Partial`
- `Prompt(p)` → `signal_prompt(p)`
- `Urc(urc)` → unchanged
- `None` → unchanged

### Breaking changes

- Const generics: `RES_BUF_SIZE` → `BUF_SIZE, MAX_FRAMES`
- Constructor: `ResponseSlot` → `ResponseChannel`
- `ingress::Error` gains `ResponseChannelFull`

### Verification

- Adapt existing ingress tests
- Large response in small increments → correct frame sequence
- Back-pressure when channel full
- R4: custom error payload flows through
- R3: reset mid-response → ingress discards partial data

---

## Phase 5: AtatCmd GAT + Client Integration

### Part A: AtatCmd + AtatResp GAT migration

**Modify** `atat/src/traits.rs`:
```rust
pub trait AtatResp {
    /// Whether this response type contains streaming fields (Streaming<'a> or LengthDelimitedStream<'a>).
    /// Set automatically by the AtatResp derive macro via last-path-segment detection.
    const STREAMING: bool = false;
}

pub trait AtatCmd {
    type Response<'a>: AtatResp where Self: 'a;
    // ... other consts unchanged (no STREAMING here — it lives on AtatResp) ...
    fn write(&self, buf: &mut [u8]) -> usize;
    fn parse<'a>(&self, resp: Result<(&[u8], &[u8]), InternalError>, ctx: ParseContext<'a>) -> Result<Self::Response<'a>, Error>;
}

pub struct ParseContext<'a> {
    pub subscriber: Option<&'a dyn DynResponseSubscriber>,
}
```

The client reads `STREAMING` from the response type: `<Cmd::Response<'static> as AtatResp>::STREAMING` (const doesn't depend on lifetime).

**Modify** `atat/src/asynch/mod.rs`:
```rust
pub trait AtatClient {
    async fn send<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response<'_>, Error>;
}
```

**Modify** `atat/src/blocking/mod.rs` — same GAT pattern.

**Modify** `atat/src/traits.rs` — update `String<L>` impl to new signature (non-streaming: `const STREAMING: bool = false` is the default).

**Modify** `atat_derive/src/cmd.rs`:
```rust
// Generated for ALL commands (streaming and non-streaming):
type Response<'a> = #resp;
fn parse<'a>(&self, res: Result<(&[u8], &[u8]), atat::InternalError>, _ctx: atat::ParseContext<'a>) -> ... {
    match res {
        Ok((a, b)) => atat::serde_at::from_slice_pair::<#resp>(a, b).map_err(|_| atat::Error::Parse),
        Err(e) => Err(e.into())
    }
}
```

No `streaming` keyword needed on `#[at_cmd]` — the client reads `STREAMING` from the response type.

**Modify** `atat_derive/src/resp.rs`:
- Detect streaming fields via `#[at_arg(streaming)]` attribute (no type name detection)
- When any field has `#[at_arg(streaming)]`, generate `const STREAMING: bool = true` in `AtatResp` impl
- When streaming fields found, generate `from_streaming()` method (see Phase 6 derive macro section)
- Compile-time safety (both directions):
  - Forgetting `#[at_arg(streaming)]` on a `Streaming<'a>` field → compile error: `Streaming<'a>: Deserialize` not satisfied (because `Streaming` deliberately does not impl `Deserialize`)
  - Putting `#[at_arg(streaming)]` on a non-streaming field like `u8` → compile error: `u8: StreamingField` not satisfied

### Part B: Client rewiring

**Modify** `atat/src/asynch/client.rs`:
```rust
pub struct Client<'a, W: Write, const BUF_SIZE: usize, const MAX_FRAMES: usize> {
    writer: W,
    res_subscriber: ResponseSubscriber<'a, BUF_SIZE, MAX_FRAMES>,
    buf: &'a mut [u8],  // command serialization only (sized for max Cmd::MAX_LEN, not responses)
    config: Config,
    cooldown_timer: Option<Timer>,
}
```

`send()` flow:
1. `cmd.write(&mut self.buf)` → send
2. `self.res_subscriber.reset()` before send (clears stale frames)
3. If `!Cmd::EXPECTS_RESPONSE_CODE`: `cmd.parse(Ok((&[], &[])), ParseContext::none())`
4. `with_timeout`: `collect_and_parse(cmd)`

`collect_and_parse` for non-streaming:
1. Read first frame
2. `Complete` → `cmd.parse(Ok((&grant, &[])), ParseContext::none())`. Zero-copy.
3. `Error(code)` → R4: if `Custom`, read payload from grant deref as `InternalError::Custom(&*frame)`. Otherwise map code to `InternalError`.
4. `Prompt(p)` → handle prompt
5. `Start`/`Partial`/`End` → merge grants via `try_merge` into at most two (pre-wrap, post-wrap). Parse: `cmd.parse(Ok((&grant_a, grant_b_or_empty)), ParseContext::none())`

`collect_and_parse` for streaming (`<Cmd::Response<'static> as AtatResp>::STREAMING`):
1. Read header frame only
2. Parse with `ParseContext { subscriber: Some(&self.res_subscriber) }`
3. Return immediately — `Streaming` field reads remaining chunks

**Modify** `atat/src/blocking/client.rs` — mirror with `try_read()` polling.

**Modify** `atat/src/asynch/simple_client.rs` — add `DigestResult::ResponseData` match arm. Accumulate in own buffer. When `complete: true`, treat as final. No streaming support.

### Breaking changes

- `AtatResp` gains `const STREAMING: bool = false` (non-breaking default, but streaming response types override to `true`)
- `AtatCmd::Response` → `Response<'a>` GAT
- `AtatCmd::parse()` signature: `Result<&[u8], InternalError>` → `Result<(&[u8], &[u8]), InternalError>` + `ParseContext`
- `AtatClient::send()` returns `Cmd::Response<'_>`
- `Client` const generics change
- Constructor: `ResponseSlot` → `ResponseChannel`

### Verification

- Adapt all existing client tests
- Complete response zero-copy path
- Multi-frame reassembly via try_merge
- R4: Custom error with payload
- Timeout during multi-frame (grants dropped via RAII)

---

## Phase 6: Streaming Types

**Create** `atat/src/streaming.rs`
**Modify** `atat/src/lib.rs` — add `pub mod streaming;`

### Type erasure

```rust
pub(crate) trait DynResponseSubscriber {
    fn read_with_context(&self, cx: Option<&mut Context<'_>>) -> Poll<Option<FrameReadResult>>;
    fn release(&self);
}
// ResponseSubscriber<B, F> implements DynResponseSubscriber
```

### Streaming type

```rust
pub struct Streaming<'a> {
    subscriber: &'a dyn DynResponseSubscriber,
    current_grant: Option<FrameGrantR>,
    done: bool,
}
impl<'a> Streaming<'a> {
    pub async fn next(&mut self) -> Option<Result<&[u8], Error>> { ... }
    pub fn try_next(&mut self) -> Option<Result<&[u8], Error>> { ... }
}
impl Drop for Streaming<'_> {
    // Release current grant, drain remaining frames
}
```

### LengthDelimitedStream type

```rust
pub struct LengthDelimitedStream<'a, const S: usize = 1> {
    subscriber: &'a dyn DynResponseSubscriber,
    current_grant: Option<FrameGrantR>,
    expected_len: usize,
    received: usize,
    done: bool,
}
// Drop impl mirrors Streaming::drop
```

### Derive macro changes

**No changes to `atat_derive/src/cmd.rs`** for streaming — `#[at_cmd]` needs no `streaming` keyword. The generated parse code is identical for streaming and non-streaming commands. The client reads `AtatResp::STREAMING` from the response type to decide the collection strategy.

**Modify** `atat_derive/src/parse.rs`:
- Add `streaming: bool` field to `ArgAttributes` (alongside existing `position`, `len`, `value`, `default`)
- Parse `streaming` keyword in `#[at_arg(streaming)]`

**Modify** `atat_derive/src/resp.rs` — streaming field detection via attribute:
- Check each field's `ArgAttributes` for `streaming = true`
- No type name detection — purely attribute-driven
- Compile-time safety:
  - Forgetting `#[at_arg(streaming)]` on a `Streaming<'a>` field → `Streaming<'a>: Deserialize` not satisfied (compile error)
  - Putting `#[at_arg(streaming)]` on a `u8` field → `u8: StreamingField` not satisfied (compile error)
- When any field has `#[at_arg(streaming)]`, generate:
  1. `const STREAMING: bool = true` in the `AtatResp` impl
  2. A private `__Partial` struct with only non-streaming fields (streaming fields excluded) + `Deserialize` impl
  3. A `from_streaming(a: &[u8], b: &[u8], subscriber: &dyn DynResponseSubscriber) -> Result<Self, Error>` method that deserializes `__Partial` via `from_slice_pair`, then constructs the full struct injecting streaming fields via `StreamingField::from_subscriber(subscriber)` trait call

**Trait for streaming field construction** (in `atat/src/streaming.rs`):
```rust
/// Marker trait for fields constructed from a streaming subscriber, not from deserialization.
/// Implemented by `Streaming<'a>` and `LengthDelimitedStream<'a>`.
///
/// NOT implementing `serde::Deserialize` is load-bearing for compile-time safety.
/// This is enforced structurally: private fields contain `&'a dyn DynResponseSubscriber`
/// (non-deserializable), and no public zero-arg constructor exists. A compile-fail test
/// verifies this invariant.
pub trait StreamingField<'a>: Sized {
    fn from_subscriber(subscriber: &'a dyn DynResponseSubscriber) -> Self;
}
```

Usage:
```rust
#[derive(AtatResp)]
pub struct SocketData<'a> {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1, streaming)]
    pub data: LengthDelimitedStream<'a>,
}
```

### Verification

- Stream LengthDelimitedStream, verify all bytes
- Error mid-stream
- Back-pressure (slow consumer)
- Borrow checker prevents sending during active stream
- Compile-fail test (`trybuild`): `Streaming<'a>` does NOT satisfy `Deserialize` (structural enforcement of the non-Deserialize invariant)
- Compile-fail test: forgetting `#[at_arg(streaming)]` on a `Streaming` field → compile error
- Compile-fail test: `#[at_arg(streaming)]` on a `u8` field → compile error

---

## Dependency Graph

```
Phase 1 (ResponseChannel) ──┐
                             ├─→ Phase 4 (Ingress) ─→ Phase 5 (GAT + Client) ─→ Phase 6 (Streaming)
Phase 2 (Digester)      ────┘         ↑                       ↑
Phase 3 (serde_at)      ─────────────┘───────────────────────┘
```

Phases 1, 2, 3 are independent — can be developed in parallel.

## Critical Files

| Phase | File | Action |
|-------|------|--------|
| 1 | `atat/src/response_channel.rs` | Create |
| 1 | `atat/src/lib.rs` | Modify |
| 2 | `atat/src/digest.rs` | Modify |
| 3 | `serde_at/src/de/mod.rs` | Modify |
| 4 | `atat/src/ingress.rs` | Modify |
| 5 | `atat/src/traits.rs` | Modify |
| 5 | `atat/src/asynch/client.rs` | Modify |
| 5 | `atat/src/asynch/mod.rs` | Modify |
| 5 | `atat/src/blocking/client.rs` | Modify |
| 5 | `atat/src/blocking/mod.rs` | Modify |
| 5 | `atat/src/asynch/simple_client.rs` | Modify |
| 5 | `atat_derive/src/cmd.rs` | Modify |
| 5+6 | `atat_derive/src/resp.rs` | Modify (STREAMING const + from_streaming) |
| 6 | `atat_derive/src/parse.rs` | Modify (add `streaming` to ArgAttributes) |
| 6 | `atat/src/streaming.rs` | Create |
| 6 | `atat/src/lib.rs` | Modify |
| 6 | `atat/src/asynch/client.rs` | Modify |

## Pre-Implementation: Update Architecture Doc

**Modify** `docs/streaming-architecture.md` to incorporate all refinements from this plan:
- R1: Safe-point yielding (Digester splits at field delimiters)
- R2: Greedy URC guard (hold back `\r\n+` prefix)
- R3: Atomic reset (ResponseChannel + in_response_flag + grants)
- R4: Custom error payloads (InternalErrorCode::Custom with ring buffer data)
- R5: STREAMING const on AtatResp (not AtatCmd)
- R6: Error::SpanningField in serde_at
- Streaming field detection via `#[at_arg(streaming)]` (no type name detection)
- `StreamingField<'a>` trait with structural non-Deserialize enforcement
- Compile-fail tests for safety invariants

---

## Verification Strategy

After each phase: `cargo test` in `atat/` and `serde_at/`. After Phase 5: `cargo build --target thumbv7em-none-eabihf` for no_std.

End-to-end after Phase 5:
1. Set up ResponseChannel + Ingress + Client with small buffers (128B ingress, 512B ring buffer)
2. Feed response larger than ingress buffer
3. Verify client receives and parses correctly
4. Verify memory usage via const generic sizing

## Risks

1. **Ring buffer correctness** — Mitigated by porting proven embedded-mqtt pattern. Single-subscriber simplifies.
2. **URC interleaving mid-response** — R2 URC guard + digest order always checks URCs first.
3. **Breaking changes** — Accept as semver-breaking (0.25.0).
4. **Spanning boundary** — R1 safe-point yielding prevents in practice. R6 `SpanningField` error is safety net.
5. **Stale `in_response`** — R3 `AtomicBool` cleared by `reset()`.
6. **Grant lifecycle** — `FrameGrantR::Drop` and `Streaming::Drop` ensure cleanup. Timeout recovery depends on this.
7. **Non-streaming buffer sizing** — `BUF_SIZE` >= max response. Undersized → `Error::Timeout` (recoverable).
8. **`digest_with_pressure` false positive** — R1 safe-point yielding + R2 URC guard minimize risk. 75% pressure threshold provides headroom.
