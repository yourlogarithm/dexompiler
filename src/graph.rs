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
                    // if class.jtype().type_descriptor().to_string() + method.name() == "Lorg/fdroid/fdroid/views/main/MainActivity;onStart" {
                    //     println!("break here");
                    // }
                    if let Some(code) = method.code() {
                        let raw_bytecode = code.insns();
                        println!("  {:?}", raw_bytecode);
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
                    if new_offset < raw_bytecode.len() && raw_bytecode[new_offset] != 0 {
                        let new_block = block_container.get_block_at_offset(new_offset);
                        borrowed_block.add_succ(new_block.clone());
                        new_block.borrow_mut().add_prev(block.clone());
                        block = new_block;
                    }
                }
            }
        }
        let BlockContainer { blocks, offsets: _ } = block_container;
        blocks
    }
}


#[cfg(test)]
mod test {
    use std::{collections::HashSet, cell::RefCell, rc::Rc};

    use crate::{opcode::Opcode, block::BasicBlock};

    use super::ControlFlowGraph;

    fn assert_block_starts(opcodes: &[Opcode], blocks: &[Rc<RefCell<BasicBlock>>]) {
        let block_starts: HashSet<Opcode> = blocks.iter().map(|block| *block.borrow().instructions()[0].opcode()).collect();
        let expected_block_starts: HashSet<Opcode> = opcodes.iter().cloned().collect();
        assert_eq!(expected_block_starts, block_starts);
    }

    #[test]
    fn test_get_blocks0() {
        // Lorg/fdroid/fdroid/views/main/MainActivity;onStart
        let raw_bytecode = [4207, 743, 2, 96, 57, 275, 33, 4148, 15, 26, 21033, 8305, 855, 2, 266, 312, 7, 8532, 22998, 8302, 714, 1, 14];
        let dex = dex::DexReader::from_file("tests/test.dex").unwrap();
        let blocks = ControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_eq!(4, blocks.len());
        assert_block_starts(
            &[Opcode::InvokeSuper, Opcode::ReturnVoid, Opcode::ConstString, Opcode::IgetObject], 
            &blocks
        );
    }

    #[test]
    fn test_get_blocks1() {
        // Lorg/bouncycastle/dvcs/DVCSRequestInfo;getRequestTime
        let raw_bytecode = [
            16468, 2726, 4206, 3408, 0, 12, 57, 4, 18, 17, 
            4206, 3424, 0, 268, 312, 11, 4206, 3424, 0, 12, 
            4206, 3167, 0, 12, 17, 290, 5100, 4206, 3425, 0, 
            12, 8304, 23643, 1, 4206, 23653, 1, 12, 4206, 
            23682,  0, 12, 17, 13, 290, 2002, 546, 643, 4208, 
            1799, 2, 794, 37138, 8302,  1810, 50, 4206, 1847, 
            0, 780, 8302, 1810, 50, 4206, 1818, 2, 524, 12400, 
            7759, 33, 295
        ];
        let dex = dex::DexReader::from_file("tests/test.dex").unwrap();
        let blocks = ControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_eq!(6, blocks.len());
        assert_block_starts(
            &[Opcode::IgetObject, Opcode::InvokeVirtual, Opcode::Const4, Opcode::NewInstance, Opcode::InvokeVirtual, Opcode::MoveException], 
            &blocks
        );
    }

    #[test]
    fn test_get_blocks2() {
        // Lorg/fdroid/download/Mirror;hashCode
        let raw_bytecode = [
            8276, 11170, 4206, 1757, 0, 10, 218, 7936, 8532, 11172, 
            313, 4, 274, 1320, 4206, 1757, 1, 266, 4272, 218, 7936, 
            8533, 11171, 312, 3, 4370, 4272, 15
        ];
        let dex = dex::DexReader::from_file("tests/test.dex").unwrap();
        let blocks = ControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_eq!(6, blocks.len());
        assert_block_starts(
            &[Opcode::IgetObject, Opcode::InvokeVirtual, Opcode::Const4, Opcode::AddInt2Addr, Opcode::AddInt2Addr, Opcode::Const4], 
            &blocks
        );
    }
}