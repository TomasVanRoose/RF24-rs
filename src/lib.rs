//! # `nrf24_rs`
//!
//! This crate provides a platform agnostic Rust driver using no_std for the nRF24L01+ single chip 2.4 GHz
//! transceiver by Nordic Semiconduct for communicating data wirelessly using the [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://github.com/rust-embedded/embedded-hal
//!
//! ## Usage
//!
//! This crate can be used by adding `nrf24-rs` to your dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! nrf24-rs = "0.2"
//! ```
//!
//! The main driver is created by using the [`Nrf24l01::new`] method, which takes a handle to an
//! SpiDevice, a Chip Enable Output Pin and an [`NrfConfig`][config::NrfConfig] instance.
//!
//! ## Examples
//!
//! ### Sending data
//! This simple example will send a simple "Hello world" message.
//! ```rust
//! use nrf24_rs::config::{NrfConfig, PALevel};
//! use nrf24_rs::{Nrf24l01, SPI_MODE};
//! use embedded_hal::spi::SpiBus;
//! use embedded_hal_bus::spi::ExclusiveDevice;
//!
//! fn main() {
//!     let p = get_peripherals(); // peripherals
//!
//!     let spi = setup_spi(); // configure your SPI to use SPI_MODE
//!     
//!     // If you get an SpiBus, convert it into an SpiDevice using the
//!     // `embedded-hal-bus` crate
//!     let delay = setup_delay();
//!     let cs = setup_cs_pin(); // chip select pin
//!     let spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
//!
//!     let message = b"Hello world!"; // The message we will be sending
//!
//!     // Setup some configuration values using the builder pattern
//!     let config = NrfConfig::default()
//!         .channel(8)
//!         .pa_level(PALevel::Min)
//!         // We will use a payload size the size of our message
//!         .payload_size(message.len());
//!
//!     // Initialize the chip
//!     let mut delay = setup_delay(); // create new delay
//!
//!     let mut radio = Nrf24l01::new(spi, ce, &mut delay, config).unwrap();
//!
//!     if !radio.is_connected().unwrap() {
//!         panic!("Chip is not connected.");
//!     }
//!
//!     // Open a writing pipe on address "Node1".
//!     // The listener will have to open a reading pipe with the same address
//!     // in order to recieve this message.
//!     radio.open_writing_pipe(b"Node1").unwrap();
//!
//!     // Keep trying to send the message
//!     while let Err(e) = radio.write(&mut delay, &message) {
//!         // Something went wrong while writing, try again in 50ms
//!         delay.delay_ms(50);
//!     }
//!
//!     // Message should now successfully have been sent!
//!     loop {}
//! }
//! ```
//!
//!
//! ### Reading data
//! This simple example will read a "Hello world" message.
//! ```rust
//! use nrf24_rs::config::{NrfConfig, PALevel, DataPipe};
//! use nrf24_rs::{Nrf24l01, SPI_MODE};
//! use embedded_hal::spi::SpiBus;
//! use embedded_hal_bus::spi::ExclusiveDevice;
//!
//! fn main() {
//!     let p = get_peripherals(); // peripherals
//!
//!     let spi = setup_spi(); // configure your SPI to use SPI_MODE
//!     
//!     // If you get an SpiBus, convert it into an SpiDevice using the
//!     // `embedded-hal-bus` crate
//!     let delay = setup_delay();
//!     let cs = setup_cs_pin(); // chip select pin
//!     let spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
//!
//!     let message = b"Hello world!"; // The message we will be sending
//!
//!     // Setup some configuration values using the builder pattern
//!     let config = NrfConfig::default()
//!         .channel(8)
//!         .pa_level(PALevel::Min)
//!         // We will use a payload size the size of our message
//!         .payload_size(b"Hello world!".len());
//!
//!     // Initialize the chip
//!     let mut delay = setup_delay; // create new delay
//!
//!     let mut radio = Nrf24l01::new(spi, ce, &mut delay, config).unwrap();
//!
//!     if !radio.is_connected().unwrap() {
//!         panic!("Chip is not connected.");
//!     }
//!
//!     // Open reading pipe 0 with address "Node1".
//!     // The sender will have to open its writing pipe with the same address
//!     // in order to transmit this message successfully.
//!     radio.open_reading_pipe(DataPipe::DP0, b"Node1").unwrap();
//!     // Set the chip in RX mode
//!     radio.start_listening().unwrap();
//!
//!     // Keep checking if there is any data available to read
//!     while !radio.data_available().unwrap() {
//!         // No data availble, wait 50ms, then check again
//!         delay.delay_ms(50);
//!     }
//!     // Now there is some data availble to read
//!
//!     // Initialize empty buffer
//!     let mut buffer = [0; b"Hello world!".len()];
//!     radio.read(&mut buffer).unwrap();
//!
//!     assert_eq!(buffer, b"Hello world!");
//!
//!     loop {}
//! }
//! ```
//!
//! ## Feature-flags
//!
//! - **defmt** provides a `defmt::Format` implementation from the [defmt crate](https://docs.rs/defmt) for all public structs and enums.
#![warn(
    missing_docs,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts
)]
#![no_std]

pub mod config;
pub mod error;
mod nrf24;
mod register_acces;
pub mod status;

pub use crate::nrf24::Nrf24l01;

/// SPI mode. Use this when initializing the SPI instance.
pub const SPI_MODE: embedded_hal::spi::Mode = embedded_hal::spi::MODE_0;
/// Max size in bytes of a single payload to be sent or recieved.
pub const MAX_PAYLOAD_SIZE: u8 = 32;
