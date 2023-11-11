use std::{rc::Rc, cell::RefCell, fmt, collections::{HashSet, HashMap}};

use crate::instruction::Instruction;

pub struct BasicBlock {
    offset: usize,
    exc_entry: bool,
    prev: Vec<Rc<RefCell<BasicBlock>>>,
    instructions: Vec<Instruction>,
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

    pub fn instructions(&self) -> &Vec<Instruction> {
        &self.instructions
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
}
