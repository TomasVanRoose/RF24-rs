# Rust nRF24L01 driver
This crate provides a platform agnostic Rust driver for the nRF24L01 single chip 2.4 GHz
transceiver by Nordic Semiconduct for communicating data wirelessly using the [`embedded-hal`](https://github.com/rust-embedded/embedded-hal) traits.

## Usage

This crate can be used by adding `rf24-rs` to your dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
rf24-rs = "0.1"
```

## Examples

## Feature-flags

- **micro-fmt:** provides a `uDebug` implementation from the [ufmt crate](https://docs.rs/ufmt) for all public structs and enums.

## License

This project is licensed under Apache License, Version 2.0 ([LICENSE](LICENSE) or https://www.apache.org/licenses/LICENSE-2.0).
