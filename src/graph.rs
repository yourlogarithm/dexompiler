use std::{collections::{HashSet, HashMap}, vec};

use dex::Dex;
use crate::{instruction::Instruction, block::{BasicBlock, BlockPtr}, opcode::Opcode, concat_words};

pub(crate) fn into_blocks(dex: Dex<impl AsRef<[u8]>>) -> Vec<BlockPtr> {
    let mut blocks = vec![];
    for class in dex.classes() {
        if let Ok(class) = class {
            for method in class.methods() {
                if let Some(code) = method.code() {
                    let b = get_blocks(code.insns());
                    let entry = b.first().unwrap().clone();
                    blocks.push(entry);
                }
            }
        }
    }
    blocks
}

fn get_blocks(raw_bytecode: &[u16]) -> Vec<BlockPtr> {
    let mut instructions: Vec<Instruction> = vec![];
    let mut block_starts = vec![0 as usize];
    let mut edges = vec![];
    let mut offset = 0;
    while offset < raw_bytecode.len() {
        if let Some((inst, length)) = Instruction::try_from_raw_bytecode(raw_bytecode, offset).unwrap() {
            offset += length;
            match *inst.opcode() as u8 {
                0x32..=0x3D => {
                    let current_block_start = *block_starts.last().unwrap();
                    edges.push((current_block_start, offset));
                    edges.push((current_block_start, inst.branch_target().unwrap()));
                    block_starts.push(offset);
                    block_starts.push(inst.branch_target().unwrap());
                    
                },
                0x28..=0x2A => {
                    let current_block_start = *block_starts.last().unwrap();
                    edges.push((current_block_start, inst.branch_target().unwrap()));
                    block_starts.push(inst.branch_target().unwrap());
                },
                0x2B | 0x2C => {
                    let jump_target = inst.branch_target().unwrap();
                    let size = raw_bytecode[jump_target + 1];
                    let current_offset = *inst.offset();
                    let current_block_start = *block_starts.last().unwrap();
                    let targets = if inst.opcode() == &Opcode::PackedSwitch {
                        &raw_bytecode[jump_target + 4..]
                    } else {
                        &raw_bytecode[jump_target + 2 + size as usize * 2..]
                    };
                    for i in (0..(size as usize * 2)).step_by(2) {
                        let relative_target = concat_words!(targets[i], targets[i+1]) as i32;
                        let target = (current_offset as i32 + relative_target) as u32;
                        block_starts.push(target as usize);
                        edges.push((current_block_start, target as usize));
                    }
                },
                _ => ()
            }
            instructions.push(inst);
        } else {
            break;
        }
    }
    let block_starts = block_starts.into_iter().collect::<HashSet<usize>>();
    let mut blocks: Vec<BlockPtr> = vec![];
    let mut index_mapping = HashMap::new();
    for inst in instructions.into_iter() {
        if block_starts.contains(inst.offset()) {
            blocks.push(BasicBlock::new());
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


#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};
    use super::get_blocks;
    use crate::{opcode::Opcode, block::BasicBlock};

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
        let blocks = get_blocks(&raw_bytecode);
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
        let blocks = get_blocks(&raw_bytecode);
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
        let blocks = get_blocks(&raw_bytecode);
        assert_block_starts(
            &[
                Opcode::IgetObject, Opcode::Const4, Opcode::InvokeVirtual, 
                Opcode::AddInt2Addr, Opcode::Const4, Opcode::AddInt2Addr], 
            &blocks
        );
    }
}