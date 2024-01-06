mod opcode;
mod error;
mod intermediate;

use intermediate::*;
use error::InstructionParsingError;
use num_traits::FromPrimitive;
use serde::Serialize;

use self::opcode::Opcode;


macro_rules! split_word {
    ($word:expr) => {
        (
            ($word & 0xFF) as _, 
            ($word >> 8) as _
        )
    };
}

macro_rules! split_registers {
    ($word:expr) => {
        (
            Register::Byte(($word & 0xF) as _), 
            Register::Byte(($word >> 4) as _)
        )
    }
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

macro_rules! identifiable_operation {
    ($opcode_byte:expr, $immediate_args:expr, $dst:expr) => {
        match $opcode_byte {
            0x44 | 0x52 | 0x60 => IdentifiedOperation::Get(Iop::Int($dst)),
            0x45 | 0x53 | 0x61 => {
                let dst1 = Register::Byte($immediate_args + 1);
                IdentifiedOperation::Get(Iop::Wide(($dst, dst1)))
            },
            0x46 | 0x54 | 0x62 => IdentifiedOperation::Get(Iop::Object($dst)),
            0x47 | 0x55 | 0x63 => IdentifiedOperation::Get(Iop::Boolean($dst)),
            0x48 | 0x56 | 0x64 => IdentifiedOperation::Get(Iop::Byte($dst)),
            0x49 | 0x57 | 0x65 => IdentifiedOperation::Get(Iop::Char($dst)),
            0x4A | 0x58 | 0x66 => IdentifiedOperation::Get(Iop::Short($dst)),
            0x4B | 0x59 | 0x67 => IdentifiedOperation::Put(Iop::Int($dst)),
            0x4C | 0x5A | 0x68 => {
                let dst1 = Register::Byte($immediate_args + 1);
                IdentifiedOperation::Put(Iop::Wide(($dst, dst1)))
            },
            0x4D | 0x5B | 0x69 => IdentifiedOperation::Put(Iop::Object($dst)),
            0x4E | 0x5C | 0x6A => IdentifiedOperation::Put(Iop::Boolean($dst)),
            0x4F | 0x5D | 0x6B => IdentifiedOperation::Put(Iop::Byte($dst)),
            0x50 | 0x5E | 0x6C => IdentifiedOperation::Put(Iop::Char($dst)),
            0x51 | 0x5F | 0x6D => IdentifiedOperation::Put(Iop::Short($dst)),
            _ => unreachable!()
        }
    }
}

macro_rules! decode_invocation_parameters {
    ($raw_bytecode:expr, $immediate_args:expr) => {{
        let sz = $immediate_args >> 4;
        let mut regs = Vec::with_capacity(sz as usize);

        for i in 0..sz {
            // Starting if parameter number > 4 and parameter number % 4 == 1, 
            // the 5th (9th, etc.) parameter is encoded on the 4 lowest bit of the byte immediately following the instruction. 
            // Curiously, this encoding is not used in case of 1 parameter, 
            // in this case an entire 16 bit word is added after the method index of which only 4 bit is used to encode the single parameter while the lowest 4 bit of the byte following the instruction byte is left unused.
            if i == 4 || i == 8 {
                regs.push(Register::Byte(($immediate_args & 0xF) as _));
            } else {
                let nibble = ($raw_bytecode >> (i * 4)) & 0xF;
                regs.push(Register::Byte(nibble as _));
            }
        }

        regs
    }};
}

macro_rules! binop_args {
    ($dst_byte:expr, $srcs_word:expr, $binop:expr, $primitive:expr) => {{
        let (v0, v1) = split_word!($srcs_word);
        let v0 = Register::Byte(v0);
        let v1 = Register::Byte(v1);
        let dst = Register::Byte($dst_byte);
        (Instruction::Binop($binop($primitive), dst, v0, v1), 2)
    }}
}


#[derive(Debug)]
pub enum Instruction {
    Nop,
    Move(Register, Register),
    MoveWide(Register, Register),
    MoveObject(Register, Register),
    MoveResult(Register),
    MoveResultWide(Register),
    MoveResultObject(Register),
    MoveException(Register),
    Return(Option<Register>),
    ReturnWide(Register),
    ReturnObject(Register),
    Const(Register, ImmediateValue),
    ConstString(Register, ConstantPoolIndex),
    ConstClass(Register, ConstantPoolIndex),
    MonitorEnter(Register),
    MonitorExit(Register),
    CheckCast(Register, ConstantPoolIndex),
    InstanceOf(Register, Register, ConstantPoolIndex),
    ArrayLength(Register, Register),
    NewInstance(Register, ConstantPoolIndex),
    NewArray(Register, Register, ConstantPoolIndex),
    FilledNewArray(ConstantPoolIndex, Vec<Register>),
    FillArrayData(Register, BranchOffset),
    Throw(Register),
    Goto(BranchOffset),
    Switch(Switch),
    Cmp(Register, Register, Register),
    If(If),
    ArrayOp(IdentifiedOperation, Register, Register),
    InstanceOp(IdentifiedOperation, Register, ConstantPoolIndex),
    StaticOp(IdentifiedOperation, ConstantPoolIndex),
    InvokeKind(InvKind),
    Unop(Unop, Register, Register),
    Binop(Binop, Register, Register, Register),
    Binop2Addr(Binop, Register, Register),
    BinopLit(Binop, Register, Register, ImmediateValue),
    InvokePolymorphic(u8, ConstantPoolIndex, u8, Vec<Register>, ConstantPoolIndex),
    InvokeCustom(u8, ConstantPoolIndex, Vec<Register>),
    ConstMethodHandle(Register, ConstantPoolIndex),
    ConstMethodType(Register, ConstantPoolIndex),
}

impl Instruction {
    pub fn try_from_raw_bytecode(raw_bytecode: &[u16], offset: &mut usize) -> Result<Option<Instruction>, InstructionParsingError> {
        let raw_bytecode = &raw_bytecode[*offset..];
        let (opcode_byte, immediate_args) = split_word!(raw_bytecode[0]);
        let opcode: Opcode = FromPrimitive::from_u8(opcode_byte).ok_or(InstructionParsingError { byte: opcode_byte, offset: *offset })?;
        let (instruction, length) = match opcode_byte {
            0x00 => {
                if (1..=3).contains(&immediate_args) {
                    return Ok(None);
                }
                (Instruction::Nop, 1)
            },
            0x01 => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Move(dst, src), 1)
            },
            0x02 => {
                let dst = Register::Byte(immediate_args);
                let src = Register::Word(raw_bytecode[1]);
                (Instruction::Move(dst, src), 2)
            },
            0x03 => {
                let dst = Register::Word(raw_bytecode[1]);
                let src = Register::Word(raw_bytecode[2]);
                (Instruction::Move(dst, src), 3)
            },
            0x04 => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::MoveWide(dst, src), 1)
            },
            0x05 => {
                let dst = Register::Byte(immediate_args);
                let src = Register::Word(raw_bytecode[1]);
                (Instruction::MoveWide(dst, src), 2)
            },
            0x06 => {
                let dst = Register::Word(raw_bytecode[1]);
                let src = Register::Word(raw_bytecode[2]);
                (Instruction::MoveWide(dst, src), 3)
            },
            0x07 => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::MoveObject(dst, src), 1)
            },
            0x08 => {
                let dst = Register::Byte(immediate_args);
                let src = Register::Word(raw_bytecode[1]);
                (Instruction::MoveObject(dst, src), 2)
            },
            0x09 => {
                let dst = Register::Word(raw_bytecode[1]);
                let src = Register::Word(raw_bytecode[2]);
                (Instruction::MoveObject(dst, src), 3)
            },
            0x0A => {
                let dst = Register::Byte(immediate_args);
                (Instruction::MoveResult(dst), 1)
            },
            0x0B => {
                let dst = Register::Byte(immediate_args);
                (Instruction::MoveResultWide(dst), 1)
            },
            0x0C => {
                let dst = Register::Byte(immediate_args);
                (Instruction::MoveResultObject(dst), 1)
            },
            0x0D => {
                let dst = Register::Byte(immediate_args);
                (Instruction::MoveException(dst), 1)
            },
            0x0E => {
                (Instruction::Return(None), 1)
            },
            0x0F => {
                let value = Register::Byte(immediate_args);
                (Instruction::Return(Some(value)), 1)
            },
            0x10 => {
                let value = Register::Byte(immediate_args);
                (Instruction::ReturnWide(value), 1)
            },
            0x11 => {
                let value = Register::Byte(immediate_args);
                (Instruction::ReturnObject(value), 1)
            },
            0x12 => {
                let dst = Register::Byte(immediate_args & 0xF0);
                let literal = ImmediateValue::Signed8((immediate_args & 0x0F) as i8);
                (Instruction::Const(dst, literal), 1)
            },
            0x13 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Signed16(raw_bytecode[1] as _);
                (Instruction::Const(dst, literal), 2)
            },
            0x14 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Arbitrary32(concat_words!(raw_bytecode[1], raw_bytecode[2]));
                (Instruction::Const(dst, literal), 3)
            },
            0x15 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Signed16(raw_bytecode[1] as _);
                (Instruction::Const(dst, literal), 2)
            },
            0x16 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Signed16(raw_bytecode[1] as _);
                (Instruction::Const(dst, literal), 2)
            },
            0x17 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Signed32(concat_words!(raw_bytecode[1], raw_bytecode[2]) as _);
                (Instruction::Const(dst, literal), 3)
            },
            0x18 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Arbitrary64(concat_words!(raw_bytecode[1], raw_bytecode[2], raw_bytecode[3], raw_bytecode[4]) as _);
                (Instruction::Const(dst, literal), 5)
            },
            0x19 => {
                let dst = Register::Byte(immediate_args);
                let literal = ImmediateValue::Signed16(raw_bytecode[1] as _);
                (Instruction::Const(dst, literal), 2)
            },
            0x1A => {
                let dst = Register::Byte(immediate_args);
                let str_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                (Instruction::ConstString(dst, str_idx), 2)
            },
            0x1B => {
                let dst = Register::Byte(immediate_args);
                let str_idx = ConstantPoolIndex::U32(concat_words!(raw_bytecode[1], raw_bytecode[2]));
                (Instruction::ConstString(dst, str_idx), 3)
            },
            0x1C => {
                let dst = Register::Byte(immediate_args);
                let cls_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                (Instruction::ConstClass(dst, cls_idx), 2)
            },
            0x1D => {
                let reg = Register::Byte(immediate_args);
                (Instruction::MonitorEnter(reg), 1)
            },
            0x1E => {
                let reg = Register::Byte(immediate_args);
                (Instruction::MonitorExit(reg), 1)
            },
            0x1F => {
                let reg = Register::Byte(immediate_args);
                let cls_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                (Instruction::CheckCast(reg, cls_idx), 2)
            },
            0x20 => {
                let (dst, src) = split_registers!(immediate_args);
                let cls_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                (Instruction::InstanceOf(dst, src, cls_idx), 2)
            },
            0x21 => {
                let (dst, arr_ref) = split_registers!(immediate_args);
                (Instruction::ArrayLength(dst, arr_ref), 1)
            },
            0x22 => {
                let dst = Register::Byte(immediate_args);
                let type_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                (Instruction::NewInstance(dst, type_idx), 2)
            },
            0x23 => {
                let (dst, size) = split_registers!(immediate_args);
                let type_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                (Instruction::NewArray(dst, size, type_idx), 2)
            },
            0x24 => {
                let type_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                let regs = decode_invocation_parameters!(raw_bytecode[2], immediate_args);
                (Instruction::FilledNewArray(type_idx, regs), 3)
            },
            0x25 => {
                let type_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                let vc = raw_bytecode[2];
                let regs = (vc..=vc + immediate_args as u16)
                    .take(immediate_args as usize)
                    .map(Register::Word)
                    .collect::<Vec<_>>();
                (Instruction::FilledNewArray(type_idx, regs), 3)
            },
            0x26 => {
                let arr_ref = Register::Byte(immediate_args);
                let offset = BranchOffset::I32(concat_words!(raw_bytecode[1], raw_bytecode[2]) as _);
                (Instruction::FillArrayData(arr_ref, offset), 3)
            },
            0x27 => (Instruction::Throw(Register::Byte(immediate_args)), 1),
            0x28 => (Instruction::Goto(BranchOffset::I8(immediate_args as _)), 1),
            0x29 => (Instruction::Goto(BranchOffset::I16(raw_bytecode[1] as _)), 2),
            0x2A => {
                let offset = BranchOffset::I32(concat_words!(raw_bytecode[1], raw_bytecode[2]) as _);
                (Instruction::Goto(offset), 3)
            },
            0x2B => {
                let va = Register::Byte(immediate_args);
                let offset = BranchOffset::I32(concat_words!(raw_bytecode[1], raw_bytecode[2]) as _);
                (Instruction::Switch(Switch::Packed(va, offset)), 3)
            },
            0x2C => {
                let va = Register::Byte(immediate_args);
                let offset = BranchOffset::I32(concat_words!(raw_bytecode[1], raw_bytecode[2]) as _);
                (Instruction::Switch(Switch::Sparse(va, offset)), 3)
            },
            0x2D..=0x31 => {
                let dst = Register::Byte(immediate_args);
                let (vb, vc) = split_word!(raw_bytecode[1]);
                (Instruction::Cmp(dst, Register::Byte(vb), Register::Byte(vc)), 2)
            },
            0x32 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Test(Conditional::Eq(va, vb), offset)), 2)
            },
            0x33 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Test(Conditional::Ne(va, vb), offset)), 2)
            },
            0x34 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Test(Conditional::Lt(va, vb), offset)), 2)
            },
            0x35 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Test(Conditional::Ge(va, vb), offset)), 2)
            },
            0x36 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Test(Conditional::Gt(va, vb), offset)), 2)
            },
            0x37 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Test(Conditional::Le(va, vb), offset)), 2)
            },
            0x38 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Testz(Conditional::Eq(va, vb), offset)), 2)
            },
            0x39 => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Testz(Conditional::Ne(va, vb), offset)), 2)
            },
            0x3A => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Testz(Conditional::Lt(va, vb), offset)), 2)
            },
            0x3B => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _);
                (Instruction::If(If::Testz(Conditional::Ge(va, vb), offset)), 2)
            },
            0x3C => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _); 
                (Instruction::If(If::Testz(Conditional::Gt(va, vb), offset)), 2)
            },
            0x3D => {
                let (va, vb) = split_registers!(immediate_args);
                let offset = BranchOffset::I8(raw_bytecode[1] as _); 
                (Instruction::If(If::Testz(Conditional::Le(va, vb), offset)), 2)
            },
            0x44..=0x51 => {
                let dst = Register::Byte(immediate_args);
                let (arr_reg, idx_reg) = split_registers!(raw_bytecode[1]);
                let iop = identifiable_operation!(opcode_byte, immediate_args, dst);
                (Instruction::ArrayOp(iop, arr_reg, idx_reg), 2)
            },
            0x52..=0x5F => {
                let (dst, obj_reg) = split_registers!(immediate_args);
                let field_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                let iop = identifiable_operation!(opcode_byte, immediate_args, dst);
                (Instruction::InstanceOp(iop, obj_reg, field_idx), 2)
            },
            0x60..=0x6D => {
                let dst = Register::Byte(immediate_args);
                let static_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                let iop = identifiable_operation!(opcode_byte, immediate_args, dst);
                (Instruction::StaticOp(iop, static_idx), 2)
            },
            0x6E..=0x72 => {
                let method_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                let regs = decode_invocation_parameters!(raw_bytecode[2], immediate_args);
                let ivk = match opcode_byte {
                    0x6E => InvKind::Virtual(method_idx, regs),
                    0x6F => InvKind::Super(method_idx, regs),
                    0x70 => InvKind::Direct(method_idx, regs),
                    0x71 => InvKind::Static(method_idx, regs),
                    0x72 => InvKind::Interface(method_idx, regs),
                    _ => unreachable!()
                };
                (Instruction::InvokeKind(ivk), 3)
            },
            0x74..=0x78 => {
                let method_idx = ConstantPoolIndex::U16(raw_bytecode[1]);
                let vc = raw_bytecode[2];
                let regs = (vc..=vc + immediate_args as u16)
                    .take(immediate_args as usize)
                    .map(Register::Word)
                    .collect::<Vec<_>>();
                let ivk = match opcode_byte {
                    0x74 => InvKind::Virtual(method_idx, regs),
                    0x75 => InvKind::Super(method_idx, regs),
                    0x76 => InvKind::Direct(method_idx, regs),
                    0x77 => InvKind::Static(method_idx, regs),
                    0x78 => InvKind::Interface(method_idx, regs),
                    _ => unreachable!()
                };
                (Instruction::InvokeKind(ivk), 3)
            },
            0x7B => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Neg(Primitive::Int), dst, src), 1)
            },
            0x7C => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Not(Primitive::Int), dst, src), 1)
            },
            0x7D => {
                let (dst_pair, src_pair) = Register::get_pairs(immediate_args);
                (Instruction::Unop(Unop::Neg(Primitive::Long), dst_pair, src_pair), 1)
            },
            0x7E => {
                let (dst_pair, src_pair) = Register::get_pairs(immediate_args);
                (Instruction::Unop(Unop::Not(Primitive::Long), dst_pair, src_pair), 1)
            },
            0x7F => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Neg(Primitive::Float), dst, src), 1)
            },
            0x80 => {
                let (dst_pair, src_pair) = Register::get_pairs(immediate_args);
                (Instruction::Unop(Unop::Neg(Primitive::Double), dst_pair, src_pair), 1)
            },
            0x81 => {
                let (dst_pair, src) = Register::get_dst_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Int, Primitive::Long), dst_pair, src), 1)
            },
            0x82 => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Int, Primitive::Float), dst, src), 1)
            },
            0x83 => {
                let (dst_pair, src) = Register::get_dst_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Int, Primitive::Double), dst_pair, src), 1)
            },
            0x84 => {
                let (dst, src_pair) = Register::get_src_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Long, Primitive::Int), dst, src_pair), 1)
            },
            0x85 => {
                let (dst, src_pair) = Register::get_src_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Long, Primitive::Float), dst, src_pair), 1)
            },
            0x86 => {
                let (dst_pair, src_pair) = Register::get_pairs(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Long, Primitive::Double), dst_pair, src_pair), 1)
            },
            0x87 => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Float, Primitive::Int), dst, src), 1)
            },
            0x88 => {
                let (dst_pair, src) = Register::get_dst_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Float, Primitive::Long), dst_pair, src), 1)
            },
            0x89 => {
                let (dst_pair, src) = Register::get_dst_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Float, Primitive::Double), dst_pair, src), 1)
            },
            0x8A => {
                let (dst, src_pair) = Register::get_src_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Double, Primitive::Int), dst, src_pair), 1)
            },
            0x8B => {
                let (dst_pair, src_pair) = Register::get_pairs(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Double, Primitive::Long), dst_pair, src_pair), 1)
            },
            0x8C => {
                let (dst, src_pair) = Register::get_src_pair(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Double, Primitive::Float), dst, src_pair), 1)
            },
            0x8D => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Int, Primitive::Byte), dst, src), 1)
            },
            0x8E => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Int, Primitive::Char), dst, src), 1)
            },
            0x8F => {
                let (dst, src) = split_registers!(immediate_args);
                (Instruction::Unop(Unop::Convert(Primitive::Int, Primitive::Short), dst, src), 1)
            },
            0x90..=0x9A => {
                let binop = match opcode_byte {
                    0x90 => Binop::Add,
                    0x91 => Binop::Sub,
                    0x92 => Binop::Mul,
                    0x93 => Binop::Div,
                    0x94 => Binop::Rem,
                    0x95 => Binop::And,
                    0x96 => Binop::Or,
                    0x97 => Binop::Xor,
                    0x98 => Binop::Shl,
                    0x99 => Binop::Shr,
                    0x9A => Binop::Ushr,
                    _ => unreachable!()
                };
                binop_args!(immediate_args, raw_bytecode[1], binop, Primitive::Int)
            },
            0x91 => {
                binop_args!(immediate_args, raw_bytecode[1], Binop::Sub, Primitive::Int)
            }
            _ => return Err(InstructionParsingError { byte: opcode_byte, offset: *offset })
        };
        *offset += length;
        Ok(Some(instruction))
    }
}