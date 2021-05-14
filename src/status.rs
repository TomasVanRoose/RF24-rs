//! Status datastructures.
use crate::config::DataPipe;
#[cfg(feature = "micro-fmt")]
use ufmt::{uDebug, uWrite, Formatter};

/// Wrapper for the status value returned from the device.
/// Provides convenience methods and debug implemantions.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Status(u8);

/// Wrapper around the FIFO status.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FIFOStatus(u8);

impl Status {
    /// Create a status obj with all the flags turned on.
    pub fn flags() -> Self {
        Self(0b01110000)
    }
    /// Returns the raw value represented by this struct.
    pub fn value(&self) -> u8 {
        self.0
    }
    /// Checks if the status is valid.
    pub fn is_valid(&self) -> bool {
        (self.0 >> 7) & 1 == 0
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

impl From<u8> for Status {
    fn from(t: u8) -> Self {
        Status(t)
    }
}

impl From<u8> for FIFOStatus {
    fn from(t: u8) -> Self {
        FIFOStatus(t)
    }
}

impl core::fmt::Debug for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if !&self.is_valid() {
            f.write_str("Invalid status. Something went wrong during communication with nrf24l01")
        } else {
            let mut s = f.debug_struct("Status");
            let s = s.field("Data ready", &self.data_ready());
            let s = s.field("Data sent", &self.data_sent());
            let s = s.field("Reached max retries", &self.reached_max_retries());
            let s = match &self.data_pipe_available() {
                None => s.field("No data ready to be read in FIFO", &true),
                Some(pipe) => s.field("Data ready to be read on pipe", &pipe.pipe()),
            };
            let s = s.field("Transmission FIFO full", &self.tx_full());
            s.finish()
        }
    }
}

#[cfg(feature = "micro-fmt")]
impl uDebug for Status {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        if !&self.is_valid() {
            f.write_str("Invalid status. Something went wrong during communication with nrf24l01")
        } else {
            let mut s = f.debug_struct("Status")?;
            let s = s.field("Data ready", &self.data_ready())?;
            let s = s.field("Data sent", &self.data_sent())?;
            let s = s.field("Reached max retries", &self.reached_max_retries())?;
            let s = match &self.data_pipe_available() {
                None => s.field("No data ready to be read in FIFO", &true)?,
                Some(pipe) => s.field("Data ready to be read on pipe", &pipe.pipe())?,
            };
            let s = s.field("Transmission FIFO full", &self.tx_full())?;
            s.finish()
        }
    }
}
