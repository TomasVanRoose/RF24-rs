//! Errors that can occur when sending and receiving data.

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TransceiverError<SpiErr, CeErr> {
    /// SPI communication error
    Spi(SpiErr),
    /// Chip enable error
    Ce(CeErr),
    /// Communication error with module
    Comm(u8),
    /// Max retries reached
    MaxRetries,
}
