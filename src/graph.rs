use std::{rc::Rc, cell::RefCell, collections::HashMap};

use dex::Dex;
use petgraph::prelude::DiGraph;
use crate::{instruction::Instruction, block::{BasicBlock, BlockContainer, BlockType}};


#[derive(Debug)]
pub struct ControlFlowGraph {
    graph: DiGraph<BasicBlock, ()>
}

impl ControlFlowGraph {
    pub(crate) fn from_dex(dex: Dex<impl AsRef<[u8]>>) -> Self {
        let class_count = dex.classes().count();
        for (class_idx, class) in dex.classes().enumerate() {
            if let Ok(class) = class {
                let method_count = class.methods().count();
                for (method_idx, method) in class.methods().enumerate() {
                    println!("{}", class.jtype().type_descriptor().to_string() + method.name());
                    if ("Lorg/bouncycastle/crypto/util/PrivateKeyInfoFactory;createPrivateKeyInfo" == class.jtype().type_descriptor().to_string() + method.name()) {
                        println!("");
                    }
                    if let Some(code) = method.code() {
                        let raw_bytecode = code.insns();
                        let blocks = Self::get_blocks(&dex, raw_bytecode);
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
        let binding = block.clone();
        let mut borrowed_block = binding.borrow_mut();
        while !offsets.is_empty() {
            let offset = offsets.pop().unwrap();
            if offset >= raw_bytecode.len() {
                break;
            }
            if let Some(inst) = Instruction::try_from_raw_bytecode(&raw_bytecode, offset, &dex).unwrap() {
                offsets.push(offset + inst.length as usize);
                borrowed_block.push(inst);
                let inst = borrowed_block.instructions().last().unwrap();
                if inst.is_terminator() {
                    if let Some(jump_target) = inst.jump_target() {
                        let successor = block_container.get_block_at_offset(jump_target);
                        borrowed_block.add_succ(successor.clone());
                        successor.borrow_mut().add_prev(block.clone());
                        offsets.push(jump_target);
                    }
                    block = block_container.get_block_at_offset(offsets.last().unwrap().clone());
                }
            }
        }
        let BlockContainer { blocks } = block_container;
        blocks
    }
}
