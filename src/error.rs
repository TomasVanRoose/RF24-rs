#[cfg(feature = "micro-fmt")]
use ufmt::{uDebug, uWrite, Formatter};

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

#[cfg(feature = "micro-fmt")]
impl<SPIError, PinError> uDebug for TransferError<SPIError, PinError> {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            Self::Spi(_) => f.write_str("SPI error"),
            Self::Pin(_) => f.write_str("Pin error"),
            Self::CommunicationError(_) => f.write_str("Communication error"),
            Self::MaximumRetries => f.write_str("SPI error"),
        }
    }
}
