# Examples

There are two sets of examples: an example for `embassy` / `no-std` and examples for `tokio` / `std`.
The `embassy` example depends on the feature `embedded`, the `tokio` examples depend on the `std` feature
in the [Cargo.toml](./Cargo.toml).

| Binary           | Feature    | Description                                                                 |
|------------------|------------|-----------------------------------------------------------------------------|
| `embassy`        | `embedded` | RP2040 target using `embassy-rp` buffered UART, flashed with `probe-run`    |
| `std-tokio`      | `std`      | Host example talking to a `tokio-serial` pseudo-terminal pair               |
| `std-tokio-mock` | `std`      | Host example with an in-process mock modem replying with a canned `+CME ERROR` |

## `std`

```sh
cargo run --bin std-tokio --features std
cargo run --bin std-tokio-mock --features std
```

If you want to adapt the `tokio` examples for your own `std` application,
make sure to add the following dependencies to your `Cargo.toml`:

```toml
embassy-time = { version = "0.5", features = ["std", "generic-queue-8"] }
critical-section = { version = "1", features = ["std"] }
```

`embassy-time` is a dependency of `atat` even in a `tokio` context. The `std` feature provides a
time driver so it does not depend on the embassy executor, and `generic-queue-8` provides a timer queue.
`critical-section` needs an implementation because `atat`'s `UrcChannel` and `ResponseSlot` are built on
`embassy-sync` primitives using `CriticalSectionRawMutex`. Without it the binary fails to link with an
undefined `_critical_section_1_0_acquire` symbol.
For more details, refer to the [`embassy-time` documentation](https://docs.rs/embassy-time/latest/embassy_time/).

## `embassy`

```sh
cargo run --bin embassy --features embedded --target thumbv6m-none-eabi
```

The runner and linker flags are configured in [`.cargo/config.toml`](../.cargo/config.toml).
