/// Errors that can occur when sending and receiving data.
#[derive(Debug)]
pub enum Error<CommE, PinE> {
    /// SPI communication error
    Spi(CommE),
    /// Pin set error
    Pin(PinE),
    /// Communication error with module
    CommunicationError(u8),
    /// Max retries reached
    MaxRT,
}
