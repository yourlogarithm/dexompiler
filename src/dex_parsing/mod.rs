use std::collections::{HashSet, HashMap};

use dex::Dex;
mod instruction;
mod opcode;
mod block;
use crate::concat_words;

use self::{instruction::Instruction, block::{BlockPtr, BasicBlock}, opcode::Opcode};

pub(crate) fn parse_dexes(dexes: Vec<Dex<impl AsRef<[u8]>>>, sequence_cap: usize) -> (Vec<u8>, Vec<(usize, usize)>) {
    let mut op_seq = vec![]; 
    let mut method_bounds = vec![];
    let mut pos = 0;
    for dex in dexes {
        let (curr_op_seq, curr_method_bounds) = get_op_seq(dex, &mut pos, sequence_cap);
        op_seq.extend(curr_op_seq);
        method_bounds.extend(curr_method_bounds);
    }
    (op_seq, method_bounds)
}


fn get_op_seq(dex: Dex<impl AsRef<[u8]>>, pos: &mut usize, sequence_cap: usize) -> (Vec<u8>, Vec<(usize, usize)>) {
    let mut op_seq = vec![];
    let mut m_bounds = vec![];
    for class in dex.classes() {
        if let Ok(class) = class {
            for method in class.methods() {
                if let Some(code) = method.code() {
                    let raw_bytecode = code.insns();
                    let mut offset = 0;
                    let mut current_method_seq = vec![];
                    let mut do_extend = true;
                    let start = *pos;
                    while offset < raw_bytecode.len() {
                        match Instruction::try_from_raw_bytecode(raw_bytecode, offset) {
                            Ok(Some((inst, length))) => {
                                offset += length;
                                current_method_seq.push(*inst.opcode() as u8);
                            },
                            Ok(None) => break,
                            Err(_) => {
                                // eprintln!("Error parsing: {}::{}", class.jtype().to_java_type(), method.name());
                                do_extend = false;
                                break;
                            },
                        }
                    }
                    if do_extend && (sequence_cap == 0 || op_seq.len() + current_method_seq.len() < sequence_cap) {
                        extend(&mut op_seq, current_method_seq, &mut m_bounds, pos, start)
                    }
                }
            }
        }
    }
    (op_seq, m_bounds)
}

fn extend(op_seq: &mut Vec<u8>, current_method_seq: Vec<u8>, m_bounds: &mut Vec<(usize, usize)>, pos: &mut usize, start: usize) {
    *pos += current_method_seq.len();
    m_bounds.push((start, *pos - 1));
    op_seq.extend(current_method_seq);
}

pub(crate) fn into_blocks(dex: Dex<impl AsRef<[u8]>>) -> Vec<BlockPtr> {
    let mut blocks = vec![];
    for class in dex.classes() {
        if let Ok(class) = class {
            for method in class.methods() {
                if let Some(code) = method.code() {
                    if let Ok(b) = get_blocks(code.insns()) {
                        if let Some(block) = b.first() {
                            blocks.push(block.clone());
                        }
                    } else {
                        eprintln!("Error parsing: {}::{}", class.jtype().to_java_type(), method.name());
                    }
                }
            }
        }
    }
    blocks
}

fn get_blocks(raw_bytecode: &[u16]) -> Result<Vec<BlockPtr>, String> {
    let mut instructions: Vec<Instruction> = vec![];
    let mut block_starts = vec![0 as usize];
    let mut edges = vec![];
    let mut offset = 0;
    while offset < raw_bytecode.len() {
        match Instruction::try_from_raw_bytecode(raw_bytecode, offset) {
            Ok(Some((inst, length))) => {
                offset += length;
                match *inst.opcode() as u8 {
                    0x32..=0x3D => {
                        let current_block_start = *block_starts.last().unwrap();
                        edges.push((current_block_start, offset));
                        let jump_target = *inst.dest().as_ref().unwrap().as_branch_target().unwrap();
                        edges.push((current_block_start, jump_target));
                        block_starts.push(offset);
                        block_starts.push(jump_target);
                        
                    },
                    0x28..=0x2A => {
                        let current_block_start = *block_starts.last().unwrap();
                        let jump_target = *inst.dest().as_ref().unwrap().as_branch_target().unwrap();
                        edges.push((current_block_start, jump_target));
                        block_starts.push(jump_target);
                    },
                    0x2B | 0x2C => {
                        let jump_target = *inst.dest().as_ref().unwrap().as_branch_target().unwrap();
                        if jump_target + 1 > raw_bytecode.len() {
                            return Err(format!("Jump target out of bounds: {}", jump_target).to_string());
                        }
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
            },
            Ok(None) => break,
            Err(_) => return Err(format!("Error parsing instruction at offset: {}", offset).to_string()),
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
        let src_index = match index_mapping.get(&src) {
            Some(&index) => index,
            None => return Err(format!("No source index {}", src).to_string()),
        };
        let dst_index = match index_mapping.get(&dst) {
            Some(&index) => index,
            None => return Err(format!("No destination index {}", dst).to_string()),
        };
        let src_block = blocks.get(src_index).unwrap().clone();
        let dst_block = blocks.get(dst_index).unwrap().clone();
        src_block.borrow_mut().add_succ(dst_block.clone());
        dst_block.borrow_mut().add_prev(src_block.clone());
    }
    Ok(blocks)
}


#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};
    use super::get_blocks;
    use super::{opcode::Opcode, block::BasicBlock};

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
        let blocks = get_blocks(&raw_bytecode).unwrap();
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
        let blocks = get_blocks(&raw_bytecode).unwrap();
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
        let blocks = get_blocks(&raw_bytecode).unwrap();
        assert_block_starts(
            &[
                Opcode::IgetObject, Opcode::Const4, Opcode::InvokeVirtual, 
                Opcode::AddInt2Addr, Opcode::Const4, Opcode::AddInt2Addr], 
            &blocks
        );
    }
}