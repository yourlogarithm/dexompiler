use std::{rc::Rc, cell::RefCell, fmt, collections::{HashSet, HashMap}, os::linux::raw};

use crate::instruction::Instruction;

pub struct BasicBlock {
    offset: usize,
    exc_entry: bool,
    prev: Vec<Rc<RefCell<BasicBlock>>>,
    instructions: Vec<Rc<Instruction>>,
    succ: Vec<Rc<RefCell<BasicBlock>>>,
    visited: bool,
}

impl fmt::Debug for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Block")
            .field("prev", &self.prev.iter().map(|p| Rc::as_ptr(p)).collect::<Vec<_>>())
            .field("instructions", &self.instructions)
            .field("succ", &self.succ.iter().map(|s| Rc::as_ptr(s)).collect::<Vec<_>>())
            .finish()
    }
}

impl BasicBlock {
    pub fn new(offset: usize, exc_entry: bool) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { offset, exc_entry, prev: vec![], instructions: vec![], succ: vec![], visited: false }))
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn exc_entry(&self) -> bool {
        self.exc_entry
    }

    pub fn prev(&self) -> &Vec<Rc<RefCell<BasicBlock>>> {
        &self.prev
    }

    pub fn succ(&self) -> &Vec<Rc<RefCell<BasicBlock>>> {
        &self.succ
    }

    pub fn instructions(&self) -> &Vec<Rc<Instruction>> {
        &self.instructions
    }

    pub fn add_prev(&mut self, block: Rc<RefCell<BasicBlock>>) {
        self.prev.push(block);
    }

    pub fn add_succ(&mut self, block: Rc<RefCell<BasicBlock>>) {
        self.succ.push(block);
    }
    
    pub fn push(&mut self, instruction: Rc<Instruction>) {
        self.instructions.push(instruction);
    }

    pub fn visit(&mut self) {
        self.visited = true;

        // println!("        {}: {:?}", self.offset, self.instructions());
        println!("        {}: {:?}", self.offset, BasicBlockFeatures::from(&self));
        for succ in self.succ.iter() {
            if let Ok(mut borrowed) = succ.try_borrow_mut() {
                if !borrowed.visited {
                    borrowed.visit();
                }
            }
        }
    }

}



#[derive(Debug)]
pub struct BlockContainer {
    pub blocks: Vec<Rc<RefCell<BasicBlock>>>,
    pub offsets: HashSet<usize>
}

impl BlockContainer {
    pub fn new() -> Self {
        Self { blocks: vec![], offsets: HashSet::new() }
    }

    fn push(&mut self, block: Rc<RefCell<BasicBlock>>) -> Rc<RefCell<BasicBlock>> {
        self.blocks.push(block.clone());
        block
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn get_block_at_offset(&mut self, offset: usize, raw_bytecode: &[u16]) -> Rc<RefCell<BasicBlock>> {
        for block in self.blocks.iter() {
            if let Ok(borrowed) = block.try_borrow() {
                if borrowed.offset == offset {
                    return block.clone();
                }
            }
        }
        self.offsets.insert(offset);
        self.push(BasicBlock::new(offset, raw_bytecode[offset] == 0x0D))
    }
}

#[derive(Debug)]
enum Terminator {
    None,
    Goto,
    Return,
    Throw
}

#[derive(Debug)]
pub struct BasicBlockFeatures {
    inst_cnt: f32,
    mov: f32,
    monitor: f32,
    arr_op: f32,
    /// cmpkind
    cmp: f32,
    /// Static/Instance field access
    facc: f32,
    invoke: f32,
    // Bitwise operations
    bop: f32,
    terminator: Terminator,
    exc_entry: bool
}

impl Into<Vec<f32>> for BasicBlockFeatures {
    fn into(self) -> Vec<f32> {
        vec![
            self.inst_cnt,
            self.store,
            self.load,
            self.op,
            self.invoke,
            self.move_,
            self.const_,
            self.terminator as u8 as f32,
            self.exc_entry as u8 as f32
        ]
    }
}

impl From<&&mut BasicBlock> for BasicBlockFeatures {
    fn from(block: &&mut BasicBlock) -> Self {
        let mut op_freq = HashMap::new();
        let mut move_ = 0.;
        let mut const_ = 0.;
        let mut invoke = 0.;
        let mut op = 0.;
        let mut load = 0.;
        let mut store = 0.;
        for inst in block.instructions() {
            let opcode = inst.opcode();
            let freq = op_freq.entry(opcode).or_insert(0);
            *freq += 1;
            match *opcode as u8 {
                0x01..=0x13 => move_ += 1.,
                0x12..=0x1C => const_ += 1.,
                0x6E..=0x78 | 0xFA..=0xFD => invoke += 1.,
                0x90..=0xE2 | 0x7B..=0x8F => op += 1.,
                0x44..=0x4A | 0x52..=0x58 | 0x60..=0x66 => load += 1.,
                0x4B..=0x51 | 0x59..=0x5F | 0x67..=0x6D => store += 1.,
                _ => ()
            }
        }
        let terminator = match *block.instructions().last().unwrap().opcode() as u8 {
            0x0E..=0x11 => Terminator::Return,
            0x27 => Terminator::Throw,
            0x28..=0x2C => Terminator::Goto,
            _ => Terminator::None
        };
        let length = block.instructions().len() as f32;
        BasicBlockFeatures { 
            inst_cnt: length, 
            store: (store / length) as f32,
            load: (load / length) as f32,
            op: (op / length) as f32,
            invoke: (invoke / length) as f32,
            move_: (move_ / length) as f32,
            const_: (const_ / length) as f32,
            terminator,
            exc_entry: block.exc_entry() 
        }
    }
}
