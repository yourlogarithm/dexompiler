use dex::Dex;

use crate::{opcode::Opcode, reference::{Reference, Type}, ref_from_dex};


#[derive(Debug)]
enum Argument {
    PackedRegister(u8),
    WideRegister(u16),
    ImmediateSignedByte(i8),
    ConstantPoolIndex(Reference),
    ImmediateSignedHat(i16),
    ImmediateSigned32(u32),  // i32 or f32
    ImmediateSigned64(u64),    // i64 or f64
    ImmediateSignedNibble(i8),
    ImmediateSignedShort(i16),
    BranchTarget8(i8),
    BranchTarget16(i16),
    BranchTarget32(i32),
}

#[derive(Debug)]
pub struct Instruction {
    /// The opcode of the instruction
    opcode: Opcode,
    /// Non-register arguments to the instruction
    arguments: Vec<Argument>,
}


macro_rules! concat_words {
    // Case for concatenating 4 words into u64
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        ((($a as u64) << 48) |
         (($b as u64) << 32) |
         (($c as u64) << 16) |
          ($d as u64))
    };
    // Case for concatenating 2 words into u32
    ($a:expr, $b:expr) => {
        (($a as u32) << 16) | ($b as u32)
    };
}

macro_rules! split_word {
    ($word:expr) => {
        (($word & 0xff) as _, ($word >> 8) as _)
    };
}

impl Instruction {
    pub fn opcode(&self) -> &Opcode {
        &self.opcode
    }

    pub fn arguments(&self) -> &Vec<Argument> {
        &self.arguments
    }

    pub fn try_from_raw_bytecode<T>(raw_bytecode: &[u16], offset: usize, dex: &Dex<T>) -> Result<Option<(Self, u8)>, &'static str> where T: AsRef<[u8]> {
        let raw_bytecode = &raw_bytecode[offset..];
        let hex_view = raw_bytecode.iter().map(|x| x.to_le_bytes()).flatten().collect::<Vec<u8>>();
        let (opcode_byte, immediate_args) = split_word!(raw_bytecode[0]);
        let opcode = Opcode::try_from(opcode_byte).map_err(|_| "Invalid opcode")?;

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
                0x1a => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::String, raw_bytecode[1], dex))], 2),
                0x1c | 0x1f | 0x20 | 0x22 | 0x23 => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::Type, raw_bytecode[1], dex))], 2),
                0x52..=0x6d => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::Field, raw_bytecode[1], dex))], 2),
                0xfe => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::MethodHandle, raw_bytecode[1], dex))], 2),
                0xff => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::Prototype, raw_bytecode[1], dex))], 2),
                0x1b => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::String, concat_words!(raw_bytecode[2], raw_bytecode[1]), dex))], 3),
                0x24 | 0x25 => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::Type, raw_bytecode[1], dex)), Argument::WideRegister(raw_bytecode[2])], 3),  // TODO: Ensure correct parsing
                0x6e..=0x72 | 0x74..=0x78 => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::Method, raw_bytecode[1], dex)), Argument::WideRegister(raw_bytecode[2])], 3),  // TODO: Ensure correct parsing
                0xfc | 0xfd => (vec![Argument::PackedRegister(immediate_args), Argument::ConstantPoolIndex(ref_from_dex!(Type::CallSite, raw_bytecode[1], dex)), Argument::WideRegister(raw_bytecode[2])], 3),  // TODO: Ensure correct parsing
                0xfa | 0xfb => (vec![Argument::ConstantPoolIndex(ref_from_dex!(Type::Method, raw_bytecode[1], dex)), Argument::ConstantPoolIndex(ref_from_dex!(Type::Prototype, raw_bytecode[3], dex))], 4),  // TODO: Parse registers
                0x15 | 0x19 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSignedHat(raw_bytecode[1] as i16)], 2),
                0x14 | 0x17 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSigned32(concat_words!(raw_bytecode[2], raw_bytecode[1]))], 3),
                0x18 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSigned64(concat_words!(raw_bytecode[4], raw_bytecode[3], raw_bytecode[2], raw_bytecode[1]))], 5),
                0x12 => (vec![Argument::PackedRegister(immediate_args >> 4), Argument::ImmediateSignedNibble((immediate_args & 0xf) as i8)], 1),
                0x13 | 0x16 => (vec![Argument::PackedRegister(immediate_args), Argument::ImmediateSignedShort(raw_bytecode[1] as i16)], 2), 
                0xd0..=0xd7 => (vec![Argument::PackedRegister(immediate_args >> 4), Argument::PackedRegister(immediate_args & 0xf), Argument::ImmediateSignedShort(raw_bytecode[1] as i16)], 2),
                0x28 => (vec![Argument::BranchTarget8(immediate_args as i8)], 1),
                0x29 => (vec![Argument::BranchTarget16(raw_bytecode[1] as i16)], 2),
                0x32..=0x3d => (vec![Argument::PackedRegister(immediate_args >> 4), Argument::PackedRegister(immediate_args & 0xf), Argument::BranchTarget16(raw_bytecode[1] as i16)], 2),
                0x26 => (vec![Argument::PackedRegister(immediate_args), Argument::BranchTarget32(concat_words!(raw_bytecode[2], raw_bytecode[1]) as i32)], 3),
                0x2a..=0x2c => (vec![Argument::BranchTarget32(concat_words!(raw_bytecode[2], raw_bytecode[1]) as i32)], 3),
                0x3e..=0x43 | 0x73 | 0x79..=0x7a | 0xe3..=0xf9 => {
                    println!("{:?}", hex_view);
                    return Err("Unimplemented") 
                }
        };

        Ok(Some((Instruction { opcode, arguments }, length)))
    }
}