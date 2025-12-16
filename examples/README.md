# Examples

There are two sets of examples: an example for `embassy` / `no-std` and examples for `tokio` / `std`.
The `embassy` example depends on the feature `embedded`, the `tokio` examples depend on the `std` feature
in the [Cargo.toml](./Cargo.toml).

## `std`

If you want to adapt the `tokio` examples for you own `std` application,
make sure to add the following dependency to your `Cargo.toml`:
```toml
embassy-time = {version = "<current_required_version>", features = ["std", "generic-queue-8"]}
```
This ensures, that `embassy-time` (a dependency of `atat`, even in a `tokio` context) does not depend on the embassy executor.
For more derails, refer to the [`embassy-time` documentation](https://docs.rs/embassy-time/latest/embassy_time/)
