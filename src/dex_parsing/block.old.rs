// use std::{rc::Rc, cell::RefCell, fmt, sync::{Mutex, Arc}, collections::HashSet};

// use super::instruction::Instruction;

// pub(crate) struct BasicBlock {
//     prev: Vec<Rc<RefCell<BasicBlock>>>,
//     instructions: Vec<Instruction>,
//     succ: Vec<Rc<RefCell<BasicBlock>>>,
//     visited: bool,
// }

// impl fmt::Debug for BasicBlock {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_struct("Block")
//         .field("prev", &self.prev.iter().map(|p| Rc::as_ptr(p)).collect::<Vec<_>>())
//         .field("instructions", &self.instructions)
//         .field("succ", &self.succ.iter().map(|s| Rc::as_ptr(s)).collect::<Vec<_>>())
//         .finish()
//     }
// }

// type BlockPtr = Rc<RefCell<BasicBlock>>;

// impl BasicBlock {

//     pub fn new() -> BlockPtr {
//         Rc::new(RefCell::new(Self { prev: vec![], instructions: vec![], succ: vec![], visited: false }))
//     }

//     #[allow(dead_code)]
//     pub fn instructions(&self) -> &Vec<Instruction> {
//         &self.instructions
//     }

//     pub fn add_prev(&mut self, block: BlockPtr) {
//         self.prev.push(block);
//     }

//     pub fn add_succ(&mut self, block: BlockPtr) {
//         self.succ.push(block);
//     }

//     pub fn push(&mut self, instruction: Instruction) {
//         self.instructions.push(instruction);
//     }

//     pub fn visit(&mut self, accumulator: &Arc<Mutex<HashSet<String>>>) {
//         self.visited = true;
//         {
//             let mut acc = accumulator.lock().unwrap();
//             acc.insert(self.instructions.iter().map(|i| format!("{}", *i.opcode() as u8)).collect::<Vec<_>>().join(" "));
//         }
//         for succ in self.succ.iter() {
//             if let Ok(mut succ) = succ.try_borrow_mut() {
//                 if !succ.visited {
//                     succ.visit(accumulator);
//                 }
//             }
//         }
//     }
// }