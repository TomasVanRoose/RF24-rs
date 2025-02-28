//! Status datastructures.
use crate::config::DataPipe;

/// Wrapper for the status value returned from the device.
/// Provides convenience methods and debug implemantions.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Status(u8);

impl Status {
    /// Returns the raw value represented by this struct.
    pub fn raw(&self) -> u8 {
        self.0
    }
    /// Indicates if there is data ready to be read.
    pub fn data_ready(&self) -> bool {
        (self.0 >> 6) & 1 != 0
    }
    /// Indicates whether data has been sent.
    pub fn data_sent(&self) -> bool {
        (self.0 >> 5) & 1 != 0
    }
    /// Indicates whether the max retries has been reached.
    /// Can only be true if auto acknowledgement is enabled.
    pub fn reached_max_retries(&self) -> bool {
        (self.0 >> 4) & 1 != 0
    }
    /// Returns data pipe number for the payload availbe for reading
    /// or None if RX FIFO is empty.
    pub fn data_pipe_available(&self) -> Option<DataPipe> {
        match (self.0 >> 1) & 0b111 {
            x @ 0..=5 => Some(x.into()),
            6 => panic!(),
            7 => None,
            _ => unreachable!(), // because we AND the value
        }
    }
    /// Indicates whether the transmission queue is full or not.
    pub fn tx_full(&self) -> bool {
        (self.0 & 0b1) != 0
    }
}

/// Represents the different interrupt types available on the nRF24L01.
///
/// Each variant corresponds to a specific interrupt condition, with values matching
/// the bit positions in the STATUS register.
///
/// The nRF24L01 IRQ pin is active LOW and will be asserted when any enabled interrupt occurs.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptKind {
    /// Maximum number of TX retransmits interrupt.
    MaxRetries = 0b0001_0000,
    /// Data Sent TX FIFO interrupt. Asserted when packet transmitted on TX.
    TxDataSent = 0b0010_0000,
    /// Data Ready RX FIFO interrupt. Asserted when new data arrives RX FIFO.
    RxDataReady = 0b0100_0000,
}

/// A bitfield representing multiple nRF24L01 interrupts.
///
/// This structure allows for the manipulation and checking of interrupt flags
/// as they appear in the STATUS register. Multiple interrupts can be
/// combined using the builder pattern methods.
///
/// # Examples
///
/// ```
/// // Create an interrupt set with RX data ready and max retries
/// let interrupts = Interrupts::new().rx_data_ready().max_retries();
///
/// // Check if a specific interrupt is set
/// if interrupts.contains(InterruptKind::RxDataReady) {
///     // Handle RX data ready interrupt
/// }
/// ```
#[derive(Copy, Clone)]
pub struct Interrupts(u8);

impl Interrupts {
    /// Creates a new empty interrupt set with no interrupts enabled.
    pub fn new() -> Self {
        Self(0)
    }
    /// Creates an interrupt set with all possible interrupts enabled.
    pub fn all() -> Self {
        Self::new().max_retries().tx_data_sent().rx_data_ready()
    }
    /// Adds the Maximum Retries interrupt to this set.
    ///
    /// This interrupt is triggered when the maximum number of retransmits
    /// has been reached on a packet.
    pub fn max_retries(mut self) -> Self {
        self.0 |= InterruptKind::MaxRetries as u8;
        self
    }
    /// Adds the TX Data Sent interrupt to this set.
    ///
    /// This interrupt is triggered when a packet has been successfully transmitted.
    pub fn tx_data_sent(mut self) -> Self {
        self.0 |= InterruptKind::TxDataSent as u8;
        self
    }
    /// Adds the RX Data Ready interrupt to this set.
    ///
    /// This interrupt is triggered when new data has arrived in the RX FIFO.
    pub fn rx_data_ready(mut self) -> Self {
        self.0 |= InterruptKind::RxDataReady as u8;
        self
    }
    /// Checks if the given interrupt kind is set in this interrupt set.
    ///
    /// Returns `true` if the interrupt is set, `false` otherwise.
    pub fn contains(&self, irq: InterruptKind) -> bool {
        self.0 & irq as u8 != 0
    }
    /// Returns the raw byte value of this interrupt set.
    ///
    /// This is useful when writing to the STATUS or CONFIG registers.
    pub(crate) fn raw(&self) -> u8 {
        self.0
    }
}

impl From<u8> for Interrupts {
    /// Converts a raw byte value to an Interrupts struct.
    ///
    /// Only bits that correspond to valid interrupts are preserved.
    fn from(t: u8) -> Self {
        Self(t & Self::all().raw())
    }
}

impl From<u8> for Status {
    fn from(t: u8) -> Self {
        Status(t)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Status {
    fn format(&self, fmt: defmt::Formatter) {
        if self.raw() & 0x80 != 0 {
            defmt::write!(
                fmt,
                "Invalid status. Something went wrong during communication with nrf24l01"
            )
        } else {
            defmt::write!(
            fmt,
            "Status {{ data_ready: {}, data_sent: {}, reached_max_retries: {}, data_pipe_available: {:?}, tx_full: {} }}",
            self.data_ready(),
            self.data_sent(),
            self.reached_max_retries(),
            self.data_pipe_available(),
            self.tx_full()
        )
        }
    }
}

/// Wrapper around the FIFO status.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FIFOStatus(u8);

impl FIFOStatus {
    /// Returns `true` if there are availbe locations in transmission queue
    pub fn tx_full(&self) -> bool {
        (self.0 >> 5) & 1 != 0
    }

    /// Returns `true` if the transmission queue is empty
    pub fn tx_empty(&self) -> bool {
        (self.0 >> 4) & 1 != 0
    }

    /// Returns `true` if there are availbe locations in receive queue
    pub fn rx_full(&self) -> bool {
        (self.0 >> 1) & 1 != 0
    }

    /// Returns `true` if the receive queue is empty
    pub fn rx_empty(&self) -> bool {
        self.0 & 1 != 0
    }
}

impl From<u8> for FIFOStatus {
    fn from(t: u8) -> Self {
        FIFOStatus(t)
    }
}
