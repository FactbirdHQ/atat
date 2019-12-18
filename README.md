# `AT Parser`

> A driver support crate for AT-command based serial modules, using the [embedded-hal] traits.


[embedded-hal]: https://crates.io/crates/embedded-hal

## [Documentation](https://docs.rs/at-rs/latest/at-rs/)

## Tests

> The crate is covered by tests using the [embedded-hal-mock] crate. These tests can be run by `cargo test --lib --target = x86_64-unknown-linux-gnu` or the `cargo th` alias.

[embedded-hal-mock]: https://crates.io/crates/embedded-hal-mock

## Examples

> The crate has examples for usage with [cortex-m-rt] and [cortex-m-rtfm] crates.

> Furthermore I have used the crate to build initial drivers for uBlox cell modules ([ublox-cell-rs]) and uBlox wifi modules ([ublox-wifi-rs])

[cortex-m-rt]: https://crates.io/crates/cortex-m-rt
[cortex-m-rtfm]: https://crates.io/crates/cortex-m-rtfm
[ublox-wifi-rs]: https://crates.io/crates/ublox-wifi-rs
[ublox-cell-rs]: https://crates.io/crates/ublox-cell-rs

## About

    - Minimum rustc version 1.31
    - Tested and built using nightly toolchain, but should work fine for stable as well

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
