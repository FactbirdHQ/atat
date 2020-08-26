# ATAT

![Test][test]
[![Code coverage][codecov-badge]][codecov]
[![Crates.io Version][crates-io-badge]][crates-io]
[![Crates.io Downloads][crates-io-download-badge]][crates-io-download]
[![chat][chat-badge]][chat]
![No Std][no-std-badge]


<div>
  <img style="vertical-align:middle; padding-bottom: 20px; padding-right: 40px;"  src="https://w7.pngwing.com/pngs/154/828/png-transparent-star-wars-patent-all-terrain-armored-transport-printmaking-atat-uuml-rk-monochrome-film-mecha.png" alt="ATAT" width="250" />
</div>

`#![no_std]` crate for parsing AT commands

A driver support crate for AT-command based serial modules, using the [embedded-hal] traits.

[embedded-hal]: https://crates.io/crates/embedded-hal

## AT Best practices

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

## [Documentation](https://docs.rs/atat/latest)

## Tests

> The crate is covered by tests. These tests can be run by `cargo test --tests --all-features`, and are run by the CI on every push.


## Examples
The crate has examples for usage with [cortex-m-rt] and [cortex-m-rtic] crates.

The samples can be built using `cargo build --example cortex-m-rt --target thumbv7em-none-eabihf` and `cargo build --example rtic --target thumbv7em-none-eabihf`.

Furthermore I have used the crate to build initial WIP drivers for uBlox cellular modules ([ublox-cellular-rs]) and uBlox short-range modules ([ublox-short-range-rs])

[cortex-m-rt]: https://crates.io/crates/cortex-m-rt
[cortex-m-rtic]: https://crates.io/crates/cortex-m-rtic
[ublox-short-range-rs]: https://github.com/BlackbirdHQ/ublox-short-range-rs
[ublox-cellular-rs]: https://github.com/BlackbirdHQ/ublox-cellular-rs

## Releasing to crates.io

This workspace uses `cargo-release` to do workspace releases to crates.io. It can be installed through cargo with `cargo install cargo-release`. The steps involved in a new release are:
1. Run `cargo release --dry-run -- major|minor|patch`, and verify the output
2. Run `cargo release -- major|minor|patch`, to release

## About

  - Minimum rustc version 1.31
  - Tested and built using nightly toolchain, but should work fine for stable as well

## Supported Crates

The following dependent crates provide platform-agnostic device drivers built on `embedded-hal` which also implement this crate's traits:

| Device Name | Description | Crate + Docs |
|-------------|-------------|--------------|
| [ublox-short-range-rs] | Driver crate for U-Blox host-based short range devices (wifi and BT) with AT-command interface | <!--[![crates.io][ublox-short-range-rs-crate-img]][ublox-short-range-rs] [![docs.rs][ublox-short-range-rs-docs-img]][ublox-short-range-rs-docs] --> |
| [ublox-cellular-rs] | Driver crate for U-Blox host-based cellular devices with AT-command interface | <!--[![crates.io][ublox-cellular-rs-crate-img]][ublox-cellular-rs] [![docs.rs][ublox-cellular-rs-docs-img]][ublox-cellular-rs-docs] --> |
| [espresso] | AT based driver crate for ESP8266 WiFi modules | <!--[![crates.io][espresso-crate-img]][espresso] [![docs.rs][espresso-docs-img]][espresso-docs] --> |

[ublox-short-range-rs]: https://github.com/BlackbirdHQ/ublox-short-range-rs
<!-- [ublox-short-range-rs-crate-img]: https://img.shields.io/crates/v/ublox-short-range-rs.svg
[ublox-short-range-rs-docs-img]: https://docs.rs/ublox-short-range-rs/badge.svg
[ublox-short-range-rs-docs]: https://docs.rs/ublox-short-range-rs/ -->

[ublox-cellular-rs]: https://github.com/BlackbirdHQ/ublox-cellular-rs
<!-- [ublox-cellular-rs-crate-img]: https://img.shields.io/crates/v/ublox-cellular-rs.svg
[ublox-cellular-rs-docs-img]: https://docs.rs/ublox-cellular-rs/badge.svg
[ublox-cellular-rs-docs]: https://docs.rs/ublox-cellular-rs/ -->

[espresso]: https://github.com/dbrgn/espresso
<!-- [espresso-crate-img]: https://img.shields.io/crates/v/espresso.svg
[espresso-docs-img]: https://docs.rs/espresso/badge.svg
[espresso-docs]: https://docs.rs/espresso/ -->

## Features

 - `derive`: Enabled by default. Re-exports `atat_derive` to allow deriving `Atat__` traits.
 - `log-logging`: Disabled by default. Enable log statements on various log levels to aid debugging. Powered by `log`.
 - `defmt-default`: Disabled by default. Enable log statements at INFO, or TRACE, level and up, to aid debugging. Powered by `defmt`.
 - `defmt-trace`: Disabled by default. Enable log statements at TRACE level and up, to aid debugging. Powered by `defmt`.
 - `defmt-debug`: Disabled by default. Enable log statements at DEBUG level and up, to aid debugging. Powered by `defmt`.
 - `defmt-info`: Disabled by default. Enable log statements at INFO level and up, to aid debugging. Powered by `defmt`.
 - `defmt-warn`: Disabled by default. Enable log statements at WARN level and up, to aid debugging. Powered by `defmt`.
 - `defmt-error`: Disabled by default. Enable log statements at ERROR level and up, to aid debugging. Powered by `defmt`.

## Chat / Getting Help

If you have questions on the development of AT-AT or want to write a driver
based on it, feel free to join our matrix room at `#atat:matrix.org`!

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
[test]: https://github.com/BlackbirdHQ/atat/workflows/Test/badge.svg
[crates-io]: https://crates.io/crates/atat
[chat]: https://matrix.to/#/!ocRyOwQJhEWrphujkM:matrix.org?via=chat.berline.rs&via=matrix.org
[chat-badge]: https://img.shields.io/badge/chat-atat%3Amatrix.org-brightgreen
[crates-io-badge]: https://img.shields.io/crates/v/atat.svg?maxAge=3600
[crates-io-download]: https://crates.io/crates/atat
[crates-io-download-badge]: https://img.shields.io/crates/d/atat.svg?maxAge=3600
[no-std-badge]: https://img.shields.io/badge/no__std-yes-blue
[codecov-badge]: https://codecov.io/gh/BlackbirdHQ/atat/branch/master/graph/badge.svg
[codecov]: https://codecov.io/gh/BlackbirdHQ/atat
