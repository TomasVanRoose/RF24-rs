//! This is a platform agnostic Rust driver for the nRF24L01 single
//! chip 2.4G Hz transceiver by Nordic Semiconduct for communicating
//! data wirelessly using the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://github.com/rust-embedded/embedded-hal
#![warn(
    missing_docs,
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts
)]
#![no_std]

extern crate embedded_hal as hal;
use crate::hal::spi;

mod error;
mod nrf24;
mod register_acces;

pub mod config;
pub mod status;

pub use crate::error::Error;
pub use crate::nrf24::Nrf24l01;
//pub use crate::register_acces::Register;

/// SPI mode
pub const MODE: spi::Mode = spi::MODE_0;
