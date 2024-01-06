#[derive(Debug)]
pub enum ConstantPoolIndex {
    U16(u16),
    U32(u32)
}

#[derive(Debug)]
pub enum ImmediateValue {
    Signed8(i8),
    Signed16(i16),
    Signed32(i32),
    Arbitrary32(u32),
    Float32(f32),
    Signed64(i64),
    Arbitrary64(u64),
    Float64(f64)
}

#[derive(Debug)]
pub enum BranchOffset {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32)
}

#[derive(Debug)]
pub enum Register {
    Byte(u8),
    Word(u16),
    Pair(u8, u8)
}

impl Register {
    pub fn get_pairs(word: u8) -> (Register, Register) {
        let dst = word & 0xF;
        let src = word >> 4;
        (
            Register::Pair(dst, dst + 1),
            Register::Pair(src, src + 1),
        )
    }

    pub fn get_dst_pair(word: u8) -> (Register, Register) {
        let dst = word & 0xF;
        let src = word >> 4;
        (
            Register::Pair(dst, dst + 1),
            Register::Byte(src),
        )
    }

    pub fn get_src_pair(word: u8) -> (Register, Register) {
        let dst = word & 0xF;
        let src = word >> 4;
        (
            Register::Byte(dst),
            Register::Pair(src, src + 1),
        )
    }
}

#[derive(Debug)]
pub enum Switch {
    Packed(Register, BranchOffset),
    Sparse(Register, BranchOffset)
}

#[derive(Debug)]
pub enum Conditional {
    Eq(Register, Register),
    Ne(Register, Register),
    Lt(Register, Register),
    Ge(Register, Register),
    Gt(Register, Register),
    Le(Register, Register)
}

#[derive(Debug)]
pub enum If {
    Test(Conditional, BranchOffset),
    Testz(Conditional, BranchOffset),
}

#[derive(Debug)]
pub enum Iop {
    Int(Register),
    Wide((Register, Register)),
    Object(Register),
    Boolean(Register),
    Byte(Register),
    Char(Register),
    Short(Register)
}

#[derive(Debug)]
pub enum IdentifiedOperation {
    Get(Iop),
    Put(Iop)
}

#[derive(Debug)]
pub enum InvKind {
    Virtual(ConstantPoolIndex, Vec<Register>),
    Super(ConstantPoolIndex, Vec<Register>),
    Direct(ConstantPoolIndex, Vec<Register>),
    Static(ConstantPoolIndex, Vec<Register>),
    Interface(ConstantPoolIndex, Vec<Register>)
}

#[derive(Debug)]
pub enum Primitive {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double
}

#[derive(Debug)]
pub enum Unop {
    Neg(Primitive),
    Not(Primitive),
    Convert(Primitive, Primitive)
}

#[derive(Debug)]
pub enum Binop {
    Add(Primitive),
    Sub(Primitive),
    Mul(Primitive),
    Div(Primitive),
    Rem(Primitive),
    And(Primitive),
    Or(Primitive),
    Xor(Primitive),
    Shl(Primitive),
    Shr(Primitive),
    Ushr(Primitive)
}