use std::{rc::Rc, cell::RefCell, collections::{HashSet, HashMap}, vec};

use dex::Dex;
use crate::{instruction::Instruction, block::BasicBlock, opcode::Opcode, concat_words};

#[derive(Debug)]
pub(crate) struct DexMethod {
    name: String,
    entry: Rc<RefCell<BasicBlock>>
}

impl DexMethod {
    pub fn name(&self) -> &str {
        &self.name
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
                let cname = class.jtype().to_string();
                let mut methods = Vec::new();
                for method in class.methods() {
                    if let Some(code) = method.code() {
                        let mname = method.name().to_string();
                        let concat = format!("{}::{}", cname, mname);
                        println!("{}", concat);
                        if concat == "Lorg/fdroid/fdroid/views/main/LatestAdapter;::onCreateViewHolder" {
                            println!("here");
                        }
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
        let mut instructions: Vec<Instruction> = vec![];
        let mut block_starts = vec![0 as usize];
        let mut edges = vec![];
        let mut offset = 0;
        while offset < raw_bytecode.len() {
            if let Some(inst) = Instruction::try_from_raw_bytecode(raw_bytecode, offset, dex).unwrap() {
                offset += *inst.length() as usize;
                match *inst.opcode() as u8 {
                    0x32..=0x3D => {
                        let current_block_start = *block_starts.last().unwrap();
                        edges.push((current_block_start, offset));
                        edges.push((current_block_start, inst.jump_target().unwrap()));
                        block_starts.push(offset);
                        block_starts.push(inst.jump_target().unwrap());
                        
                    },
                    0x28..=0x2A => {
                        let current_block_start = *block_starts.last().unwrap();
                        edges.push((current_block_start, inst.jump_target().unwrap()));
                        block_starts.push(inst.jump_target().unwrap());
                    },
                    0x2B => {
                        let jump_target = inst.jump_target().unwrap();
                        let size = raw_bytecode[jump_target + 1];
                        let current_offset = *inst.offset();
                        let current_block_start = *block_starts.last().unwrap();
                        let targets = &raw_bytecode[jump_target + 4..];
                        for i in (0..(size as usize * 2)).step_by(2) {
                            let relative_target = concat_words!(targets[i], targets[i+1]) as i32;
                            let target = (current_offset as i32 + relative_target) as u32;
                            block_starts.push(target as usize);
                            edges.push((current_block_start, target as usize));
                        }
                    },
                    0x2C => {
                        println!("{}", inst.jump_target().unwrap());
                    },
                    _ => ()
                }
                instructions.push(inst);
            } else {
                break;
            }
        }
        let block_starts = block_starts.into_iter().collect::<HashSet<usize>>();
        let mut blocks = vec![];
        let mut index_mapping = HashMap::new();
        for inst in instructions.into_iter() {
            if block_starts.contains(inst.offset()) {
                blocks.push(BasicBlock::new(*inst.offset()));
                index_mapping.insert(*inst.offset(), blocks.len() - 1);
            }
            let mut current_block = blocks.last().expect("No current block").borrow_mut();
            current_block.push(inst);
        }
        for (src, dst) in edges.into_iter() {
            let src_index = index_mapping.get(&src).expect("No source index");
            let dst_index = index_mapping.get(&dst).expect("No destination index");
            let src_block = blocks.get(*src_index).unwrap().clone();
            let dst_block = blocks.get(*dst_index).unwrap().clone();
            src_block.borrow_mut().add_succ(dst_block.clone());
            dst_block.borrow_mut().add_prev(src_block.clone());
        }
        blocks
    }
}


#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::{opcode::Opcode, block::BasicBlock};

    use super::DexControlFlowGraph;

    fn assert_block_starts(opcodes: &[Opcode], blocks: &[Rc<RefCell<BasicBlock>>]) {
        for (opcode, block) in opcodes.iter().zip(blocks.iter()) {
            let borrowed = block.borrow();
            assert_eq!(*opcode, *borrowed.instructions().first().unwrap().opcode());
        }
    }

    #[test]
    fn test_get_blocks0() {
        // Lorg/fdroid/fdroid/views/main/MainActivity;onStart
        let raw_bytecode = [4207, 743, 2, 96, 57, 275, 33, 4148, 15, 26, 21033, 8305, 855, 2, 266, 312, 7, 8532, 22998, 8302, 714, 1, 14];
        let dex = dex::DexReader::from_file("tests/test.dex").unwrap();
        let blocks = DexControlFlowGraph::get_blocks(&dex, &raw_bytecode);
        assert_eq!(4, blocks.len());
        assert_block_starts(
            &[Opcode::InvokeSuper, Opcode::ConstString, Opcode::IgetObject, Opcode::ReturnVoid], 
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
                Opcode::IgetObject, Opcode::Const4, Opcode::InvokeVirtual, Opcode::InvokeVirtual,
                Opcode::NewInstance, Opcode::ReturnObject, Opcode::MoveException,
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
                Opcode::IgetObject, Opcode::Const4, Opcode::InvokeVirtual, 
                Opcode::AddInt2Addr, Opcode::Const4, Opcode::AddInt2Addr], 
            &blocks
        );
    }
}