//! Different structs and values for configuration of the chip
use crate::register_acces::Register;
use crate::MAX_PAYLOAD_SIZE;
#[cfg(feature = "micro-fmt")]
use ufmt::{uDebug, uWrite, Formatter};

/// Configuration struct for NRF chip
#[derive(Copy, Debug, Clone)]
pub struct NrfConfig {
    pub(crate) payload_size: u8,
    pub(crate) channel: u8,
    pub(crate) addr_width: AddressWidth,
    pub(crate) data_rate: DataRate,
    pub(crate) pa_level: PALevel,
    pub(crate) crc_encoding_scheme: Option<EncodingScheme>,
    pub(crate) dynamic_payloads_enabled: bool,
    pub(crate) ack_payloads_enabled: bool,
    pub(crate) auto_retry: AutoRetransmission,
}

impl NrfConfig {
    /// Set Payload Size
    /// Must be a number in [1..MAX_PAYLOAD_SIZE], values outside will be clipped.
    pub fn payload_size(&mut self, payload_size: u8) -> &Self {
        let payload_size = core::cmp::min(payload_size, MAX_PAYLOAD_SIZE);
        let payload_size = core::cmp::max(payload_size, 1);
        self.payload_size = payload_size;
        self
    }
    /// Set RF channel
    /// Must be a number in [0..125], values outside will be clipped
    pub fn channel(&mut self, channel: u8) -> &Self {
        self.channel = core::cmp::min(channel, 125);
        self
    }
    /// Set the Address Width
    /// If using a number, it must be in [3..5], values outside will be clipped
    pub fn addr_width<T: Into<AddressWidth>>(&mut self, addr_width: T) -> &Self {
        self.addr_width = addr_width.into();
        self
    }
    /// Set the Data Rate
    pub fn data_rate(&mut self, data_rate: DataRate) -> &Self {
        self.data_rate = data_rate;
        self
    }
    /// Set the Power Amplification Level
    pub fn pa_level(&mut self, pa_level: PALevel) -> &Self {
        self.pa_level = pa_level;
        self
    }
    /// Set the Cyclic Redundancy Check Encodign Scheme
    /// None will disable the CRC.
    pub fn crc_encoding_scheme(&mut self, crc_encoding_scheme: Option<EncodingScheme>) -> &Self {
        self.crc_encoding_scheme = crc_encoding_scheme;
        self
    }
    /// Configure if dynamic payloads are enabled
    pub fn dynamic_payloads_enabled(&mut self, dynamic_payloads_enabled: bool) -> &Self {
        self.dynamic_payloads_enabled = dynamic_payloads_enabled;
        self
    }
    /// Configure if auto acknowledgements are enabled
    pub fn ack_payloads_enabled(&mut self, ack_payloads_enabled: bool) -> &Self {
        self.ack_payloads_enabled = ack_payloads_enabled;
        self
    }
    /// Set the automatic retransmission config
    pub fn auto_retry<T: Into<AutoRetransmission>>(&mut self, auto_retry: T) -> &Self {
        self.auto_retry = auto_retry.into();
        self
    }
}

impl Default for NrfConfig {
    fn default() -> Self {
        Self {
            payload_size: MAX_PAYLOAD_SIZE,
            channel: 76,
            addr_width: AddressWidth::default(),
            crc_encoding_scheme: Some(EncodingScheme::R2Bytes),
            pa_level: PALevel::default(),
            data_rate: DataRate::default(),
            dynamic_payloads_enabled: false,
            ack_payloads_enabled: false,
            auto_retry: AutoRetransmission::default(),
        }
    }
}

/// Different RF power levels. The higher the level the bigger range, but the more the current
/// consumption.
///
/// Defaults to Min.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PALevel {
    /// -18 dBm, 7 mA current consumption.
    Min = 0b0000_0000,
    /// -12 dBm, 7.5 mA current consumption.
    Low = 0b0000_0010,
    /// -6 dBm, 9.0 mA current consumption.
    High = 0b0000_0100,
    /// -0 dBm, 11.3 mA current consumption.
    Max = 0b0000_0110,
}

impl PALevel {
    pub(crate) fn bitmask() -> u8 {
        0b0000_0110
    }
    pub(crate) fn level(&self) -> u8 {
        *self as u8
    }
}

impl Default for PALevel {
    fn default() -> Self {
        PALevel::Min
    }
}

impl From<u8> for PALevel {
    fn from(t: u8) -> Self {
        match t & Self::bitmask() {
            0b0000_0000 => Self::Min,
            0b0000_0010 => Self::Low,
            0b0000_0100 => Self::High,
            0b0000_0110 => Self::Max,
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "micro-fmt")]
impl uDebug for PALevel {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            PALevel::Min => f.write_str("Min PA Level (-18 dBm)"),
            PALevel::Low => f.write_str("Low PA level (-12 dBm)"),
            PALevel::High => f.write_str("High PA level (-6 dBm)"),
            PALevel::Max => f.write_str("Max PA level (0 dBm)"),
        }
    }
}

/// Configured speed at which data will be sent.
///
/// Defaults to 2Mpbs.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum DataRate {
    /// 1 Mbps
    R1Mbps = 0b0000_0000,
    /// 2 Mbps
    R2Mbps = 0b0000_0001,
}

impl DataRate {
    pub(crate) fn bitmask() -> u8 {
        0b1111_1110
    }
    pub(crate) fn rate(&self) -> u8 {
        *self as u8
    }
}

impl Default for DataRate {
    fn default() -> Self {
        DataRate::R1Mbps
    }
}

impl From<u8> for DataRate {
    fn from(t: u8) -> Self {
        match t & Self::bitmask() {
            0 => Self::R1Mbps,
            1 => Self::R2Mbps,
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "micro-fmt")]
impl uDebug for DataRate {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            DataRate::R1Mbps => f.write_str("1 Mbps"),
            DataRate::R2Mbps => f.write_str("2 Mbps"),
        }
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

/// Address width
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum AddressWidth {
    /// 3 bytes
    R3Bytes = 1,
    /// 4 bytes
    R4Bytes = 2,
    /// 5 bytes
    R5Bytes = 3,
}

impl AddressWidth {
    pub(crate) fn value(&self) -> u8 {
        *self as u8
    }
}
impl Default for AddressWidth {
    fn default() -> Self {
        Self::R5Bytes
    }
}

impl From<u8> for AddressWidth {
    fn from(t: u8) -> Self {
        match t {
            0..=3 => Self::R3Bytes,
            4 => Self::R4Bytes,
            5..=u8::MAX => Self::R5Bytes,
        }
    }
}

/// Configuration of automatic retransmission.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct AutoRetransmission {
    /// The auto retransmit delay.
    /// Values can be between 0 and 15.
    /// The delay before a retransmit is initiated, is calculated according to the following formula:
    /// > ((**delay** + 1) * 250) + 86 µs
    delay: u8,
    /// The number of times there will be an auto retransmission.
    /// Must be a value between 0 and 15.
    count: u8,
}

impl Default for AutoRetransmission {
    fn default() -> Self {
        Self {
            delay: 5,
            count: 15,
        }
    }
}

impl AutoRetransmission {
    pub fn delay(&self) -> u8 {
        self.delay
    }
    pub fn count(&self) -> u8 {
        self.count
    }
}

impl From<(u8, u8)> for AutoRetransmission {
    fn from((d, c): (u8, u8)) -> Self {
        Self {
            delay: core::cmp::min(d, 15),
            count: core::cmp::min(c, 15),
        }
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

impl Into<Register> for DataPipe {
    fn into(self) -> Register {
        match self {
            DataPipe::DP0 => Register::RX_ADDR_P0,
            DataPipe::DP1 => Register::RX_ADDR_P1,
            DataPipe::DP2 => Register::RX_ADDR_P2,
            DataPipe::DP3 => Register::RX_ADDR_P3,
            DataPipe::DP4 => Register::RX_ADDR_P4,
            DataPipe::DP5 => Register::RX_ADDR_P5,
        }
    }
}

#[cfg(feature = "micro-fmt")]
impl uDebug for DataPipe {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            DataPipe::DP0 => f.write_str("Data pipe 0"),
            DataPipe::DP1 => f.write_str("Data pipe 1"),
            DataPipe::DP2 => f.write_str("Data pipe 2"),
            DataPipe::DP3 => f.write_str("Data pipe 3"),
            DataPipe::DP4 => f.write_str("Data pipe 4"),
            DataPipe::DP5 => f.write_str("Data pipe 5"),
        }
    }
}
