/// Errors that can occur when sending and receiving data.
#[derive(Copy, Clone, Debug)]
pub enum TransferError<SPIError, PinError> {
    /// SPI communication error
    Spi(SPIError),
    /// Pin set error
    Pin(PinError),
    /// Communication error with module
    CommunicationError(u8),
    /// Max retries reached
    MaximumRetries,
}
