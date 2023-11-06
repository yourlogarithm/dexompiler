use std::{rc::Rc, cell::RefCell, fmt, collections::{HashMap, HashSet}};

use crate::instruction::Instruction;

pub struct BasicBlock {
    pub offset: usize,
    prev: Vec<Rc<RefCell<BasicBlock>>>,
    instructions: Vec<Instruction>,
    succ: Vec<Rc<RefCell<BasicBlock>>>,
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
    pub fn new(offset: usize) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { offset, prev: vec![], instructions: vec![], succ: vec![] }))
    }

    pub fn prev(&self) -> &[Rc<RefCell<BasicBlock>>] {
        &self.prev
    }

    pub fn succ(&self) -> &[Rc<RefCell<BasicBlock>>] {
        &self.succ
    }

    pub fn add_prev(&mut self, block: Rc<RefCell<BasicBlock>>) {
        self.prev.push(block);
    }

    pub fn add_succ(&mut self, block: Rc<RefCell<BasicBlock>>) {
        self.succ.push(block);
    }
    
    pub fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
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

    pub fn get_block_at_offset(&mut self, offset: usize) -> Rc<RefCell<BasicBlock>> {
        for block in self.blocks.iter() {
            if let Ok(borrowed) = block.try_borrow() {
                if borrowed.offset == offset {
                    return block.clone();
                }
            }
        }
        self.offsets.insert(offset);
        self.push(BasicBlock::new(offset))
    }
}