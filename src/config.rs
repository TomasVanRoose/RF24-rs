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
