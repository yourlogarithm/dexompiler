use core::fmt;
use std::error::Error;

use num_traits::FromPrimitive;

use crate::opcode::Opcode;


#[macro_export]
macro_rules! concat_words {
    // Case for concatenating 4 words into u64
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        ((($b as u64) << 48) |
         (($a as u64) << 32) |
         (($d as u64) << 16) |
          ($c as u64))
    };
    // Case for concatenating 2 words into u32
    ($a:expr, $b:expr) => {
        (($b as u32) << 16) | ($a as u32)
    };
}

macro_rules! split_word {
    ($word:expr) => {
        (($word & 0xff) as _, ($word >> 8) as _)
    };
}


#[derive(Debug)]
pub struct InstructionParsingError {
    byte: u8,
    offset: usize,
}

impl Error for InstructionParsingError {}

impl fmt::Display for InstructionParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid instruction at offset {}: {}", self.offset, self.byte)
    }
}


#[derive(Debug, PartialEq)]
pub struct Instruction {
    /// The opcode of the instruction
    opcode: Opcode,
    /// The offset of the instruction in the method bytecode
    offset: usize,
    /// Branch target of the instruction
    branch_target: Option<usize>,
}


impl Instruction {
    pub fn try_from_raw_bytecode(raw_bytecode: &[u16], offset: usize) -> Result<Option<(Self, usize)>, InstructionParsingError>  {
        let raw_bytecode = &raw_bytecode[offset..];
        let (opcode_byte, immediate_args) = split_word!(raw_bytecode[0]);
        let opcode: Opcode = FromPrimitive::from_u8(opcode_byte).ok_or(InstructionParsingError { byte: opcode_byte, offset: offset })?;

        let (length, branch_target) = match opcode_byte {
            0x0 => {
                if (1..=3).contains(&immediate_args) {
                    return Ok(None);
                }
                (1, None)
            },
            0x01 | 0x04 | 0x07 | 0x0A..=0x12 | 0x1D | 0x1E | 0x21 | 0x27 | 0x7B..=0x8F | 0xB0..=0xCF => (1, None),
            0x02 | 0x05 | 0x08 | 0x13 | 0x15 | 0x16 | 0x19 | 0x1A | 0x1C | 0x1F | 0x20 | 0x22 | 0x23 | 0x2D..=0x31 | 0x44..=0x6D | 0x90..=0xAF | 0xD0..=0xE2 | 0xFE | 0xFF => {
                if raw_bytecode.len() < 2 {
                    return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
                }
                (2, None)
            },
            0x03 | 0x06 | 0x09 | 0x14 | 0x17 | 0x1B | 0x24..=0x26 | 0x6E..=0x72 | 0x74..=0x78 | 0xFC | 0xFD => {
                if raw_bytecode.len() < 3 {
                    return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
                }
                (3, None)
            },
            0xFA | 0xFB => (4, None),
            0x18 => (5, None),
            0x28 => (1, Some(immediate_args as i8 as i32)),
            0x29 => (2, Some(immediate_args as i16 as i32)),
            0x2A => {
                if raw_bytecode.len() < 3 {
                    return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
                }
                (3, Some(concat_words!(raw_bytecode[1], raw_bytecode[2]) as i32))
            },
            0x2B | 0x2C => {
                if raw_bytecode.len() < 3 {
                    return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
                }
                (3, Some(concat_words!(raw_bytecode[1], raw_bytecode[2]) as i32))
            },
            0x32..=0x3D => {
                if raw_bytecode.len() < 2 {
                    return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
                }
                (2, Some(raw_bytecode[1] as i16 as i32))
            },
            0x3e..=0x43 | 0x73 | 0x79..=0x7a | 0xe3..=0xf9 => {
                return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
            }
        };
        if length > raw_bytecode.len() {
            return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
        }
        let branch_target = match branch_target {
            Some(target) => Some((target + offset as i32) as usize),
            None => None
        };
        Ok(Some((Instruction { opcode, offset, branch_target }, length)))
    }

    pub fn opcode(&self) -> &Opcode {
        &self.opcode
    }

    pub fn offset(&self) -> &usize {
        &self.offset
    }

    pub fn branch_target(&self) -> &Option<usize> {
        &self.branch_target
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_try_from_raw_bytecode0() {
        let raw_bytecode = [8303, 921, 33];
        let (instruction, length) = Instruction::try_from_raw_bytecode(&raw_bytecode, 0).unwrap().expect("Failed to parse instruction");
        assert!(length == 3);
        assert_eq!(instruction, Instruction { opcode: Opcode::InvokeSuper, offset: 0, branch_target: None });
    }

    #[test]
    fn test_try_from_raw_bytecode1() {
        let raw_bytecode = [45874, 102];
        let (instruction, length) = Instruction::try_from_raw_bytecode(&raw_bytecode, 0).unwrap().expect("Failed to parse instruction");
        assert_eq!(length, 2);
        assert_eq!(instruction, Instruction { opcode: Opcode::IfEq, offset: 0, branch_target: Some(102) });
    }

    #[test]
    fn test_try_from_raw_bytecode2() {
        let raw_bytecode = [290, 648];
        let (instruction, length) = Instruction::try_from_raw_bytecode(&raw_bytecode, 0).unwrap().expect("Failed to parse instruction");
        assert_eq!(length, 2);
        assert_eq!(instruction, Instruction { opcode: Opcode::NewInstance, offset: 0, branch_target: None });
    }
}