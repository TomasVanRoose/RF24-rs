//! nRF24 implementations

use crate::config::{DataPipe, DataRate, EncodingScheme, PALevel};
use crate::error::Error;
use crate::hal::blocking::{
    delay::DelayMs,
    delay::DelayUs,
    spi::{Transfer, Write},
};
use crate::hal::digital::v2::OutputPin;
use crate::register_acces::{Instruction, Register};
use crate::status::{FIFOStatus, Status};
use core::fmt;

const MAX_PAYLOAD_SIZE: u8 = 32;

/// nRF24L01 Driver
pub struct Nrf24l01<SPI, CE, NCS> {
    spi: SPI,
    ncs: NCS,
    ce: CE,
    config_reg: u8,
    payload_size: u8,
    tx_buf: [u8; MAX_PAYLOAD_SIZE as usize + 1],
}

type Result<T, E, F> = core::result::Result<T, Error<E, F>>;

impl<SPI, CE, NCS, SPIErr, PinErr> Nrf24l01<SPI, CE, NCS>
where
    SPI: Transfer<u8, Error = SPIErr> + Write<u8, Error = SPIErr>,
    NCS: OutputPin<Error = PinErr>,
    CE: OutputPin<Error = PinErr>,
{
    const MAX_ADDR_WIDTH: usize = 5;
    const CORRECT_CONFIG: u8 = 0b00001110;
    const STATUS_RESET: u8 = 0b01110000;

    /// Creates a new nrf24l01 driver.
    pub fn new<D>(
        spi: SPI,
        ce: CE,
        ncs: NCS,
        delay: &mut D,
        payload_size: u8,
    ) -> Result<Self, SPIErr, PinErr>
    where
        D: DelayMs<u8>,
    {
        let mut chip = Nrf24l01 {
            spi,
            ncs,
            ce,
            config_reg: 0,
            payload_size: 0,
            tx_buf: [0; MAX_PAYLOAD_SIZE as usize + 1],
        };

        chip.set_payload_size(payload_size);

        // Set the output pins to the correct levels
        chip.ce.set_low().map_err(Error::Pin)?;
        chip.ncs.set_high().map_err(Error::Pin)?;

        // Must allow the radio time to settle else configuration bits will not necessarily stick.
        // This is actually only required following power up but some settling time also appears to
        // be required after resets too. For full coverage, we'll always assume the worst.
        // Enabling 16b CRC is by far the most obvious case if the wrong timing is used - or skipped.
        // Technically we require 4.5ms + 14us as a worst case. We'll just call it 5ms for good measure.
        delay.delay_ms(5);

        // Set retries
        chip.set_retries(5, 15)?;
        // Set rf
        chip.setup_rf(DataRate::default(), PALevel::default())?;
        // Reset status
        chip.reset_status()?;
        // Set up default configuration.  Callers can always change it later.
        // This channel should be universally safe and not bleed over into adjacent spectrum.
        chip.set_channel(76)?;
        // flush buffers
        chip.flush_rx()?;
        chip.flush_tx()?;

        // clear CONFIG register, Enable PTX, Power Up & 16-bit CRC
        chip.enable_crc(EncodingScheme::R2Bytes)?;

        chip.config_reg = chip.read_register(Register::CONFIG)?;

        chip.power_up(delay)?;

        if chip.config_reg != Self::CORRECT_CONFIG {
            Err(Error::CommunicationError(chip.config_reg))
        } else {
            Ok(chip)
        }
    }

    /// Power up now.
    ///
    /// # Examples
    /// ```rust
    /// chip.power_up(&mut delay)?;
    /// ```
    pub fn power_up<D>(&mut self, delay: &mut D) -> Result<(), SPIErr, PinErr>
    where
        D: DelayMs<u8>,
    {
        // if not powered up, power up and wait for the radio to initialize
        if !self.is_powered_up() {
            // update the stored config register
            self.config_reg |= 1 << 1;
            self.write_register(Register::CONFIG, self.config_reg)?;

            delay.delay_ms(5);
        }
        Ok(())
    }

    /// Check if there are any bytes available to be read.
    pub fn data_available(&mut self) -> Result<bool, SPIErr, PinErr> {
        let fifo_status = self
            .read_register(Register::FIFO_STATUS)
            .map(FIFOStatus::from)?;

        Ok(!fifo_status.rx_empty())
    }

    /// Returns the data pipe where the data is available and `None` if no data available.
    pub fn data_available_on_pipe(&mut self) -> Result<Option<DataPipe>, SPIErr, PinErr> {
        if self.data_available()? {
            self.status().map(|s| Some(s.data_pipe()))
        } else {
            Ok(None)
        }
    }

    /// Opens a reading pipe.
    ///
    /// Call this before calling [start_listening()]().
    pub fn open_reading_pipe(&mut self, mut addr: &[u8]) -> Result<(), SPIErr, PinErr> {
        if addr.len() > Self::MAX_ADDR_WIDTH {
            addr = &addr[0..Self::MAX_ADDR_WIDTH];
        }
        self.write_mult_register(Register::RX_ADDR_P0, addr)?;

        // Enable RX Addr 0
        let old_reg = self.read_register(Register::EN_RXADDR)?;
        self.write_register(Register::EN_RXADDR, old_reg | 1)?;

        // set payload size
        self.write_register(Register::RX_PW_P0, self.payload_size)?;
        Ok(())
    }

    /// Starts listening on the pipes that are opened for reading.
    ///
    /// Make sure [open_reading_pipe()]() is called first.
    ///
    /// TODO: Use the type system to make start and stop listening by RAII and Drop
    pub fn start_listening(&mut self) -> Result<(), SPIErr, PinErr> {
        // Enable RX listening flag
        self.config_reg |= 1;
        self.write_register(Register::CONFIG, self.config_reg)?;
        // Flush interrupts
        self.reset_status()?;

        self.ce.set_high().map_err(Error::Pin)?;

        Ok(())
    }
    /// Stop listening.
    ///
    /// TODO: Use the type system to make start and stop listening by RAII and Drop
    pub fn stop_listening(&mut self) -> Result<(), SPIErr, PinErr> {
        self.ce.set_low().map_err(Error::Pin)?;

        self.config_reg &= !0b1;
        self.write_register(Register::CONFIG, self.config_reg)?;

        Ok(())
    }

    /// Read the available payload
    pub fn read(&mut self, buf: &mut [u8]) -> Result<(), SPIErr, PinErr> {
        let len = core::cmp::min(buf.len(), self.payload_size as usize);

        // Use tx buffer to copy the values into
        // First byte will be the opcode
        self.tx_buf[0] = Instruction::RRX.opcode();
        // Write to spi
        self.ncs.set_low().map_err(Error::Pin)?;
        let r = self
            .spi
            .transfer(&mut self.tx_buf[..=len])
            .map_err(Error::Spi)?;
        self.ncs.set_high().map_err(Error::Pin)?;

        // Transfer the data read to buf
        buf.copy_from_slice(&r[1..=len]);

        Ok(())
    }

    /// Opens a writing pipe.
    ///
    /// Must be called before writing data.
    pub fn open_writing_pipe(&mut self, mut addr: &[u8]) -> Result<(), SPIErr, PinErr> {
        if addr.len() > Self::MAX_ADDR_WIDTH {
            addr = &addr[0..Self::MAX_ADDR_WIDTH];
        }
        self.write_mult_register(Register::TX_ADDR, addr)?;
        // We need to open Reading Pipe 0 with the same address name
        // because ACK messages will be recieved on this channel
        self.write_mult_register(Register::RX_ADDR_P0, addr)?;

        // set payload size
        self.write_register(Register::RX_PW_P0, self.payload_size)?;
        Ok(())
    }

    /// Writes data on the opened channel
    pub fn write<D: DelayUs<u8>>(
        &mut self,
        delay: &mut D,
        buf: &[u8],
    ) -> Result<(), SPIErr, PinErr> {
        // Can transmit a max of `payload_size` bytes
        let len = core::cmp::min(buf.len(), self.payload_size as usize);

        // Copy data over to tx fifo
        let _status = self.send_command_bytes(Instruction::WTX, &buf[..=len])?;

        // Start transmission:
        // pulse CE pin to signal transmission start
        self.ce.set_high().map_err(Error::Pin)?;
        delay.delay_us(10);
        self.ce.set_low().map_err(Error::Pin)?;

        Ok(())
    }

    /// Setup of automatic retransmission.
    ///
    /// # Arguments
    /// * `delay` is the auto retransmit delay.
    /// Values can be between 0 and 15.
    /// The delay before a retransmit is initiated, is calculated according to the following formula:
    /// > ((**delay** + 1) * 250) + 86 µs
    ///
    /// * `count` is number of times there will be an auto retransmission.
    /// Must be a value between 0 and 15.
    ///
    /// # Examples
    /// ```rust
    /// // Set the auto transmit delay to (5 + 1) * 250) + 86 = 1586µs
    /// // and the retransmit count to 15.
    /// nrf24l01.set_retries(5, 15)?;
    /// ```
    pub fn set_retries(&mut self, delay: u8, count: u8) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::SETUP_RETR, (delay << 4) | (count))
    }

    /// Set the frequency channel nRF24L01 operates on.
    ///
    /// # Arguments
    ///
    /// * `channel` number between 0 and 127.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.set_channel(73)?;
    /// ```
    pub fn set_channel(&mut self, channel: u8) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::RF_CH, (0xf >> 1) & channel)
    }

    /// Flush transmission FIFO, used in TX mode.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.flush_tx()?;
    /// ```
    pub fn flush_tx(&mut self) -> Result<(), SPIErr, PinErr> {
        self.send_command(Instruction::FTX).map(|_| ())
    }

    /// Flush reciever FIFO, used in RX mode.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.flush_rx()?;
    /// ```
    pub fn flush_rx(&mut self) -> Result<(), SPIErr, PinErr> {
        self.send_command(Instruction::FRX).map(|_| ())
    }

    /// Enable CRC encoding scheme.
    ///
    /// **Note** that this configures the nrf24l01 in transmit mode.
    ///
    /// # Examples
    /// ```rust
    /// chip.enable_crc(EncodingScheme::R2Bytes)?;
    /// ```
    pub fn enable_crc(&mut self, scheme: EncodingScheme) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::CONFIG, (1 << 3) | (scheme.scheme() << 2))
    }

    /// Configure the data rate and PA level.
    pub fn configure(&mut self, data_rate: DataRate, level: PALevel) -> Result<(), SPIErr, PinErr> {
        self.setup_rf(data_rate, level)
    }

    /// Set the payload size.
    ///
    /// Values bigger than [MAX_PAYLOAD_SIZE](MAX_PAYLOAD_SIZE) will be set to the maximum
    pub fn set_payload_size(&mut self, payload_size: u8) {
        self.payload_size = core::cmp::min(MAX_PAYLOAD_SIZE, payload_size);
    }

    /// Reads the status register from device.
    pub fn status(&mut self) -> Result<Status, SPIErr, PinErr> {
        self.send_command(Instruction::NOP)
    }

    /// Resets the following flags in the status register:
    /// - data ready RX fifo interrupt
    /// - data sent TX fifo interrupt
    /// - maximum number of number of retries interrupt
    pub fn reset_status(&mut self) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::STATUS, Self::STATUS_RESET)
    }

    /// Sends an instruction over the SPI bus without extra data.
    ///
    /// Returns the status recieved from the device.
    /// Normally used for the other instructions then read and write.  
    fn send_command(&mut self, instruction: Instruction) -> Result<Status, SPIErr, PinErr> {
        self.send_command_bytes(instruction, &[])
    }

    // Sends an instruction with some payload data over the SPI bus
    //
    // Returns the status from the device
    fn send_command_bytes(
        &mut self,
        instruction: Instruction,
        buf: &[u8],
    ) -> Result<Status, SPIErr, PinErr> {
        // Use tx buffer to copy the values into
        // First byte will be the opcode
        self.tx_buf[0] = instruction.opcode();
        self.tx_buf[1..=buf.len()].copy_from_slice(buf);
        // Write to spi
        self.ncs.set_low().map_err(Error::Pin)?;
        let r = self
            .spi
            .transfer(&mut self.tx_buf[..=buf.len()])
            .map_err(Error::Spi)?;
        self.ncs.set_high().map_err(Error::Pin)?;

        Ok(Status::from(r[0]))
    }

    /// Writes a value to a given register
    fn write_register(&mut self, register: Register, value: u8) -> Result<(), SPIErr, PinErr> {
        self.write_mult_register(register, &[value])
    }

    /// Writes a byte array to a given register
    fn write_mult_register(
        &mut self,
        register: Register,
        buf: &[u8],
    ) -> Result<(), SPIErr, PinErr> {
        // Use tx buffer to copy the values into
        // First byte will be the opcode
        self.tx_buf[0] = Instruction::WR.opcode() | register.addr();
        // Copy over the values
        self.tx_buf[1..=buf.len()].copy_from_slice(buf);
        // Write to spi
        self.ncs.set_low().map_err(Error::Pin)?;
        self.spi
            .write(&self.tx_buf[..=buf.len()])
            .map_err(Error::Spi)?;
        self.ncs.set_high().map_err(Error::Pin)?;

        Ok(())
    }

    fn read_register(&mut self, register: Register) -> Result<u8, SPIErr, PinErr> {
        let mut buffer = [Instruction::RR.opcode() | register.addr(), 0];
        self.ncs.set_low().map_err(Error::Pin)?;
        self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
        self.ncs.set_high().map_err(Error::Pin)?;
        Ok(buffer[1])
    }

    fn setup_rf(&mut self, data_rate: DataRate, level: PALevel) -> Result<(), SPIErr, PinErr> {
        self.write_register(Register::RF_SETUP, data_rate.rate() | level.level())
    }

    fn is_powered_up(&self) -> bool {
        self.config_reg & (1 << 1) != 0
    }
}

impl<SPI, CE, NCS> fmt::Debug for Nrf24l01<SPI, CE, NCS>
where
    SPI: fmt::Debug,
    CE: fmt::Debug,
    NCS: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Nrf24l01")
            .field("spi", &self.spi)
            .field("ncs", &self.ncs)
            .field("ce", &self.ce)
            .field("config_reg", &self.config_reg)
            .field("payload_size", &self.payload_size)
            .field("tx_buf", &&self.tx_buf[1..])
            .finish()
    }
}
