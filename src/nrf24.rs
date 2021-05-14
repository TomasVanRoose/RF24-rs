//! nRF24 implementations.

use crate::config::{
    AddressWidth, AutoRetransmission, DataPipe, DataRate, EncodingScheme, NrfConfig, PALevel,
    PayloadSize,
};
use crate::error::TransferError;
use crate::hal::blocking::{
    delay::DelayMs,
    delay::DelayUs,
    spi::{Transfer, Write},
};

use crate::hal::digital::v2::OutputPin;
use crate::register_acces::{Instruction, Register};
use crate::status::Status;
use crate::MAX_PAYLOAD_SIZE;
use core::fmt;

/// The nRF24L01 driver type. This struct encapsulates all functionality.
///
/// For the different configuration options see: [`NrfConfig`].
///
/// # Examples
/// ```
/// use nrf24::Nrf24l01;
/// use nrf24::config::NrfConfig;
///
/// // Initialize the chip with deafault configuration.
/// let nrf24 = Nrf24l01::new(spi, ce, ncs, &mut delay, NrfConfig::default()).unwrap();
///
/// ```
pub struct Nrf24l01<SPI, CE, NCS> {
    spi: SPI,
    // SPI Chip Select Pin, active low
    ncs: NCS,
    // Chip Enable Pin
    ce: CE,
    // Config Register
    config_reg: u8,
    // Payload size
    payload_size: PayloadSize,
    // Transmission buffer
    tx_buf: [u8; MAX_PAYLOAD_SIZE as usize + 1],
}

//type Result<T, E, F> = core::result::Result<T, Error<E, F>>;

impl<SPI, CE, NCS, SPIErr, PinErr> Nrf24l01<SPI, CE, NCS>
where
    SPI: Transfer<u8, Error = SPIErr> + Write<u8, Error = SPIErr>,
    NCS: OutputPin<Error = PinErr>,
    CE: OutputPin<Error = PinErr>,
{
    const MAX_ADDR_WIDTH: usize = 5;
    const CORRECT_CONFIG: u8 = 0b00001110;
    const STATUS_RESET: u8 = 0b01110000;

    /// Creates a new nrf24l01 driver with given config.
    /// Starts up the device after initialization, so calling [`power_up()`](#method.power_up) is not necessary.
    ///
    /// # Examples
    /// ```
    /// // Initialize all pins required
    /// let dp = Peripherals::take()::unwrap();
    /// let mut portd = dp.PORTD.split();
    /// let ce = portd.pd3.into_output(&mut portd.ddr); // Chip Enable
    ///
    /// let mut portb = dp.PORTB.split();
    /// let ncs = portb.pb2.into_output(&mut portb.ddr); // Chip Select (active low)
    /// let mosi = portb.pb3.into_output(&mut portb.ddr); // Master Out Slave In Pin
    /// let miso = portb.pb4.into_pull_up_input(&mut portb.ddr); // Master In Slave Out Pin
    /// let sclk = portb.pb5.into_output(&mut portb.ddr); // Clock
    ///
    /// // Now we initialize SPI settings to create an SPI instance
    /// let settings = spi::Settings {
    ///     data_order: DataOrder::MostSignificantFirst,
    ///     clock: SerialClockRate::OscfOver4,
    ///     // The required SPI mode for communication with the nrf chip is specified in
    ///     // this crate
    ///     mode: nrf24_rs::SPI_MODE,
    /// };
    /// let (spi, ncs) = spi::Spi::new(dp.SPI, sclk, mosi, miso, ncs, settings);
    /// let mut delay = hal::delay::Delay::<clock::MHz16>::new();
    ///
    /// // Construct a new instance of the chip with a default configuration
    /// // This will initialize the module and start it up
    /// let nrf24 = nrf24_rs::Nrf24l01::new(spi, ce, ncs, &mut delay, NrfConfig::default())?;
    ///
    /// ```
    pub fn new<D>(
        spi: SPI,
        ce: CE,
        ncs: NCS,
        delay: &mut D,
        config: NrfConfig,
    ) -> Result<Self, TransferError<SPIErr, PinErr>>
    where
        D: DelayMs<u8>,
    {
        let mut chip = Nrf24l01 {
            spi,
            ncs,
            ce,
            config_reg: 0,
            payload_size: PayloadSize::Static(0),
            tx_buf: [0; MAX_PAYLOAD_SIZE as usize + 1],
        };

        // Set the output pins to the correct levels
        chip.set_ce_low()?;
        chip.set_ncs_high()?;

        // Must allow the radio time to settle else configuration bits will not necessarily stick.
        // This is actually only required following power up but some settling time also appears to
        // be required after resets too. For full coverage, we'll always assume the worst.
        // Enabling 16b CRC is by far the most obvious case if the wrong timing is used - or skipped.
        // Technically we require 4.5ms + 14us as a worst case. We'll just call it 5ms for good measure.
        delay.delay_ms(5);

        // Set retries
        chip.set_retries(config.auto_retry)?;
        // Set rf
        chip.setup_rf(config.data_rate, config.pa_level)?;
        // Set payload size
        chip.set_payload_size(config.payload_size)?;
        // Set address length
        chip.set_address_width(config.addr_width)?;
        // Reset status
        chip.reset_status()?;
        // This channel should be universally safe and not bleed over into adjacent spectrum.
        chip.set_channel(config.channel)?;
        // flush buffers
        chip.flush_rx()?;
        chip.flush_tx()?;

        // clear CONFIG register, Enable PTX, Power Up & 16-bit CRC
        if let Some(encoding_scheme) = config.crc_encoding_scheme {
            chip.enable_crc(encoding_scheme)?;
        }

        chip.config_reg = chip.read_register(Register::CONFIG)?;

        chip.power_up(delay)?;

        if chip.config_reg != Self::CORRECT_CONFIG {
            Err(TransferError::CommunicationError(chip.config_reg))
        } else {
            Ok(chip)
        }
    }

    /// Checks if the chip is connected to the SPI bus.
    /// # Examples
    /// ```rust
    /// if !chip.is_connected()? {
    ///     // Handle disconnection
    /// }
    /// ```
    pub fn is_connected(&mut self) -> Result<bool, TransferError<SPIErr, PinErr>> {
        let setup = self.read_register(Register::SETUP_AW)?;
        if setup >= 1 && setup <= 3 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Opens a reading pipe for reading data on an address.
    ///
    /// # Examples
    /// ```rust
    /// chip.open_reading_pipe(DataPipe::DP0, b"Node1")?;
    /// ```
    ///
    /// `pipe` can either be an instance of the type [`DataPipe`] or an integer.
    /// Note that if an integer is provided, numbers higher than 5 will default to reading pipe 0.
    ///
    /// # Warnings
    /// You have to call this before calling [`start_listening()`](#method.start_listening).
    pub fn open_reading_pipe<T: Into<DataPipe>>(
        &mut self,
        pipe: T,
        mut addr: &[u8],
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        let pipe = pipe.into();
        if addr.len() > Self::MAX_ADDR_WIDTH {
            addr = &addr[0..Self::MAX_ADDR_WIDTH];
        }

        // Get the memory map address corresponding to the data pipe.
        let rx_address_reg: Register = pipe.into();
        match pipe {
            DataPipe::DP0 | DataPipe::DP1 => self.write_register(rx_address_reg, addr)?,
            _ => self.write_register(rx_address_reg, addr[0])?,
        }

        // Enable corresponding RX Addr
        let old_reg = self.read_register(Register::EN_RXADDR)?; // Read old value
        self.write_register(Register::EN_RXADDR, old_reg | (1 << pipe.pipe()))?; // Update

        Ok(())
    }

    /// Opens a writing pipe for writing data to an address.
    /// # Examples
    /// ```rust
    /// // Open writing pipe for address "Node1"
    /// chip.open_writing_pipe(b"Node1")?;
    /// ```
    /// # Warnings
    /// Must be called before writing data.
    pub fn open_writing_pipe(
        &mut self,
        mut addr: &[u8],
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        if addr.len() > Self::MAX_ADDR_WIDTH {
            addr = &addr[0..Self::MAX_ADDR_WIDTH];
        }
        // We need to open Reading Pipe 0 with the same address name
        // because ACK messages will be recieved on this channel
        self.write_register(Register::RX_ADDR_P0, addr)?;
        // Open writing pipe
        self.write_register(Register::TX_ADDR, addr)?;

        Ok(())
    }

    /// Starts listening on the pipes that are opened for reading.
    /// Used in Receiver Mode.
    ///
    /// # Examples
    /// ```rust
    /// // First open data pipe 0 with address "Node1"
    /// chip.open_reading_pipe(DataPipe::DP0, b"Node1")?;
    /// // Configure the chip to listening modes (non blocking)
    /// chip.start_listening()
    /// // Now we can check for available messages and read them
    /// ```
    /// # Warnings
    /// Make sure at least one pipe is opened for reading using the [`open_reading_pipe()`](#method.open_reading_pipe) method.
    ///
    // TODO: Use the type system to make start and stop listening by RAII and Drop
    pub fn start_listening(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        // Enable RX listening flag
        self.config_reg |= 1;
        self.write_register(Register::CONFIG, self.config_reg)?;
        // Flush interrupts
        self.reset_status()?;

        self.set_ce_high()?;

        Ok(())
    }

    /// Stops listening.
    ///
    /// # Examples
    /// ```rust
    /// // Configure chip and start listening
    /// chip.open_reading_pipe(DataPipe::DP0, b"Node1")?;
    /// chip.start_listening()?;
    /// // ... read data
    /// // Reading is done, now we can stop listening
    /// chip.stop_listening()?;
    /// ```
    ///
    // TODO: Use the type system to make start and stop listening by RAII and Drop
    pub fn stop_listening(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.set_ce_low()?;

        self.config_reg &= !0b1;
        self.write_register(Register::CONFIG, self.config_reg)?;

        Ok(())
    }

    /// Checks if there are any bytes available to be read.
    ///
    /// # Examples
    /// ```rust
    /// // Chip has to be set in listening mode first
    /// chip.open_reading_pipe(DataPipe::DP0, b"Node1")?;
    /// chip.start_listening()?;
    /// // Check if there is any data to read
    /// while chip.data_available()? {
    ///     // ... read the payload
    ///     delay.delay_ms(50); // small delay between calls of data_available
    /// }
    /// ```
    ///
    /// # Notes
    /// If data_available is called in too rapid succession, the chip can glitch out.
    /// If this is the case, just add a small delay between calling successive `data_available`.
    pub fn data_available(&mut self) -> Result<bool, TransferError<SPIErr, PinErr>> {
        Ok(self.data_available_on_pipe()?.is_some())
    }

    /// Returns the data pipe where the data is available and `None` if no data available.
    ///
    /// # Examples
    /// ```rust
    /// // Chip has to be set in listening mode first
    /// chip.open_reading_pipe(DataPipe::DP0, b"Node1")?;
    /// chip.start_listening()?;
    /// // Check if there is any data to read on pipe 1
    /// while let Some(pipe) = chip.data_available_on_pipe()? {
    ///     if pipe == DataPipe::DP1 {
    ///         // ... read the payload
    ///         delay.delay_ms(50); // small delay between calls of data_available
    ///     }
    /// }
    /// ```
    pub fn data_available_on_pipe(
        &mut self,
    ) -> Result<Option<DataPipe>, TransferError<SPIErr, PinErr>> {
        Ok(self.status()?.data_pipe_available())
    }

    /// Reads the available payload. To check if there are any payloads available, call
    /// [`data_available()`](#method.data_available).
    ///
    /// Make sure the chip is configured in listening mode and at
    /// least one data pipe is opened for reading, see:
    /// * [`open_reading_pipe()`](#method.open_reading_pipe)
    /// * [`start_listening()`](#method.start_listening)
    ///
    /// Returns the number of bytes read into the buffer.
    ///
    /// # Examples
    /// ```rust
    /// // We will be receiving float values
    /// // Set the payload size to 4 bytes, the size of an f32
    /// let config = NrfConfig::default().payload_size(PayloadSize::Static(4));
    /// let chip = Nrf24l01::new(spi, ce, ncs, &mut delay, config).unwrap();
    /// // Put the chip in listening mode
    /// chip.open_reading_pipe(DataPipe::DP0, b"Node1");
    /// chip.start_listening();
    ///
    /// // The buffer where we will read the data into
    /// let mut buffer = [0u8; 4];
    /// loop {
    ///     // Keep reading data if any is available
    ///     while let Ok(true) = chip.data_available() {
    ///         match chip.read(&mut buffer) {
    ///             Err(e) => eprintln!("Error while reading data from buffer: {:?}", e),
    ///             Ok(n) => {
    ///                 println!("Successfully read {} bytes of data!", n);
    ///                 assert_eq!(n, 4);
    ///                 // reinterpret memory as a float
    ///                 let f = f32::from_le_bytes(buffer);
    ///                 println!("Received value: {}", f);
    ///             },
    ///         }
    ///     }
    ///     // Wait some time before trying again
    ///     delay.delay_us(50u16);
    /// }
    /// ```
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransferError<SPIErr, PinErr>> {
        let len = if let PayloadSize::Static(n) = self.payload_size {
            n as usize
        } else {
            core::cmp::min(buf.len(), MAX_PAYLOAD_SIZE as usize)
        };

        // Use tx buffer to copy the values into
        // First byte will be the opcode
        self.tx_buf[0] = Instruction::RRX.opcode();
        // Write to spi
        self.set_ncs_low()?;
        let r = self.spi_transfer_tx_buf(len)?;
        // Transfer the data read to buf.
        // Skip first byte because it contains the command.
        // Make both slices are the same length, otherwise `copy_from_slice` panics.
        buf[..len].copy_from_slice(&r[1..=len]);
        self.set_ncs_high()?;

        Ok(len)
    }

    /// Writes data to the opened channel.
    ///
    /// # Examples
    /// ```rust
    /// // We will be sending float values
    /// // Set the payload size to 4 bytes, the size of an f32
    /// let config = NrfConfig::default().payload_size(PayloadSize::Static(4));
    /// let chip = Nrf24l01::new(spi, ce, ncs, &mut delay, config).unwrap();
    /// // Put the chip in transmission mode
    /// chip.open_writing_pipe(b"Node1");
    /// chip.stop_listening();
    ///
    /// // The buffer where we will write data into before sending
    /// let mut buffer = [0u8; 4];
    /// loop {
    ///     let f = get_reading(); // data from some sensor
    ///     // reinterpret float to bytes and put into buffer
    ///     buffer.copy_from_slice(&f.to_le_bytes());
    ///
    ///     match chip.write(&mut delay, &buffer) {
    ///         Err(e) => eprintln!("Error while sending data {:?}", e),
    ///         Ok(_) => {
    ///             println!("Successfully wrote the data!");
    ///         },
    ///     }
    ///     // Wait some time before trying again
    ///     delay.delay_us(50u16);
    /// }
    /// ```
    ///
    /// Will clear all interrupt flags after write.
    /// Returns an error when max retries have been reached.
    pub fn write<D: DelayUs<u8>>(
        &mut self,
        delay: &mut D,
        buf: &[u8],
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        let send_count = if let PayloadSize::Static(n) = self.payload_size {
            let n = n as usize;
            // we have to send `n` bytes
            let len = core::cmp::min(buf.len(), n);
            self.tx_buf[1..=len].copy_from_slice(&buf[..len]);
            if len < MAX_PAYLOAD_SIZE as usize {
                self.tx_buf[len + 1..=n].fill(0);
            }
            // now our tx_buf is guarantueed to have `n` bytes filled
            n
        } else {
            // In dynamic payload mode, max payload_size is the limit
            core::cmp::min(buf.len(), MAX_PAYLOAD_SIZE as usize)
        };

        // Add instruction to buffer
        self.tx_buf[0] = Instruction::WTX.opcode();
        // Write to spi
        self.set_ncs_low()?;
        let r = self.spi_transfer_tx_buf(send_count)?;
        let status = Status::from(r[0]);
        self.set_ncs_high()?;

        // Start transmission:
        // pulse CE pin to signal transmission start
        self.set_ce_high()?;
        delay.delay_us(10);
        self.set_ce_low()?;

        // Clear interrupt flags
        self.write_register(Register::STATUS, Status::flags().value())?;

        // Max retries exceeded
        if status.reached_max_retries() {
            self.flush_tx()?;
            return Err(TransferError::MaximumRetries);
        }

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
    /// nrf24l01.set_retries((5, 15))?;
    /// ```
    pub fn set_retries<T: Into<AutoRetransmission>>(
        &mut self,
        auto_retry: T,
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        let auto_retry = auto_retry.into();
        self.write_register(
            Register::SETUP_RETR,
            (auto_retry.raw_delay() << 4) | (auto_retry.count()),
        )
    }

    /// Returns the auto retransmission config.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize the chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, ncs_pin, delay, NrfConfig::default())?;
    ///
    /// let retries_config = chip.retries()?;
    /// // Default values for the chip
    /// assert_eq!(retries_config.delay(), 1586);
    /// assert_eq!(retries_config.count(), 15);
    /// ```
    pub fn retries(&mut self) -> Result<AutoRetransmission, TransferError<SPIErr, PinErr>> {
        self.read_register(Register::SETUP_RETR)
            .map(AutoRetransmission::from_register)
    }

    /// Set the frequency channel nRF24L01 operates on.
    ///
    /// # Arguments
    ///
    /// * `channel` number between 0 and 127.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.set_channel(74)?;
    /// ```
    pub fn set_channel(&mut self, channel: u8) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.write_register(Register::RF_CH, (u8::MAX >> 1) & channel)
    }

    /// Return the frequency channel nRF24L01 operates on.
    /// Note that the actual frequency will we the channel +2400 MHz.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize the chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, ncs_pin, delay, NrfConfig::default())?;
    /// // Default is channel 76
    /// assert_eq!(chip.channel()?, 76);
    /// ```
    pub fn channel(&mut self) -> Result<u8, TransferError<SPIErr, PinErr>> {
        self.read_register(Register::RF_CH)
    }

    /// Set the address width, saturating values above or below allowed range.
    ///
    /// # Arguments
    ///
    /// * `width` number between 3 and 5.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.set_address_width(5)?;
    /// ```
    pub fn set_address_width<T>(&mut self, width: T) -> Result<(), TransferError<SPIErr, PinErr>>
    where
        T: Into<AddressWidth>,
    {
        let width = width.into();
        self.write_register(Register::SETUP_AW, width.value())
    }

    /// Returns the current data rate as a [`DataRate`] enum.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize the chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, ncs_pin, delay, NrfConfig::default())?;
    /// // Default is 2 Mb/s
    /// assert_eq!(chip.data_rate()?, DataRate::R2Mbps);
    /// ```
    pub fn data_rate(&mut self) -> Result<DataRate, TransferError<SPIErr, PinErr>> {
        self.read_register(Register::RF_SETUP).map(DataRate::from)
    }

    /// Returns the current power amplifier level as a [`PALevel`] enum.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize the chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, ncs_pin, delay, NrfConfig::default())?;
    /// // Default is Min PALevel
    /// assert_eq!(chip.power_amp_level()?, PALevel::Min);
    /// ```
    pub fn power_amp_level(&mut self) -> Result<PALevel, TransferError<SPIErr, PinErr>> {
        self.read_register(Register::RF_SETUP).map(PALevel::from)
    }

    /// Flush transmission FIFO, used in TX mode.
    ///
    /// # Examples
    /// ```rust
    /// chip.flush_tx()?;
    /// ```
    pub fn flush_tx(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.send_command(Instruction::FTX).map(|_| ())
    }

    /// Flush reciever FIFO, used in RX mode.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.flush_rx()?;
    /// ```
    pub fn flush_rx(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
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
    pub fn enable_crc(
        &mut self,
        scheme: EncodingScheme,
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.write_register(Register::CONFIG, (1 << 3) | (scheme.scheme() << 2))
    }

    /// Sets the payload size in bytes.
    /// This can either be static with a set size, or dynamic.
    ///
    /// `payload_size` can either be an instance of the [`PayloadSize`] enum, or an integer.
    ///
    /// # Notes
    /// * A value of 0 means the dynamic payloads will be enabled.
    /// * Values bigger than [`MAX_PAYLOAD_SIZE`](constant.MAX_PAYLOAD_SIZE.html) will be set to the maximum.
    ///
    /// # Examples
    /// ```rust
    /// // Two equal methods to set the chip to dynamic payload mode.
    /// chip.set_payload_size(PayloadSize::Dynamic)?;
    /// chip.set_payload_size(0)?;
    /// // Following methods set a static payload size.
    /// chip.set_payload_size(12)?; // Messages will be 12 bytes
    /// chip.set_payload_size(PayloadSize::Static(12))?; // Same as previous
    /// chip.set_payload_size(49)?; // Messages will be `MAX_PAYLOAD_SIZE`
    /// ```
    pub fn set_payload_size<T: Into<PayloadSize>>(
        &mut self,
        payload_size: T,
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        let payload_size = payload_size.into().truncate();
        match payload_size {
            PayloadSize::Static(payload_size) => {
                if self.payload_size == PayloadSize::Dynamic {
                    // currently dynamic payload enabled
                    // Disable dynamic payloads
                    let feature = self.read_register(Register::FEATURE)?;
                    self.write_register(Register::CONFIG, feature & !(1 << 2))?;
                }

                self.write_register(Register::RX_PW_P0, payload_size)?;
                self.write_register(Register::RX_PW_P1, payload_size)?;
                self.write_register(Register::RX_PW_P2, payload_size)?;
                self.write_register(Register::RX_PW_P3, payload_size)?;
                self.write_register(Register::RX_PW_P4, payload_size)?;
                self.write_register(Register::RX_PW_P5, payload_size)?;
            }
            PayloadSize::Dynamic => {
                let feature = self.read_register(Register::FEATURE)?;
                self.write_register(Register::CONFIG, feature | (1 << 2))?;
                self.write_register(Register::DYNPD, 0b0001_1111)?; // enable on all pipes
            }
        }
        self.payload_size = payload_size;
        Ok(())
    }

    /// Returns the payload size as a [`PayloadSize`] enum.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, ncs_pin, delay, NrfConfig::default())?;
    /// // Default payload size is MAX_PAYLOAD_SIZE
    /// assert_eq!(chip.payload_size()?, PayloadSize::Static(MAX_PAYLOAD_SIZE));
    /// ```
    pub fn payload_size(&self) -> PayloadSize {
        self.payload_size
    }

    /// Powers the chip up. Note that a new initialized device will already be in power up mode, so
    /// calling [`power_up()`](#method.power_up) is not necessary.
    ///
    /// Should be called after [`power_down()`](#method.power_down) to put the chip back into power up mode.
    ///
    /// # Examples
    /// ```rust
    /// // Go to sleep
    /// chip.power_down(&mut delay)?;
    /// // Zzz
    /// // ...
    /// chip.power_up(&mut delay)?; // power back up
    /// ```
    pub fn power_up<D>(&mut self, delay: &mut D) -> Result<(), TransferError<SPIErr, PinErr>>
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

    /// Powers the chip down. This is the low power mode.
    /// The chip will consume approximatly 900nA.
    ///
    /// To power the chip back up, call [`power_up()`](#method.power_up).
    ///
    /// # Examples
    /// ```rust
    /// // Go to sleep
    /// chip.power_down(&mut delay)?;
    /// // Zzz
    /// // ...
    /// chip.power_up(&mut delay)?; // power back up
    /// ```
    pub fn power_down(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.set_ce_low()?;
        self.config_reg &= !(1 << 1);
        self.write_register(Register::CONFIG, self.config_reg)?;
        Ok(())
    }

    /// Reads the status register from device. See [`Status`].
    pub fn status(&mut self) -> Result<Status, TransferError<SPIErr, PinErr>> {
        self.send_command(Instruction::NOP)
    }

    /// Resets the following flags in the status register:
    /// - data ready RX fifo interrupt
    /// - data sent TX fifo interrupt
    /// - maximum number of number of retries interrupt
    pub fn reset_status(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.write_register(Register::STATUS, Self::STATUS_RESET)
    }

    /// Sends an instruction over the SPI bus without extra data.
    ///
    /// Returns the status recieved from the device.
    /// Normally used for the other instructions than read and write.  
    fn send_command(
        &mut self,
        instruction: Instruction,
    ) -> Result<Status, TransferError<SPIErr, PinErr>> {
        self.send_command_bytes(instruction, &[])
    }

    // Sends an instruction with some payload data over the SPI bus
    //
    // Returns the status from the device
    fn send_command_bytes(
        &mut self,
        instruction: Instruction,
        buf: &[u8],
    ) -> Result<Status, TransferError<SPIErr, PinErr>> {
        // Use tx buffer to copy the values into
        // First byte will be the opcode
        self.tx_buf[0] = instruction.opcode();
        self.tx_buf[1..=buf.len()].copy_from_slice(buf);
        // Write to spi
        self.set_ncs_low()?;
        let r = self.spi_transfer_tx_buf(buf.len())?;
        let status = Status::from(r[0]);
        self.set_ncs_high()?;

        Ok(status)
    }

    /// Writes values to a given register.
    ///
    /// This can be anything that can be turned into a buffer of u8's.
    /// `IntoBuf` is currently implemented for T and for &[T].
    /// This means that this function can be polymorphically called for single value writes as well
    /// as for arrays.
    fn write_register<T: IntoBuf<u8>>(
        &mut self,
        register: Register,
        buf: T,
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        let buf = buf.into_buf();
        // Use tx buffer to copy the values into
        // First byte will be the opcode
        self.tx_buf[0] = Instruction::WR.opcode() | register.addr();
        // Copy over the values
        self.tx_buf[1..=buf.len()].copy_from_slice(buf);
        // Write to spi
        self.set_ncs_low()?;
        self.spi_write_tx_buf(buf.len())?;
        self.set_ncs_high()?;

        Ok(())
    }

    fn read_register(&mut self, register: Register) -> Result<u8, TransferError<SPIErr, PinErr>> {
        self.tx_buf[..2].copy_from_slice(&[Instruction::RR.opcode() | register.addr(), 0]);
        self.set_ncs_low()?;
        let reg = self.spi_transfer_tx_buf(2)?[1];
        self.set_ncs_high()?;
        Ok(reg)
    }

    fn setup_rf(
        &mut self,
        data_rate: DataRate,
        level: PALevel,
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.write_register(Register::RF_SETUP, data_rate.rate() | level.level())
    }

    fn is_powered_up(&self) -> bool {
        self.config_reg & (1 << 1) != 0
    }
}

/// Helper functions for setting Chip Select pin.
/// Returns the error enum defined in this crate, so the rest of the code can use the
/// `?` operator.
impl<SPI, CE, NCS, PinErr> Nrf24l01<SPI, CE, NCS>
where
    NCS: OutputPin<Error = PinErr>,
{
    fn set_ncs_high<SPIErr>(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.ncs.set_high().map_err(TransferError::Pin)
    }
    fn set_ncs_low<SPIErr>(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.ncs.set_low().map_err(TransferError::Pin)
    }
}

/// Helper functions for setting Chip Enable pin.
/// Returns the error enum defined in this crate, so the rest of the code can use the
/// `?` operator.
impl<SPI, CE, NCS, PinErr> Nrf24l01<SPI, CE, NCS>
where
    CE: OutputPin<Error = PinErr>,
{
    fn set_ce_high<SPIErr>(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.ce.set_high().map_err(TransferError::Pin)
    }
    fn set_ce_low<SPIErr>(&mut self) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.ce.set_low().map_err(TransferError::Pin)
    }
}

/// Helper function for transfering data over the SPI bus.
/// Returns the error enum defined in this crate, so the rest of the code can use the
/// `?` operator.
impl<SPI, CE, NCS, SPIErr> Nrf24l01<SPI, CE, NCS>
where
    SPI: Transfer<u8, Error = SPIErr>,
{
    /// *NOTE*
    /// Make sure the data to be transfered is copied to the TX Buf before calling this function.
    /// Because the first byte always has to be the command, the `len` argument
    /// is the inclusive length.
    fn spi_transfer_tx_buf<PinErr>(
        &mut self,
        len: usize,
    ) -> Result<&[u8], TransferError<SPIErr, PinErr>> {
        self.spi
            .transfer(&mut self.tx_buf[..=len])
            .map_err(TransferError::Spi)
    }
}

/// Helper function for writing data over the SPI bus.
/// Returns the error enum defined in this crate, so the rest of the code can use the
/// `?` operator.
impl<SPI, CE, NCS, SPIErr> Nrf24l01<SPI, CE, NCS>
where
    SPI: Write<u8, Error = SPIErr>,
{
    /// *NOTE*
    /// Make sure the data to be written is copied to the TX Buf before calling this function.
    /// Because the first byte always has to be the command, the `len` argument
    /// is the inclusive length.
    fn spi_write_tx_buf<PinErr>(
        &mut self,
        len: usize,
    ) -> Result<(), TransferError<SPIErr, PinErr>> {
        self.spi
            .write(&mut self.tx_buf[..=len])
            .map_err(TransferError::Spi)
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
            //.field("payload_size", &self.payload_size)
            .field("tx_buf", &&self.tx_buf[1..])
            .finish()
    }
}

#[cfg(feature = "micro-fmt")]
impl<SPI, CE, NCS, SPIErr, PinErr> Nrf24l01<SPI, CE, NCS>
where
    PinErr: core::fmt::Debug,
    SPIErr: core::fmt::Debug,
    SPI: Transfer<u8, Error = SPIErr> + Write<u8, Error = SPIErr>,
    NCS: OutputPin<Error = PinErr>,
    CE: OutputPin<Error = PinErr>,
{
    /// Write debug information to formatter.
    pub fn debug_write<W: ?Sized>(
        &mut self,
        f: &mut ufmt::Formatter<'_, W>,
    ) -> core::result::Result<(), W::Error>
    where
        W: ufmt::uWrite,
    {
        f.debug_struct("NRF Configuration")?
            .field("channel", &self.channel().unwrap())?
            .field("frequency", &(self.channel().unwrap() as u16 + 2400))?
            .field("data rate", &self.data_rate().unwrap())?
            .field(
                "power amplification level",
                &self.power_amp_level().unwrap(),
            )?
            //.field("crc encoding scheme", &self.enco().unwrap())?
            //.field("address length", &self.set_address_width)
            .field("payload size", &self.payload_size())?
            .field("auto retransmission", &self.retries().unwrap())?
            .finish()
    }
}

/// A trait representing a type that can be turned into a buffer.
///
/// Is used for representing single values as well as slices as buffers.
trait IntoBuf<T> {
    fn into_buf(&self) -> &[T];
}

impl<T> IntoBuf<T> for T {
    fn into_buf(&self) -> &[T] {
        core::slice::from_ref(self)
    }
}
impl<T> IntoBuf<T> for &[T] {
    fn into_buf(&self) -> &[T] {
        self
    }
}
