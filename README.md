# `AT Parser`

[![Build status][workflow-badge]][workflow]
[![Crates.io Version][crates-io-badge]][crates-io]
[![Crates.io Downloads][crates-io-download-badge]][crates-io-download]

A driver support crate for AT-command based serial modules, using the [embedded-hal] traits.


[embedded-hal]: https://crates.io/crates/embedded-hal


### AT Best practices

This crate attempts to work from these AT best practices:

> - The DTE shall flush the AT channel (i.e. check if there are data waiting to be read) before sending a new AT command
> - The DTE shall detect/process complete lines (see the S3, S4 and V0/V1 settings), so they can be processed with a function that handles responses
> - The DTE shall handle the case of unexpected spaces or line endings
> - The DTE shall handle all the URCs: it can simply ignore them (not suggested) or, better, take a proper action
> - The DTE shall know what answer is expected and shall wait until it is received (i.e. final result code only or informationtext response + final result code)
> - The final result code marks the end of an AT command and can be OK, ERROR or ABORTED: when the final result is an error, be sure to handle it before continuing with the next AT command
> - The information text response format is command specific. The DTE will need explicit handling for each one. It is suggested to consult the u-blox AT Commands Manual [1]
> - It is suggested not to strictly parse information text responses but to checkif they contain interesting keywords and/or parameters
> - The DTE shall know if the issued AT command can be aborted or not
> - Some AT commands could output the final result code after some seconds, in this case check on AT manual for the suggested estimated response time. If the timeout expires then a decision should be taken accordingly: e.g. if the command can be aborted then try to abort it, etc ...
> - It is very useful, for debugging an application, to log all the command lines sent to the DCE and what is received from it
> - Create a state machine for the AT parser (i.e. idle, waiting_response, data_mode)
> - The DTE shall wait some time (the recommended value is at least 20 ms) after the reception of an AT command final response or URC before issuing a new AT commandto give the module the opportunity to transmit the buffered URCs. Otherwise the collision of the URCs with the subsequent AT command is still possible
> - The DTE shall be aware that, when using a serial port without HW flow control, the first character is used to wake up the module from power saving



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
| [ublox-short-range-rs] | Driver crate for U-Blox host-based short range devices (wifi and BT) with AT-command interface | <!--[![crates.io][ublox-short-range-rs-crate-img]][ublox-short-range-rs] [![docs.rs][ublox-short-range-rs-docs-img]][ublox-short-range-rs-docs] --> |
<!-- | [ublox-cell-rs] | Driver crate for U-Blox host-based cellular devices with AT-command interface | [![crates.io][ublox-cell-rs-crate-img]][ublox-cell-rs] [![docs.rs][ublox-cell-rs-docs-img]][ublox-cell-rs-docs] | -->

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
