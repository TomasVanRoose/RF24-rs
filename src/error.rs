/// Error
#[derive(Debug)]
pub enum Error<CommE, PinE> {
    /// SPI communication error
    Spi(CommE),
    /// Pin set error
    Pin(PinE),
    /// Communication error with module
    CommunicationError(u8),
}
