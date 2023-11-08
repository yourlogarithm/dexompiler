use std::{rc::Rc, cell::RefCell, collections::HashSet};

use dex::Dex;
use crate::{instruction::Instruction, block::{BasicBlock, BlockContainer}};

#[derive(Debug)]
pub(crate) struct DexMethod {
    name: String,
    entry: Rc<RefCell<BasicBlock>>
}

impl DexMethod {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn visit(&self) {
        self.entry.borrow_mut().visit();
    }
}


#[derive(Debug)]
pub(crate) struct DexClass {
    jtype: String,
    methods: Vec<DexMethod>,
}

impl DexClass {
    pub fn jtype(&self) -> &str {
        &self.jtype
    }

    pub fn methods(&self) -> &[DexMethod] {
        &self.methods
    }
}


#[derive(Debug)]
pub struct DexControlFlowGraph {
    classes: Vec<DexClass>,
}

impl DexControlFlowGraph {
    pub fn classes(&self) -> &[DexClass] {
        &self.classes
    }
}

impl DexControlFlowGraph {
    pub(crate) fn from_dex(dex: Dex<impl AsRef<[u8]>>) -> Self {
        let mut classes = Vec::new();
        for class in dex.classes() {
            if let Ok(class) = class {
                let mut methods = Vec::new();
                for method in class.methods() {
                    // if class.jtype().to_string() + method.name().to_string().as_str() == "Lorg/bouncycastle/crypto/engines/EthereumIESEngine;decryptBlock" {
                    //     println!("Found it!");
                    // }
                    if let Some(code) = method.code() {
                        let raw_bytecode = code.insns();
                        let blocks = Self::get_blocks(&dex, raw_bytecode);
                        let entry = blocks.first().unwrap().clone();
                        methods.push(DexMethod { name: method.name().to_string(), entry: entry });
                    }
                }
                classes.push(DexClass { jtype: class.jtype().to_string(), methods: methods });
            }
        }
        DexControlFlowGraph { classes }
    }

    fn get_blocks(dex: &Dex<impl AsRef<[u8]>>, raw_bytecode: &[u16]) -> Vec<Rc<RefCell<BasicBlock>>> {
        let mut block_container = BlockContainer::new();
        let mut offsets = vec![0];
        let mut block = block_container.get_block_at_offset(0, raw_bytecode);
        while !offsets.is_empty() {
            let offset = offsets.pop().unwrap();
            if offset >= raw_bytecode.len() {
                break;
            }
            if block_container.offsets.contains(&offset) {
                block = block_container.get_block_at_offset(offset, raw_bytecode);
            }
            let binding = block.clone();
            let mut borrowed_block = binding.borrow_mut();
            if let Some(inst) = Instruction::try_from_raw_bytecode(&raw_bytecode, offset, &dex).unwrap() {
                let new_offset = offset + inst.length as usize;
                let inst = Rc::new(inst);
                match inst.opcode as u8 {
                    0x32..=0x3D => {
                        let jump_target = inst.jump_target().unwrap();

                        let successor = block_container.get_block_at_offset(jump_target, raw_bytecode);
                        borrowed_block.add_succ(successor.clone());
                        successor.borrow_mut().add_prev(block.clone());

                        let new_block = block_container.get_block_at_offset(offset, raw_bytecode);  // the if is the start of a new block
                        borrowed_block.add_succ(new_block.clone());
                        let mut new_borrowed = new_block.borrow_mut();
                        new_borrowed.add_prev(block.clone());

                        new_borrowed.push(inst.clone());

                        offsets.push(new_offset);
                    },
                    0x28..=0x2A => {
                        borrowed_block.push(inst.clone());
                        let jump_target = inst.jump_target().unwrap();

                        let successor = block_container.get_block_at_offset(jump_target, raw_bytecode);
                        borrowed_block.add_succ(successor.clone());
                        successor.borrow_mut().add_prev(block.clone());

                        offsets.push(new_offset);
                    },
                    0x0E..=0x11 | 0x27 => {
                        borrowed_block.push(inst.clone());
                        if new_offset < raw_bytecode.len() && raw_bytecode[new_offset] != 0 {
                            block_container.get_block_at_offset(new_offset, raw_bytecode);
                            offsets.push(new_offset);
                        }
                    },
                    _ => {
                        borrowed_block.push(inst.clone());
                        if block_container.offsets.contains(&new_offset) {
                            let new_block = block_container.get_block_at_offset(new_offset, raw_bytecode);
                            borrowed_block.add_succ(new_block.clone());
                            new_block.borrow_mut().add_prev(block.clone());
                        }
                        offsets.push(new_offset);
                    }
                }
            }
        }
        let BlockContainer { blocks, offsets: _ } = block_container;
        // for block in blocks.iter() {
        //     let borrowed = block.borrow();
        //     if borrowed.instructions().is_empty() {
        //         println!("{}", borrowed.offset());
        //         for prev in borrowed.prev() {
        //             let prev_borrow = prev.borrow();
        //             println!("    {:?}", prev_borrow.offset());
        //             for inst in prev_borrow.instructions() {
        //                 println!("    {:?}", inst);
        //             }
        //         }
        //         println!("");
        //     }
        // }
        blocks
    }
}


#[cfg(test)]
mod test {
    use std::{collections::{HashSet, HashMap}, cell::RefCell, rc::Rc};

    use crate::{opcode::Opcode, block::BasicBlock};

    use super::DexControlFlowGraph;

    fn assert_block_starts(opcodes: &[Opcode], blocks: &[Rc<RefCell<BasicBlock>>]) {
        let mut block_starts: HashMap<Opcode, usize> = HashMap::new();
        for block in blocks.iter() {
            let borrowed = block.borrow();
            let opcode = borrowed.instructions().first().unwrap().opcode;
            let entry = block_starts.entry(opcode).or_insert(0);
            *entry += 1;
        }
        let mut expected_block_starts = HashMap::new();
        for opcode in opcodes.iter() {
            let entry = expected_block_starts.entry(*opcode).or_insert(0);
            *entry += 1;
        }
        assert_eq!(expected_block_starts, block_starts);
    }

    #[test]
    fn test_get_blocks0() {
        // Lorg/fdroid/fdroid/views/main/MainActivity;onStart
        let raw_bytecode = [4207, 743, 2, 96, 57, 275, 33, 4148, 15, 26, 21033, 8305, 855, 2, 266, 312, 7, 8532, 22998, 8302, 714, 1, 14];
        let dex = dex::DexReader::from_file("tests/test.dex").unwrap();
        let blocks = DexControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_eq!(4, blocks.len());
        assert_block_starts(
            &[Opcode::InvokeSuper, Opcode::IfLt, Opcode::IfEqz, Opcode::ReturnVoid], 
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
        let blocks = DexControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_block_starts(
            &[
                Opcode::IgetObject, Opcode::IfNez, Opcode::InvokeVirtual, 
                Opcode::IfEqz, Opcode::NewInstance, Opcode::MoveException
            ], 
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
        let blocks = DexControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_block_starts(
            &[
                Opcode::IgetObject, Opcode::IfNez, Opcode::InvokeVirtual, 
                Opcode::AddInt2Addr, Opcode::IfEqz, Opcode::AddInt2Addr], 
            &blocks
        );
    }

    #[test]
    fn test_entry_block() {
        let dex = dex::DexReader::from_file("tests/test.dex").unwrap();
        let cfg = DexControlFlowGraph::from_dex(dex);
        for class in cfg.classes {
            for method in class.methods {
                let borrowed = method.entry.borrow();
                assert_eq!(borrowed.prev().len(), 0);
            }
        }
    }
}