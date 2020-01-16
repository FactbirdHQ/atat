# `AT Parser`

[![Build status][workflow-badge]][workflow]
[![Crates.io Version][crates-io-badge]][crates-io]
[![Crates.io Downloads][crates-io-download-badge]][crates-io-download]

> A driver support crate for AT-command based serial modules, using the [embedded-hal] traits.


[embedded-hal]: https://crates.io/crates/embedded-hal

## [Documentation](https://docs.rs/at-rs/latest)

## Tests

> The crate is covered by tests using the [embedded-hal-mock] crate. These tests can be run by `cargo test --lib --target x86_64-unknown-linux-gnu`.

[embedded-hal-mock]: https://crates.io/crates/embedded-hal-mock

## Examples

The crate has examples for usage with [cortex-m-rt] and [cortex-m-rtfm] crates.

The samples can be built using `cargo build --example cortex-m-rt --target thumbv7em-none-eabihf` and `cargo build --example rtfm --target thumbv7em-none-eabihf`.

Furthermore I have used the crate to build initial drivers for U-Blox short-range modules ([ublox-short-range-rs])
<!-- Furthermore I have used the crate to build initial drivers for uBlox cell modules ([ublox-cell-rs]) and uBlox short-range modules ([ublox-short-range-rs]) -->

[cortex-m-rt]: https://crates.io/crates/cortex-m-rt
[cortex-m-rtfm]: https://crates.io/crates/cortex-m-rtfm
[ublox-short-range-rs]: https://github.com/BlackbirdHQ/ublox-short-range-rs
<!-- [ublox-cell-rs]: https://crates.io/crates/ublox-cell-rs -->

## About

    - Minimum rustc version 1.31
    - Tested and built using nightly toolchain, but should work fine for stable as well

## Supported Crates

The following dependent crates provide platform-agnostic device drivers built on `embedded-hal` which also implement this crate's [`ATCommandInterface`] trait:

| Device Name | Description | Crate + Docs |
|-------------|-------------|--------------|
| [ublox-short-range-rs]  | Driver crate for U-Blox host-based short range devices (wifi and BT) with AT-command interface | <!--[![crates.io][ublox-short-range-rs-crate-img]][ublox-short-range-rs] [![docs.rs][ublox-short-range-rs-docs-img]][ublox-short-range-rs-docs] --> |
<!-- | [ublox-cell-rs]  | Driver crate for U-Blox host-based cellular devices with AT-command interface | [![crates.io][ublox-cell-rs-crate-img]][ublox-cell-rs] [![docs.rs][ublox-cell-rs-docs-img]][ublox-cell-rs-docs] | -->

[ublox-short-range-rs]: https://github.com/BlackbirdHQ/ublox-short-range-rs
<!-- [ublox-short-range-rs-crate-img]: https://img.shields.io/crates/v/ublox-short-range-rs.svg
[ublox-short-range-rs-docs-img]: https://docs.rs/ublox-short-range-rs/badge.svg
[ublox-short-range-rs-docs]: https://docs.rs/ublox-short-range-rs/ -->

<!-- [ublox-cell-rs]: https://github.com/MathiasKoch/ublox-cell-rs
[ublox-cell-rs-crate-img]: https://img.shields.io/crates/v/ublox-cell-rs.svg
[ublox-cell-rs-docs-img]: https://docs.rs/ublox-cell-rs/badge.svg
[ublox-cell-rs-docs]: https://docs.rs/ublox-cell-rs/ -->

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

<!-- Badges -->
[workflow]: https://github.com/MathiasKoch/at-rs/actions?query=workflow%3ACI
[workflow-badge]: https://img.shields.io/github/workflow/status/MathiasKoch/at-rs/CI/master
[crates-io]: https://crates.io/crates/at-rs
[crates-io-badge]: https://img.shields.io/crates/v/at-rs.svg?maxAge=3600
[crates-io-download]: https://crates.io/crates/at-rs
[crates-io-download-badge]: https://img.shields.io/crates/d/at-rs.svg?maxAge=3600
