use std::{rc::Rc, cell::RefCell, collections::{HashMap, HashSet}};

use dex::Dex;
use petgraph::prelude::DiGraph;
use crate::{instruction::Instruction, block::{BasicBlock, BlockContainer}};


#[derive(Debug)]
pub struct ControlFlowGraph {
    graph: DiGraph<BasicBlock, ()>
}

impl ControlFlowGraph {
    pub(crate) fn from_dex(dex: Dex<impl AsRef<[u8]>>) -> Self {
        for class in dex.classes() {
            if let Ok(class) = class {
                for method in class.methods() {
                    println!("{}", class.jtype().type_descriptor().to_string() + method.name());
                    if class.jtype().type_descriptor().to_string() + method.name() == "Lorg/fdroid/fdroid/views/main/MainActivity;onStart" {
                        println!("break here");
                    }
                    if let Some(code) = method.code() {
                        let raw_bytecode = code.insns();
                        let blocks = Self::get_blocks(&dex, raw_bytecode);
                        for (i, block) in blocks.iter().enumerate() {
                            println!("\n  Block #{}", i);
                            for inst in block.borrow().instructions() {
                                println!("    {:?}", inst);
                            }
                        }
                    }
                }
            }
        }
        todo!()
    }

    fn get_blocks(dex: &Dex<impl AsRef<[u8]>>, raw_bytecode: &[u16]) -> Vec<Rc<RefCell<BasicBlock>>> {
        let mut block_container = BlockContainer::new();
        let mut offsets = vec![0];
        let mut block = block_container.get_block_at_offset(0);
        while !offsets.is_empty() {
            let offset = offsets.pop().unwrap();
            if block_container.offsets.contains(&offset) {
                block = block_container.get_block_at_offset(offset);
            }
            let binding = block.clone();
            let mut borrowed_block = binding.borrow_mut();
            if offset >= raw_bytecode.len() {
                break;
            }
            if let Some(inst) = Instruction::try_from_raw_bytecode(&raw_bytecode, offset, &dex).unwrap() {
                let new_offset = offset + inst.length as usize;
                offsets.push(new_offset);
                borrowed_block.push(inst);
                let inst = borrowed_block.instructions().last().unwrap();
                if inst.is_terminator() {
                    if let Some(jump_target) = inst.jump_target() {
                        let successor = block_container.get_block_at_offset(jump_target);
                        borrowed_block.add_succ(successor.clone());
                        successor.borrow_mut().add_prev(block.clone());
                    }
                    block = block_container.get_block_at_offset(new_offset);
                }
            }
        }
        let BlockContainer { blocks, offsets: _ } = block_container;
        blocks
    }
}
