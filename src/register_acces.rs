#[allow(dead_code, non_camel_case_types)]
#[derive(Clone, Copy)]
pub(crate) enum Register {
    CONFIG = 0x0,
    EN_AA = 0x1,
    EN_RXADDR = 0x2,
    SETUP_AW = 0x3,
    SETUP_RETR = 0x4,
    RF_CH = 0x5,
    RF_SETUP = 0x6,
    STATUS = 0x7,
    OBSERVE_TX = 0x8,
    CD = 0x9,
    RX_ADDR_P0 = 0xa,
    RX_ADDR_P1 = 0xb,
    RX_ADDR_P2 = 0xc,
    RX_ADDR_P3 = 0xd,
    RX_ADDR_P4 = 0xe,
    RX_ADDR_P5 = 0xf,
    TX_ADDR = 0x10,
    RX_PW_P0 = 0x11,
    RX_PW_P1 = 0x12,
    RX_PW_P2 = 0x13,
    RX_PW_P3 = 0x14,
    RX_PW_P4 = 0x15,
    RX_PW_P5 = 0x16,
    FIFO_STATUS = 0x17,
    DYNPD = 0x1c,
    FEATURE = 0x1d,
    R_RX_PL_WID = 0x60,
}

impl Register {
    pub(crate) fn addr(&self) -> u8 {
        *self as u8
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Instruction {
    /// Read registers
    RR = 0b0000_0000,
    /// Write registers
    /// Last 5 bits are the Memory Map Adress
    WR = 0b0010_0000,
    /// Read RX-payload, used in RX mode.
    RRX = 0b0110_0001,
    /// Write TX-payload, used in TX mode.
    WTX = 0b1010_0000,
    /// Flush TX FIFO, used in TX mode.
    FTX = 0b1110_0001,
    /// Flush RX FIFO, used in RX mode.
    FRX = 0b1110_0010,
    /// No operation. Might be used to read STATUS register.
    NOP = 0b1111_1111,
}

impl Instruction {
    pub(crate) fn opcode(&self) -> u8 {
        *self as u8
    }
}
