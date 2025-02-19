//! Errors that can occur when sending and receiving data.
use core::error::Error;
use core::fmt;

#[derive(Copy, Clone, Debug)]
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

impl<SpiErr: fmt::Display, CeErr: fmt::Display> fmt::Display for TransceiverError<SpiErr, CeErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransceiverError::Spi(e) => write!(f, "SPI error: {}", e),
            TransceiverError::Ce(e) => write!(f, "Chip enable error: {}", e),
            TransceiverError::Comm(_) => write!(f, "Communication error"),
            TransceiverError::MaxRetries => write!(f, "Max retries error"),
        }
    }
}

impl<SpiErr: fmt::Debug + fmt::Display, CeErr: fmt::Debug + fmt::Display> Error
    for TransceiverError<SpiErr, CeErr>
{
}
