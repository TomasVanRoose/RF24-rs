//! Different structs and values for configuration of the chip.
//!
//! To construct a representation of your config, see [`NrfConfig`].
//!
//! # Default values
//! All these options have a default value:
//!
//! * `addr_width`:             address width of 5 bytes.
//! * `ack_payloads_enabled`:   false: acknowledgement payloads are disabled by default.
//! * `auto_retry`:             enabled, will wait 1586µs on ack, and will retry 15 times.
//! * `channel`:                channel 76.
//! * `crc_encoding_scheme`:    encoding scheme with 2 bytes.
//! * `data_rate`:              1Mbps.
//! * `payload_size`:           static payload size of [`MAX_PAYLOAD_SIZE`] bytes.
//! * `pa_level`:               min amplification level.
//!
use crate::register_acces::Register;
use crate::MAX_PAYLOAD_SIZE;

const MAX_CHANNEL: u8 = 125;

/// Configuration builder struct for NRF chip.
///
/// Always created with the `default()` method and modified through
/// the builder pattern.
///
/// # Example: default
/// ```rust
/// use nrf24::Nrf24l01;
/// use nrf24::config::NrfConfig;
///
/// let config = NrfConfig::default();
///
/// let mut chip = Nrf24l01::new(spi, ce, ncs, delay, config)?;
/// ```
///
/// # Example: custom configuration
/// ```rust
/// use nrf24::Nrf24l01;
/// use nrf24::config::{PALevel, DataRate, NrfConfig, PayloadSize};
///
/// let config = NrfConfig::default()
///     .payload_size(PayloadSize::Dynamic) // set dynamic payload size
///     .channel(7)
///     .addr_width(3),
///     .data_rate(DataRate::R2Mbps)
///     .pa_level(PALevel::Max)
///     .crc_encoding_scheme(None) // disable crc
///     .ack_payloads_enabled(true)
///     .auto_retry((15, 15));
///
/// let mut chip = Nrf24l01::new(spi, ce, ncs, delay, config)?;
/// ```
#[derive(Copy, Debug, Clone)]
pub struct NrfConfig {
    pub(crate) payload_size: PayloadSize,
    pub(crate) channel: u8,
    pub(crate) addr_width: AddressWidth,
    pub(crate) data_rate: DataRate,
    pub(crate) pa_level: PALevel,
    pub(crate) crc_encoding_scheme: Option<EncodingScheme>,
    pub(crate) ack_payloads_enabled: bool,
    pub(crate) auto_retry: AutoRetransmission,
}

impl NrfConfig {
    /// Set Payload Size
    /// A value of 0 means dynamic payloads will be enabled.
    /// Values greater than [`MAX_PAYLOAD_SIZE`] will be floored.
    pub fn payload_size<T: Into<PayloadSize>>(mut self, payload_size: T) -> Self {
        self.payload_size = payload_size.into();
        self
    }
    /// Set RF channel
    /// Must be a number in [0..125], values outside will be clipped
    pub fn channel(mut self, channel: u8) -> Self {
        self.channel = core::cmp::min(channel, MAX_CHANNEL);
        self
    }
    /// Set the Address Width
    /// If using a number, it must be in [3..5], values outside will be clipped
    pub fn addr_width<T: Into<AddressWidth>>(mut self, addr_width: T) -> Self {
        self.addr_width = addr_width.into();
        self
    }
    /// Set the Data Rate
    pub fn data_rate(mut self, data_rate: DataRate) -> Self {
        self.data_rate = data_rate;
        self
    }
    /// Set the Power Amplification Level
    pub fn pa_level(mut self, pa_level: PALevel) -> Self {
        self.pa_level = pa_level;
        self
    }
    /// Set the Cyclic Redundancy Check Encodign Scheme
    /// None will disable the CRC.
    pub fn crc_encoding_scheme(mut self, crc_encoding_scheme: Option<EncodingScheme>) -> Self {
        self.crc_encoding_scheme = crc_encoding_scheme;
        self
    }
    /// Configure if auto acknowledgements are enabled
    pub fn ack_payloads_enabled(mut self, ack_payloads_enabled: bool) -> Self {
        self.ack_payloads_enabled = ack_payloads_enabled;
        self
    }
    /// Set the automatic retransmission config
    pub fn auto_retry<T: Into<AutoRetransmission>>(mut self, auto_retry: T) -> Self {
        self.auto_retry = auto_retry.into();
        self
    }
}

impl Default for NrfConfig {
    fn default() -> Self {
        Self {
            channel: 76,
            payload_size: PayloadSize::default(),
            addr_width: AddressWidth::default(),
            crc_encoding_scheme: Some(EncodingScheme::R2Bytes),
            pa_level: PALevel::default(),
            data_rate: DataRate::default(),
            ack_payloads_enabled: false,
            auto_retry: AutoRetransmission::default(),
        }
    }
}

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for NrfConfig {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        f.debug_struct("nRF configuration")?
            .field("channel", &self.channel)?
            .field("payload size", &self.payload_size)?
            .field("power amplification level", &self.pa_level)?
            .field("data rate", &self.data_rate)?
            .field("auto retransmission", &self.auto_retry)?
            .field(
                "acknowledgement payloads enabled",
                &self.ack_payloads_enabled,
            )?
            .field("address width", &self.addr_width)?
            .field("crc encoding scheme", &self.crc_encoding_scheme)?
            .finish()
    }
}
*/

/// Different RF power levels. The higher the level the bigger range, but the more the current
/// consumption.
///
/// Defaults to Min.
#[derive(PartialEq, Eq, Copy, Clone)]
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
impl core::fmt::Debug for PALevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            PALevel::Min => f.write_str("min (-18 dBm)"),
            PALevel::Low => f.write_str("low (-12 dBm)"),
            PALevel::High => f.write_str("high (-6 dBm)"),
            PALevel::Max => f.write_str("max (0 dBm)"),
        }
    }
}

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for PALevel {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            PALevel::Min => f.write_str("min (-18 dBm)"),
            PALevel::Low => f.write_str("low (-12 dBm)"),
            PALevel::High => f.write_str("high (-6 dBm)"),
            PALevel::Max => f.write_str("max (0 dBm)"),
        }
    }
}
*/

/// Enum representing the payload size.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PayloadSize {
    /// The chip will dynamically set the payload size, depending on the message size.
    Dynamic,
    /// Static payload size. Maximum value of 127.
    Static(u8),
}

impl PayloadSize {
    /// Truncates the payload size to be max [`MAX_PAYLOAD_SIZE`].
    pub(crate) fn truncate(self) -> Self {
        match self {
            Self::Dynamic => Self::Dynamic,
            Self::Static(n) => Self::Static(core::cmp::min(n, MAX_PAYLOAD_SIZE)),
        }
    }
}

impl Default for PayloadSize {
    fn default() -> Self {
        Self::Static(MAX_PAYLOAD_SIZE)
    }
}

impl From<u8> for PayloadSize {
    fn from(size: u8) -> Self {
        match size {
            0 => Self::Dynamic,
            n => Self::Static(core::cmp::min(n, MAX_PAYLOAD_SIZE)),
        }
    }
}

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for PayloadSize {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            Self::Dynamic => f.write_str("dynamic payloads"),
            Self::Static(n) => uwrite!(f, "{:?} byte static payloads", n),
        }
    }
}
*/

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
        0b0000_1000
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
            0b0000_0000 => Self::R1Mbps,
            0b0000_1000 => Self::R2Mbps,
            _ => unreachable!(),
        }
    }
}

/*
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
*/

/// Cyclic Redundancy Check encoding scheme.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum EncodingScheme {
    /// 1 byte
    R1Byte = 0,
    /// 2 bytes
    R2Bytes = 1,
}

impl EncodingScheme {
    fn bitmask() -> u8 {
        0b0000_0100
    }

    pub(crate) fn scheme(&self) -> u8 {
        *self as u8
    }
}

impl From<u8> for EncodingScheme {
    fn from(t: u8) -> Self {
        match t & Self::bitmask() {
            0b0000_0000 => Self::R1Byte,
            0b0000_0100 => Self::R2Bytes,
            _ => unreachable!(),
        }
    }
}

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for EncodingScheme {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            Self::R1Byte => f.write_str("1 byte"),
            Self::R2Bytes => f.write_str("2 bytes"),
        }
    }
}
*/

/// Address width for the reading and writing pipes.
#[derive(PartialEq, Eq, Copy, Clone)]
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
    pub(crate) fn from_register(t: u8) -> Self {
        match t & 0b11 {
            0b01 => Self::R3Bytes,
            0b10 => Self::R4Bytes,
            0b11 => Self::R5Bytes,
            _ => unreachable!(),
        }
    }
}
impl Default for AddressWidth {
    fn default() -> Self {
        Self::R5Bytes
    }
}

impl From<u8> for AddressWidth {
    // from literal value
    fn from(t: u8) -> Self {
        match t {
            0..=3 => Self::R3Bytes,
            4 => Self::R4Bytes,
            5..=u8::MAX => Self::R5Bytes,
        }
    }
}

impl core::fmt::Debug for AddressWidth {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::R3Bytes => f.write_str("3 bytes"),
            Self::R4Bytes => f.write_str("4 bytes"),
            Self::R5Bytes => f.write_str("5 bytes"),
        }
    }
}

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for AddressWidth {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            Self::R3Bytes => f.write_str("3 bytes"),
            Self::R4Bytes => f.write_str("4 bytes"),
            Self::R5Bytes => f.write_str("5 bytes"),
        }
    }
}
*/

/// Configuration of automatic retransmission consisting of a retransmit delay
/// and a retransmission count.
///
/// The delay before a retransmit is initiated, is calculated according to the following formula:
/// > ((**delay** + 1) * 250) + 86 µs
///
/// # Default
///
/// * Auto retransmission delay has a default value of 5, which means `1586 µs`.
/// * The chip will try to resend a failed message 15 times by default.
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct AutoRetransmission {
    delay: u8,
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
    pub(crate) fn from_register(reg: u8) -> Self {
        Self {
            delay: reg >> 4,
            count: reg & 0b0000_1111,
        }
    }
    /// The auto retransmit delay value.
    /// Values can be between 0 and 15.
    /// The delay before a retransmit is initiated, is calculated according to the following formula:
    /// > ((**delay** + 1) * 250) + 86 µs
    pub fn raw_delay(&self) -> u8 {
        self.delay
    }

    /// Returns the delay between auto retransmissions in ms.
    pub fn delay(&self) -> u32 {
        ((self.delay as u32 + 1) * 250) + 86
    }
    /// The number of times there will be an auto retransmission.
    /// Guarantueed to be a value between 0 and 15.
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

impl core::fmt::Debug for AutoRetransmission {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AutoRetransmission")
            .field("raw delay value", &self.raw_delay())
            .field("delay (µs)", &self.delay())
            .field("count", &self.count())
            .finish()
    }
}

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for AutoRetransmission {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        f.debug_struct("AutoRetransmission")?
            .field("raw delay value", &self.raw_delay())?
            .field("delay (µs)", &self.delay())?
            .field("count", &self.count())?
            .finish()
    }
}
*/
/// Representation of the different data pipes through which data can be received.
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

/*
#[cfg(feature = "micro-fmt")]
impl uDebug for DataPipe {
    fn fmt<W: ?Sized>(&self, f: &mut Formatter<'_, W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite,
    {
        match *self {
            DataPipe::DP0 => f.write_str("data pipe 0"),
            DataPipe::DP1 => f.write_str("data pipe 1"),
            DataPipe::DP2 => f.write_str("data pipe 2"),
            DataPipe::DP3 => f.write_str("data pipe 3"),
            DataPipe::DP4 => f.write_str("data pipe 4"),
            DataPipe::DP5 => f.write_str("data pipe 5"),
        }
    }
}
*/

#[derive(Debug, Copy, Clone)]
pub(crate) enum Mode {
    TransmissionMode,
    ReceiverMode,
}

#[derive(Copy, Clone)]
pub struct DebugInfo {
    pub(crate) channel: u8,
    pub(crate) data_rate: DataRate,
    pub(crate) pa_level: PALevel,
    pub(crate) crc_encoding_scheme: Option<EncodingScheme>,
    pub(crate) payload_size: PayloadSize,
    pub(crate) retry_setup: AutoRetransmission,
    pub(crate) mode: Mode,
    pub(crate) addr_width: AddressWidth,
    pub(crate) tx_addr: [u8; 5],
    pub(crate) rx1_addr: [u8; 5],
    pub(crate) auto_ack: u8,
    pub(crate) open_read_pipes: u8,
}

impl core::fmt::Debug for DebugInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Nrf24l01")
            .field("channel", &self.channel)
            .field("frequency", &(self.channel as u16 + 2400))
            .field("data_rate", &self.data_rate)
            .field("pa_level", &self.pa_level)
            .field("crc_encoding_scheme", &self.crc_encoding_scheme)
            .field("payload_size", &self.payload_size)
            .field("retry_setup", &self.retry_setup)
            .field("mode", &self.mode)
            .field("address_width", &self.addr_width)
            .field("tx_address", &format_args!("{:x?}", &self.tx_addr))
            .field("rx1_address", &format_args!("{:x?}", &self.rx1_addr))
            .field("auto_ack_channels", &format_args!("{:06b}", self.auto_ack))
            .field(
                "enabled_rx_addresses",
                &format_args!("{:06b}", self.open_read_pipes),
            )
            .finish()
    }
}
