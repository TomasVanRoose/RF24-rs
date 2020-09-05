use embedded_hal as hal;
use hal::blocking::spi::{Transfer, Write};
use hal::{digital::v2::OutputPin, spi};

mod register;
use register::Register;

/// SPI mode
pub const MODE: spi::Mode = spi::MODE_0;

/// Error
#[derive(Debug)]
pub enum TransmissionError<E, F> {
    /// TODO add error types
    ///
    /// SPI error
    Spi(E),
    /// Pin error
    Pin(F),
}

pub struct Nrf24l01<SPI, NCS> {
    spi: SPI,
    ncs: NCS,
}

type Result<T, E, F> = core::result::Result<T, TransmissionError<E, F>>;

impl<SPI, NCS, SPIErr, PinErr> Nrf24l01<SPI, NCS>
where
    SPI: Transfer<u8, Error = SPIErr> + Write<u8, Error = SPIErr>,
    NCS: OutputPin<Error = PinErr>,
{
    pub fn new(spi: SPI, ncs: NCS) -> Self {
        Nrf24l01 { spi, ncs }
    }

    fn write_register(&mut self, register: Register, value: u8) -> Result<(), SPIErr, PinErr> {
        let buffer = [Instruction::WR.opcode() | register.addr(), value];
        self.ncs.set_low().map_err(TransmissionError::Pin)?;
        self.spi.write(&buffer).map_err(TransmissionError::Spi)?;
        self.ncs.set_high().map_err(TransmissionError::Pin)?;

        Ok(())
    }

    fn read_register(&mut self, register: Register) -> Result<u8, SPIErr, PinErr> {
        let mut buffer = [Instruction::RR.opcode() | register.addr(), 0];
        self.ncs.set_low().map_err(TransmissionError::Pin)?;
        self.spi
            .transfer(&mut buffer)
            .map_err(TransmissionError::Spi)?;
        self.ncs.set_high().map_err(TransmissionError::Pin)?;
        Ok(buffer[1])
    }
}

#[derive(Clone, Copy)]
enum Instruction {
    /// Read registers
    RR = 0b0000_0000,
    /// Write registers
    /// Last 5 bits are the Memory Map Adress
    WR = 0b0010_0000,
    /// Read RX-payload, used in RX mode.
    RRX = 0b0110_0001,
    /// Write TX-payload, used in TX mode.
    WTX = 0b1010_0000,
    /// Flush TX FIFO, used in TX mode.
    FTX = 0b1110_0001,
    /// Flush RX FIFO, used in RX mode.
    FRX = 0b1110_0010,
    /// No operation. Might be used to read STATUS register.
    NOP = 0b1111_1111,
}

impl Instruction {
    pub(crate) fn opcode(&self) -> u8 {
        *self as u8
    }
}
