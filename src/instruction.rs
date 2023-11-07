use core::fmt;
use std::error::Error;

use dex::Dex;
use num_traits::FromPrimitive;

use crate::{opcode::Opcode, reference::{Item, Type}};


#[derive(Debug)]
pub enum Argument {
    PackedRegister(u8),
    WideRegister(u16),
    ImmediateSignedByte(i8),
    ConstantPoolIndex(Item),
    ImmediateSignedHat(i16),
    ImmediateSigned32(u32),  // i32 or f32
    ImmediateSigned64(u64),    // i64 or f64
    ImmediateSignedNibble(i8),
    ImmediateSignedShort(i16),
    BranchTarget(i32),
}

#[derive(Debug)]
pub struct Instruction {
    /// The opcode of the instruction
    opcode: Opcode,
    /// The offset of the instruction in the method bytecode
    offset: usize,
    /// Non-register arguments to the instruction
    arguments: Vec<Argument>,
    /// The length of the instruction in bytes
    pub length: u8,
}


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

#[derive(Debug)]
pub struct InstructionMemberError<'a> {
    instruction: &'a Instruction,
    member: String
}

impl Error for InstructionMemberError<'_> {}

impl fmt::Display for InstructionMemberError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid instruction member {} for instruction {:?}", self.member, self.instruction)
    }
}


impl Instruction {
    pub fn try_from_raw_bytecode(raw_bytecode: &[u16], offset: usize, dex: &Dex<impl AsRef<[u8]>>) -> Result<Option<Self>, InstructionParsingError>  {
        let raw_bytecode = &raw_bytecode[offset..];
        let hex_view = raw_bytecode.iter().map(|x| x.to_le_bytes()).flatten().collect::<Vec<u8>>();
        let (opcode_byte, immediate_args) = split_word!(raw_bytecode[0]);
        let opcode: Opcode = FromPrimitive::from_u8(opcode_byte).ok_or(InstructionParsingError { byte: opcode_byte, offset: offset })?;

        let (arguments, length) = match opcode_byte {
            0x0 => {
                if (1..=3).contains(&immediate_args) {
                    return Ok(None);
                }
                (vec![], 1)
            },
            0x1 | 0x4 | 0x7 | 0xa..=0x11 | 0x1d | 0x1e | 0x21 | 0x27 | 0x7b..=0x8f | 0xb0..=0xcf => (vec![Argument::PackedRegister(immediate_args)], 1),
                0x2 | 0x5 | 0x8 =>  (vec![Argument::PackedRegister(immediate_args), Argument::WideRegister(raw_bytecode[1])], 2),
                0x2d..=0x31 | 0x44..=0x51 | 0x90..=0xaf => {
                    let splitted = split_word!(raw_bytecode[1]);
                    (vec![Argument::PackedRegister(immediate_args), Argument::PackedRegister(splitted.0), Argument::PackedRegister(splitted.1)], 2)
                },
                0x3 | 0x6 | 0x9 => (vec![Argument::WideRegister(raw_bytecode[1]), Argument::WideRegister(raw_bytecode[2])], 3),
                0xd8..=0xe2 => {
                    let splitted = split_word!(raw_bytecode[1]); 
                    (vec![Argument::PackedRegister(immediate_args), Argument::PackedRegister(splitted.0), Argument::ImmediateSignedByte(splitted.1)], 2)
                },
                0x1a => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::String, raw_bytecode[1], dex))], 2),
                0x1c | 0x1f | 0x20 | 0x22 | 0x23 => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::Type, raw_bytecode[1], dex))], 2),
                0x52..=0x6d => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::Field, raw_bytecode[1], dex))], 2),
                0xfe => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::MethodHandle, raw_bytecode[1], dex))], 2),
                0xff => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::Prototype, raw_bytecode[1], dex))], 2),
                0x1b => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_index(Type::String, concat_words!(raw_bytecode[1], raw_bytecode[2]), dex))], 3),
                0x24 | 0x25 => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::Type, raw_bytecode[1], dex)), Argument::WideRegister(raw_bytecode[2])], 3),  // TODO: Ensure correct parsing
                0x6e..=0x72 | 0x74..=0x78 => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::Method, raw_bytecode[1], dex)), Argument::WideRegister(raw_bytecode[2])], 3),  // TODO: Ensure correct parsing
                0xfc | 0xfd => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(Item::from_short_index(Type::CallSite, raw_bytecode[1], dex)), Argument::WideRegister(raw_bytecode[2])], 3),  // TODO: Ensure correct parsing
                0xfa | 0xfb => (vec![Argument::ConstantPoolIndex(Item::from_short_index(Type::Method, raw_bytecode[1], dex)), Argument::ConstantPoolIndex(Item::from_short_index(Type::Prototype, raw_bytecode[3], dex))], 4),  // TODO: Parse registers
                0x15 | 0x19 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSignedHat(raw_bytecode[1] as i16)], 2),
                0x14 | 0x17 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSigned32(concat_words!(raw_bytecode[1], raw_bytecode[2]))], 3),
                0x18 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSigned64(concat_words!(raw_bytecode[1], raw_bytecode[2], raw_bytecode[3], raw_bytecode[4]))], 5),
                0x12 => (vec![Argument::PackedRegister(immediate_args >> 4), Argument::ImmediateSignedNibble((immediate_args & 0xf) as i8)], 1),
                0x13 | 0x16 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSignedShort(raw_bytecode[1] as i16)], 2), 
                0xd0..=0xd7 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSignedShort(raw_bytecode[1] as i16)], 2),
                0x28 => (vec![Argument::BranchTarget(immediate_args as i32)], 1),
                0x29 => (vec![Argument::BranchTarget(raw_bytecode[1] as i32)], 2),
                0x32..=0x3d => (vec![Argument::PackedRegister(immediate_args), Argument::BranchTarget(raw_bytecode[1] as i32)], 2),
                0x26 => (vec![Argument::PackedRegister(immediate_args), Argument::BranchTarget(concat_words!(raw_bytecode[1], raw_bytecode[2]) as i32)], 3),
                0x2a => (vec![Argument::BranchTarget(concat_words!(raw_bytecode[1], raw_bytecode[2]) as i32)], 3),
                0x2b | 0x2c => (vec![Argument::PackedRegister(immediate_args), Argument::BranchTarget(concat_words!(raw_bytecode[1], raw_bytecode[2]) as i32)], 3),
                0x3e..=0x43 | 0x73 | 0x79..=0x7a | 0xe3..=0xf9 => {
                    println!("{:?}", hex_view);
                    return Err(InstructionParsingError { byte: opcode_byte, offset: offset });
                }
        };

        Ok(Some(Instruction { offset, opcode, arguments, length } ))
    }

    pub fn offset(&self) -> &usize {
        &self.offset
    }

    fn opcode(&self) -> &Opcode {
        &self.opcode
    }

    fn arguments(&self) -> &Vec<Argument> {
        &self.arguments
    }

    pub fn is_terminator(&self) -> bool {
        match *self.opcode() as u8 {
            0x32..=0x3D | 0x27..=0x2C | 0x0E ..=0x11 => true,
            _ => false,
        }
    }

    pub fn jump_target(&self) -> Option<usize> {
        match *self.opcode() as u8 {
            0x28 | 0x29 | 0x2A => {
                if let Argument::BranchTarget(target) = self.arguments().first().unwrap() {
                    return Some((*self.offset() as i32 + *target) as usize);
                }
            }
            0x2B | 0x2C | 0x32..=0x3d => {
                if let Argument::BranchTarget(target) = self.arguments().last().unwrap() {
                    return Some((*self.offset() as i32 + *target) as usize);
                }
            }
            _ => (),
        }
        None
    }
}


#[cfg(test)]
mod test {
    use dex::DexReader;

    use super::*;

    #[test]
    fn test_parsing_0() {
        let raw_bytecode = [8303, 921, 33, 8304, 29798, 33, 10, 56, 3, 14, 8304, 29799, 33, 8304, 29797, 33, 14];
        let dex = DexReader::from_file("tests/test.dex").unwrap();
        let instruction = Instruction::try_from_raw_bytecode(&raw_bytecode, 0, &dex).unwrap().expect("Failed to parse instruction");
        assert_eq!(instruction.opcode(), &Opcode::InvokeSuper);
        assert_eq!(instruction.arguments()[0], )
    }
}