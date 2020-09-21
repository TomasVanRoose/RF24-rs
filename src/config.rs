pub struct Config {
    reg: u8,
    payload_size: u8,
    addr_width: u8,
}

/// Different RF output power adjustment levels
///
/// Defaults to Min
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PALevel {
    /// -18 dBm, 7 mA current consumption.
    Min = 0b0000_0000,
    /// -12 dBm, 7.5 mA current consumption.
    Low = 0b0000_0010,
    /// -6 dBm, 9.0 mA current consumption.
    High = 0b0000_0100,
    /// -0 dBm, 11.3 mA current consumption.
    Max = -0b0000_0110,
}

impl PALevel {
    pub(crate) fn level(&self) -> u8 {
        *self as u8
    }
}

impl Default for PALevel {
    fn default() -> Self {
        PALevel::Min
    }
}

/// Data rate at which to send
///
/// Defaults to 2Mpbs
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum DataRate {
    /// 1 Mbps
    R1Mbps = 0b0000_0000,
    /// 2 Mbps
    R2Mbps = 0b0000_0001,
}

impl DataRate {
    pub(crate) fn rate(&self) -> u8 {
        *self as u8
    }
}

impl Default for DataRate {
    fn default() -> Self {
        DataRate::R2Mbps
    }
}

/// CRC encoding scheme
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum EncodingScheme {
    /// 1 byte
    R1Byte = 0,
    /// 2 bytes
    R2Bytes = 1,
}

impl EncodingScheme {
    pub(crate) fn scheme(&self) -> u8 {
        *self as u8
    }
}

/// Representation of the different data pipes through which data can be received
///
/// An nRF24L01 configured as primary RX (PRX) will be able to receive data trough 6 different data
/// pipes.
/// One data pipe will have a unique address but share the same frequency channel.
/// This means that up to 6 different nRF24L01 configured as primary TX (PTX) can communicate with
/// one nRF24L01 configured as PRX, and the nRF24L01 configured as PRX will be able to distinguish
/// between them.
///
/// The default assumed data pipe is 0.
///
/// Data pipe 0 has a unique 40 bit configurable address. Each of data pipe 1-5 has an 8 bit unique
/// address and shares the 32 most significant address bits.
///
/// # Notes
/// In the PTX device data pipe 0 is used to received the acknowledgement, and therefore the
/// receive address for data pipe 0 has to be equal to the transmit address to be able to receive
/// the acknowledgement.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum DataPipe {
    /// Data pipe 0.
    /// Default pipe with a 40 bit configurable address.
    /// This pipe is used in TX mode when auto acknowledgement is enabled. On this channel the ACK
    /// messages are received.
    DP0 = 0,
    /// Data pipe 1.
    DP1 = 1,
    /// Data pipe 2.
    DP2 = 2,
    /// Data pipe 3.
    DP3 = 3,
    /// Data pipe 4.
    DP4 = 4,
    /// Data pipe 5.
    DP5 = 5,
}

impl DataPipe {
    pub(crate) fn pipe(&self) -> u8 {
        *self as u8
    }
}

impl Default for DataPipe {
    fn default() -> Self {
        DataPipe::DP0
    }
}

impl From<u8> for DataPipe {
    fn from(t: u8) -> Self {
        match t {
            0 => DataPipe::DP0,
            1 => DataPipe::DP1,
            2 => DataPipe::DP2,
            3 => DataPipe::DP3,
            4 => DataPipe::DP4,
            5 => DataPipe::DP5,
            _ => DataPipe::DP0,
        }
    }
}
