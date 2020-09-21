use crate::config::DataPipe;
#[cfg(feature = "micro-format")]
use ufmt::{uDebug, uWrite, Formatter};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Status(u8);

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FIFOStatus(u8);

impl Status {
    pub fn is_valid(&self) -> bool {
        (self.0 & (1 << 7)) == 0
    }
    pub fn data_ready(&self) -> bool {
        (self.0 & (1 << 6)) != 0
    }
    pub fn data_sent(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }
    pub fn reached_max_retries(&self) -> bool {
        (self.0 & (1 << 4)) != 0
    }
    pub fn data_pipe(&self) -> DataPipe {
        ((self.0 >> 5) & 0b111).into()
    }
    pub fn tx_full(&self) -> bool {
        (self.0 & 0b1) != 0
    }
}

impl FIFOStatus {
    /// Returns `true` if there are availbe locations in transmission queue
    pub(crate) fn tx_full(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    /// Returns `true` if the transmission queue is empty
    pub(crate) fn tx_empty(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    /// Returns `true` if there are availbe locations in receive queue
    pub(crate) fn rx_full(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    /// Returns `true` if the receive queue is empty
    pub(crate) fn rx_empty(&self) -> bool {
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

#[cfg(feature = "micro-format")]
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
            let s = s.field("Data pipe nr", &self.data_pipe().pipe())?;
            let s = s.field("Transmission FIFO full", &self.tx_full())?;
            s.finish()
        }
    }
}
