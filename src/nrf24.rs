//! nRF24 implementations.

use crate::config::{
    AddressWidth, AutoRetransmission, DataPipe, DataRate, EncodingScheme, NrfConfig, PALevel,
    PayloadSize,
};
use crate::error::TransceiverError;
use crate::register_acces::{Instruction, Register};
use crate::status::{Interrupts, Status};
use crate::MAX_PAYLOAD_SIZE;
use embedded_hal::{
    delay::DelayNs,
    digital::{ErrorType as PinErrorType, OutputPin},
    spi::{ErrorType as SpiErrorType, Operation, SpiDevice},
};

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
/// let nrf24 = Nrf24l01::new(spi, ce, &mut delay, NrfConfig::default()).unwrap();
///
/// ```
pub struct Nrf24l01<SPI, CE> {
    spi: SPI,
    // Chip Enable Pin
    ce: CE,
    // Config Register
    config_reg: u8,
    // Payload size
    payload_size: PayloadSize,
}

// Associated type alias to simplify our result types.
type NrfResult<T, SPI, CE> =
    Result<T, TransceiverError<<SPI as SpiErrorType>::Error, <CE as PinErrorType>::Error>>;

impl<SPI, CE> Nrf24l01<SPI, CE>
where
    SPI: SpiDevice,
    CE: OutputPin,
{
    const MAX_ADDR_WIDTH: usize = 5;
    const STATUS_RESET: u8 = 0b01110000;

    /// Creates a new nRF24L01 driver with the given configuration.
    ///
    /// This function initializes the device, configures it according to the provided settings,
    /// and performs validation to ensure proper communication with the chip. After initialization,
    /// the device is powered up and ready to use.
    ///
    /// # Arguments
    ///
    /// * `spi` - SPI interface for communicating with the nRF24L01 chip, this type should implement
    ///           the `SpiDevice` trait from `embedded_hal`
    /// * `ce` - Chip Enable pin for controlling the chip's operating states
    /// * `delay` - Delay provider for timing requirements during initialization
    /// * `config` - Configuration settings for the chip (see [`NrfConfig`] for options)
    ///
    /// # Errors
    ///
    /// This function may return errors in the following situations:
    /// * SPI communication errors
    /// * Chip enable pin errors
    /// * Communication errors with the module (e.g., incorrect configuration register values)
    ///
    /// # Examples
    ///
    /// ```
    /// use nrf24::{Nrf24l01, SPI_MODE};
    /// use nrf24::config::{NrfConfig, PALevel, DataRate};
    ///
    /// // Initialize hardware interfaces (platform-specific)
    /// let spi = setup_spi(SPI_MODE);
    /// let ce = setup_pin();
    /// let mut delay = setup_delay();
    ///
    /// // Create custom configuration
    /// let config = NrfConfig::default()
    ///     .channel(76)
    ///     .data_rate(DataRate::R2Mbps)
    ///     .pa_level(PALevel::Low);
    ///
    /// // Initialize the nRF24L01 driver
    /// match Nrf24l01::new(spi, ce, &mut delay, config) {
    ///     Ok(nrf) => {
    ///         // Successfully initialized
    ///         // Continue with nrf.open_reading_pipe(), nrf.start_listening(), etc.
    ///     },
    ///     Err(e) => {
    ///         // Handle initialization error
    ///         panic!("Failed to initialize nRF24L01: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// The chip requires some settling time after power-up. This function
    /// includes appropriate delays to ensure reliable initialization.
    pub fn new<D: DelayNs>(
        spi: SPI,
        ce: CE,
        delay: &mut D,
        config: NrfConfig,
    ) -> NrfResult<Self, SPI, CE> {
        let mut chip = Nrf24l01 {
            spi,
            ce,
            config_reg: 0,
            payload_size: PayloadSize::Static(0),
        };

        // Set the output pin to the correct levels
        chip.set_ce_low()?;

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

        // The value the config register should be: power up bit + crc encoding
        let config_val = (1 << 1) | config.crc_encoding_scheme.scheme();

        // clear CONFIG register, Enable PTX, Power Up & set CRC
        chip.write_register(Register::CONFIG, config_val)?;

        // wait for startup
        delay.delay_ms(5);

        chip.config_reg = chip.read_register(Register::CONFIG)?;

        if chip.config_reg != config_val {
            Err(TransceiverError::Comm(chip.config_reg))
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
    pub fn is_connected(&mut self) -> NrfResult<bool, SPI, CE> {
        self.read_register(Register::SETUP_AW)
            .map(|aw| aw == 0b1 || aw == 0b10 || aw == 0b11)
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
    ) -> NrfResult<(), SPI, CE> {
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
    pub fn open_writing_pipe(&mut self, mut addr: &[u8]) -> NrfResult<(), SPI, CE> {
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
    pub fn start_listening(&mut self) -> NrfResult<(), SPI, CE> {
        // Enable RX listening flag
        self.config_reg |= 0b1;
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
    pub fn stop_listening(&mut self) -> NrfResult<(), SPI, CE> {
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
    pub fn data_available(&mut self) -> NrfResult<bool, SPI, CE> {
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
    pub fn data_available_on_pipe(&mut self) -> NrfResult<Option<DataPipe>, SPI, CE> {
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
    /// let chip = Nrf24l01::new(spi, ce, &mut delay, config).unwrap();
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
    ///     delay.delay_us(50);
    /// }
    /// ```
    pub fn read(&mut self, buf: &mut [u8]) -> NrfResult<usize, SPI, CE> {
        let len = match self.payload_size {
            PayloadSize::Static(n) => {
                // Ensure buffer is large enough
                if buf.len() < n as usize {
                    return Err(TransceiverError::BufferTooSmall {
                        required: n,
                        actual: buf.len() as u8,
                    });
                }
                n as usize
            }
            PayloadSize::Dynamic => core::cmp::min(buf.len(), MAX_PAYLOAD_SIZE as usize),
        };

        // Write to spi
        self.spi
            .transaction(&mut [
                Operation::Write(&[Instruction::RRX.opcode()]),
                Operation::Read(&mut buf[..len]),
            ])
            .map_err(TransceiverError::Spi)?;

        Ok(len)
    }

    /// Writes data to the opened channel.
    ///
    /// # Examples
    /// ```rust
    /// // We will be sending float values
    /// // Set the payload size to 4 bytes, the size of an f32
    /// let config = NrfConfig::default().payload_size(PayloadSize::Static(4));
    /// let chip = Nrf24l01::new(spi, ce, &mut delay, config).unwrap();
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
    ///     delay.delay_us(50);
    /// }
    /// ```
    ///
    /// Will clear all interrupt flags after write.
    /// Returns an error when max retries have been reached.
    pub fn write<D: DelayNs>(&mut self, delay: &mut D, buf: &[u8]) -> NrfResult<(), SPI, CE> {
        let send_count = match self.payload_size {
            PayloadSize::Static(n) => {
                // we have to send `n` bytes
                if buf.len() < n as usize {
                    return Err(TransceiverError::BufferTooSmall {
                        required: n,
                        actual: buf.len() as u8,
                    });
                }
                n as usize
            }
            PayloadSize::Dynamic => {
                // In dynamic payload mode, max payload_size is the limit
                core::cmp::min(buf.len(), MAX_PAYLOAD_SIZE as usize)
            }
        };

        let status = self.send_command_bytes(Instruction::WTX, &buf[..send_count])?;

        // Start transmission:
        // pulse CE pin to signal transmission start
        self.set_ce_high()?;
        delay.delay_us(10);
        self.set_ce_low()?;

        // Clear interrupt flags
        self.write_register(Register::STATUS, Interrupts::all().raw())?;

        // Max retries exceeded
        if status.reached_max_retries() {
            self.flush_tx()?;
            return Err(TransceiverError::MaxRetries);
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
    ) -> NrfResult<(), SPI, CE> {
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
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, delay, NrfConfig::default())?;
    ///
    /// let retries_config = chip.retries()?;
    /// // Default values for the chip
    /// assert_eq!(retries_config.delay(), 1586);
    /// assert_eq!(retries_config.count(), 15);
    /// ```
    pub fn retries(&mut self) -> NrfResult<AutoRetransmission, SPI, CE> {
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
    pub fn set_channel(&mut self, channel: u8) -> NrfResult<(), SPI, CE> {
        self.write_register(Register::RF_CH, (u8::MAX >> 1) & channel)
    }

    /// Return the frequency channel nRF24L01 operates on.
    /// Note that the actual frequency will we the channel +2400 MHz.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize the chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, delay, NrfConfig::default())?;
    /// // Default is channel 76
    /// assert_eq!(chip.channel()?, 76);
    /// ```
    pub fn channel(&mut self) -> NrfResult<u8, SPI, CE> {
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
    pub fn set_address_width<T>(&mut self, width: T) -> NrfResult<(), SPI, CE>
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
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, delay, NrfConfig::default())?;
    /// // Default is 2 Mb/s
    /// assert_eq!(chip.data_rate()?, DataRate::R2Mbps);
    /// ```
    pub fn data_rate(&mut self) -> NrfResult<DataRate, SPI, CE> {
        self.read_register(Register::RF_SETUP).map(DataRate::from)
    }

    /// Returns the current power amplifier level as a [`PALevel`] enum.
    ///
    /// # Examples
    /// ```rust
    /// // Initialize the chip
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, delay, NrfConfig::default())?;
    /// // Default is Min PALevel
    /// assert_eq!(chip.power_amp_level()?, PALevel::Min);
    /// ```
    pub fn power_amp_level(&mut self) -> NrfResult<PALevel, SPI, CE> {
        self.read_register(Register::RF_SETUP).map(PALevel::from)
    }

    /// Flush transmission FIFO, used in TX mode.
    ///
    /// # Examples
    /// ```rust
    /// chip.flush_tx()?;
    /// ```
    pub fn flush_tx(&mut self) -> NrfResult<(), SPI, CE> {
        self.send_command(Instruction::FTX).map(|_| ())
    }

    /// Flush reciever FIFO, used in RX mode.
    ///
    /// # Examples
    /// ```rust
    /// nrf24l01.flush_rx()?;
    /// ```
    pub fn flush_rx(&mut self) -> NrfResult<(), SPI, CE> {
        self.send_command(Instruction::FRX).map(|_| ())
    }

    /// Enable CRC encoding scheme.
    ///
    /// # Examples
    /// ```rust
    /// chip.enable_crc(EncodingScheme::R2Bytes)?;
    /// ```
    pub fn enable_crc(&mut self, scheme: EncodingScheme) -> NrfResult<(), SPI, CE> {
        // Set the crc encoding bits to 0 first
        self.config_reg &= !EncodingScheme::bitmask();
        // Now set the right bits
        self.config_reg |= scheme.scheme();
        self.write_register(Register::CONFIG, self.config_reg)
    }

    /// Get the CRC encoding scheme
    ///
    /// # Examples
    /// ```rust
    /// match chip.crc_encoding_scheme()? {
    ///     EncodingScheme::NoRedundancyCheck => println("No crc check"),
    ///     EncodingScheme::R1Byte => println("8 bit check"),
    ///     EncodingScheme::R2Bytes => println("16 bit check"),
    /// };
    /// ```
    pub fn crc_encoding_scheme(&mut self) -> NrfResult<EncodingScheme, SPI, CE> {
        self.read_register(Register::CONFIG).map(From::from)
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
    ) -> NrfResult<(), SPI, CE> {
        let payload_size = payload_size.into().truncate();
        match payload_size {
            PayloadSize::Static(payload_size) => {
                if self.payload_size == PayloadSize::Dynamic {
                    // currently dynamic payload enabled
                    // Disable dynamic payloads
                    let feature = self.read_register(Register::FEATURE)?;
                    self.write_register(Register::FEATURE, feature & !(1 << 2))?;
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
                self.write_register(Register::FEATURE, feature | (1 << 2))?;
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
    /// let mut chip = Nrf24l01::new(spi_struct, ce_pin, delay, NrfConfig::default())?;
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
    pub fn power_up<D: DelayNs>(&mut self, delay: &mut D) -> NrfResult<(), SPI, CE> {
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
    pub fn power_down(&mut self) -> NrfResult<(), SPI, CE> {
        self.set_ce_low()?;
        self.config_reg &= !(1 << 1);
        self.write_register(Register::CONFIG, self.config_reg)?;
        Ok(())
    }

    /// Reads the status register from device. See [`Status`].
    pub fn status(&mut self) -> NrfResult<Status, SPI, CE> {
        self.send_command(Instruction::NOP)
    }

    /// Resets the following flags in the status register:
    /// - data ready RX fifo interrupt
    /// - data sent TX fifo interrupt
    /// - maximum number of retries interrupt
    pub fn reset_status(&mut self) -> NrfResult<(), SPI, CE> {
        self.write_register(Register::STATUS, Self::STATUS_RESET)
    }

    /// Masks the selected interrupt flags.
    ///
    /// By default, the IRQ pin will pull low when one of the following events occur:
    /// - Maximum number of retries is reached
    /// - Transmission data is sent
    /// - Receiver data is avaiable to be read
    ///
    /// This function allows you to disable these interrupts.
    ///
    /// # Note
    /// Masking an interrupt doesn't prevent the event from occurring or prevent the status flag from being set.
    /// It only prevents the external IRQ pin from triggering.
    /// You can still read the status register to see if the event occurred, even if the interrupt is masked.
    ///
    /// # Examples
    /// ```rust
    /// let interrupts = Interrupts::new().max_retries().rx_data_ready();
    /// chip.mask_interrupts(interrupts)?;
    /// ```
    /// Disable all interrupts.
    /// ```rust
    /// let interrupts = Interrupts::all();
    /// chip.mask_interrupt(interrupts);
    /// ```
    pub fn mask_interrupts(&mut self, irq: Interrupts) -> NrfResult<(), SPI, CE> {
        // Clear interrupt flags
        self.config_reg &= !Interrupts::all().raw();
        // Set configured interrupt mask
        self.config_reg |= irq.raw();
        self.write_register(Register::STATUS, self.config_reg)?;
        Ok(())
    }

    /// Query which interrupts were triggered.
    ///
    /// Clears the interrupt request flags, so new ones can come in.
    pub fn interrupt_src(&mut self) -> NrfResult<Interrupts, SPI, CE> {
        let status = self.status()?;
        // Clear flags
        self.write_register(Register::STATUS, Interrupts::all().raw())?;
        Ok(Interrupts::from(status.raw()))
    }

    /// Reads the config from the device and returns it in a `NrfConfig` struct.
    /// Can be used to log the configuration when using `defmt` feature.
    pub fn read_config(&mut self) -> NrfResult<NrfConfig, SPI, CE> {
        let addr_width = AddressWidth::from_register(self.read_register(Register::SETUP_AW)?);
        let config = NrfConfig::default()
            .payload_size(self.payload_size)
            .channel(self.channel()?)
            .addr_width(addr_width)
            .data_rate(self.data_rate()?)
            .pa_level(self.power_amp_level()?)
            .crc_encoding_scheme(self.crc_encoding_scheme()?)
            .auto_retry(self.retries()?);
        Ok(config)
    }

    /// Sends an instruction over the SPI bus without extra data.
    ///
    /// Returns the status recieved from the device.
    /// Normally used for the other instructions than read and write.  
    fn send_command(&mut self, instruction: Instruction) -> NrfResult<Status, SPI, CE> {
        self.send_command_bytes(instruction, &[])
    }

    fn send_command_bytes(
        &mut self,
        instruction: Instruction,
        buf: &[u8],
    ) -> NrfResult<Status, SPI, CE> {
        let mut status_buf = [instruction.opcode()];
        self.spi
            .transaction(&mut [
                Operation::TransferInPlace(&mut status_buf),
                Operation::Write(buf),
            ])
            .map_err(TransceiverError::Spi)?;
        Ok(Status::from(status_buf[0]))
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
    ) -> NrfResult<(), SPI, CE> {
        self.spi
            .transaction(&mut [
                Operation::Write(&[Instruction::WR.opcode() | register.addr()]),
                Operation::Write(buf.into_buf()),
            ])
            .map_err(TransceiverError::Spi)
    }

    fn read_register(&mut self, register: Register) -> NrfResult<u8, SPI, CE> {
        let mut buf = [0_u8];
        self.spi
            .transaction(&mut [
                Operation::Write(&[Instruction::RR.opcode() | register.addr()]),
                Operation::Read(&mut buf),
            ])
            .map_err(TransceiverError::Spi)?;
        Ok(buf[0])
    }

    fn setup_rf(&mut self, data_rate: DataRate, level: PALevel) -> NrfResult<(), SPI, CE> {
        self.write_register(Register::RF_SETUP, data_rate.rate() | level.level())
    }

    fn is_powered_up(&self) -> bool {
        self.config_reg & (1 << 1) != 0
    }
}

/// Helper functions for setting Chip Enable pin.
/// Returns the error enum defined in this crate, so the rest of the code can use the
/// `?` operator.
impl<SPI, CE> Nrf24l01<SPI, CE>
where
    SPI: SpiDevice,
    CE: OutputPin,
{
    fn set_ce_high(&mut self) -> NrfResult<(), SPI, CE> {
        self.ce.set_high().map_err(TransceiverError::Ce)
    }
    fn set_ce_low(&mut self) -> NrfResult<(), SPI, CE> {
        self.ce.set_low().map_err(TransceiverError::Ce)
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
