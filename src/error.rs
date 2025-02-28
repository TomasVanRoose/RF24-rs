//! Errors that can occur when sending and receiving data with the nRF24L01 transceiver.
//!
//! This module provides a comprehensive error type that encapsulates all possible
//! failure modes when interacting with the nRF24L01 module, including SPI communication
//! errors, GPIO control errors, and transceiver-specific error conditions.

/// Represents all possible errors that can occur when using the nRF24L01 transceiver.
///
/// This error type is generic over the underlying SPI and CE pin error types.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum TransceiverError<SpiErr, CeErr> {
    /// An error occurred during SPI communication.
    ///
    /// This typically represents a failure in the hardware communication layer,
    /// such as bus contention or device disconnection.
    Spi(SpiErr),

    /// An error occurred when controlling the Chip Enable (CE) pin.
    ///
    /// This could happen if the GPIO pin controlling the CE line encounters
    /// a hardware failure or configuration issue.
    Ce(CeErr),

    /// A communication error occurred with the nRF24L01 module.
    ///
    /// The wrapped value is a status byte from the device that indicates
    /// the specific nature of the communication failure.
    Comm(u8),

    /// The maximum number of retransmit attempts was reached without receiving an ACK.
    ///
    /// This occurs when auto-acknowledgement is enabled and the receiver does not
    /// acknowledge receipt of the transmitted packet after the configured number of retries.
    MaxRetries,

    /// The provided buffer is too small for the requested operation.
    ///
    /// This error includes information about the required buffer size and the
    /// actual size that was provided.
    BufferTooSmall {
        /// The required buffer size in bytes
        required: u8,
        /// The actual buffer size that was provided
        actual: u8,
    },

    /// An error occurred while waiting for an interrupt in async mode.
    ///
    /// This error is only available when the "async" feature is enabled.
    #[cfg(feature = "async")]
    InterruptWaitFailed,
}
