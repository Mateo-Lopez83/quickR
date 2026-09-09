use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketError {
    WrongBufferSize,
    InvalidChannel(u8),
    InvalidFragmenttype(u8),
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketError::WrongBufferSize => write!(f, "Buffer too short to parse packet"),
            PacketError::InvalidChannel(b) => write!(f, "Invalid channel byte received: {:#04X}", b),
            PacketError::InvalidFragmenttype(b) => write!(f, "Invalid fragment byte received: {:#04X}", b),
        }
    }
}

// Implement std::error::Error to integrate with the Rust ecosystem
impl std::error::Error for PacketError {}