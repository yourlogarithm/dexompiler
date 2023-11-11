use std::{rc::Rc, cell::RefCell, fmt};

use crate::instruction::Instruction;

pub struct BasicBlock {
    offset: usize,
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

    pub fn to_features(&self) -> Vec<f32> {
        
        todo!()
    }
}
