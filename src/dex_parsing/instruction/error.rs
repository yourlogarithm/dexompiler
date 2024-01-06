use core::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct InstructionParsingError {
    pub(super) byte: u8,
    pub(super) offset: usize,
}

impl Error for InstructionParsingError {}

impl fmt::Display for InstructionParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid instruction at offset {}: {}", self.offset, self.byte)
    }
}