use embedded_hal as hal;
use hal::blocking::spi::{Transfer, Write};
use hal::{digital::v2::OutputPin, spi};

/// SPI mode
pub const MODE: spi::Mode = spi::MODE_0;

/// Error
#[derive(Debug)]
pub enum Error<E> {
    /// TODO add error types
    ///
    /// SPI error
    Spi(E),
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Spi(e)
    }
}

pub struct Nrf24l01<SPI, CS> {
    spi: SPI,
    cs: CS,
}

impl<SPI, CS, E> Nrf24l01<SPI, CS>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E>,
    CS: OutputPin,
{
    pub fn new(spi: SPI, cs: CS) -> Self {
        Nrf24l01 { spi, cs }
    }
}
