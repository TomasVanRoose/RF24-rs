//! This crate provides a platform agnostic Rust driver for the nRF24L01+ single chip 2.4 GHz
//! transceiver by Nordic Semiconduct for communicating data wirelessly using the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://github.com/rust-embedded/embedded-hal
//!
//! # Usage
//!
//! This crate can be used by adding `rf24-rs` to your dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! rf24-rs = "0.1"
//! ```
//!
//! # Examples
//!
//! # Feature-flags
//!
//! - **micro-fmt:** provides a `uDebug` implementation from the [ufmt crate](https://docs.rs/ufmt) for all public structs and enums.
#![warn(
    missing_docs,
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts
)]
#![no_std]
extern crate embedded_hal as hal;
use hal::spi;

pub mod config;
mod error;
mod nrf24;
mod register_acces;
pub mod status;

pub use crate::error::TransferError;
pub use crate::nrf24::Nrf24l01;

/// SPI mode. Use this when initializing the SPI instance.
pub const SPI_MODE: spi::Mode = spi::MODE_0;
/// Max size in bytes of a single payload to be sent or recieved.
pub const MAX_PAYLOAD_SIZE: u8 = 32;
