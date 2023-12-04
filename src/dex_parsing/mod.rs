use dex::Dex;
mod instruction;
mod opcode;

use self::instruction::Instruction;

pub(crate) fn parse_dexes(dexes: Vec<Dex<impl AsRef<[u8]>>>, sequence_cap: usize) -> (Vec<u8>, Vec<(usize, usize)>) {
    let mut op_seq = vec![]; 
    let mut method_bounds = vec![];
    let mut pos = 0;
    for dex in dexes {
        let (curr_op_seq, curr_method_bounds) = get_op_seq(dex, &mut pos);
        op_seq.extend(curr_op_seq);
        method_bounds.extend(curr_method_bounds);
    }
    if sequence_cap > 0 {
        method_bounds.retain(|(start, end)| *start < sequence_cap && *end < sequence_cap);
        if let Some(last) = method_bounds.last() {
            let cap = last.1;
            op_seq.truncate(cap + 1);
        }
    }
    (op_seq, method_bounds)
}


fn get_op_seq(dex: Dex<impl AsRef<[u8]>>, pos: &mut usize) -> (Vec<u8>, Vec<(usize, usize)>) {
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
                    if do_extend && !current_method_seq.is_empty() {
                        *pos += current_method_seq.len();
                        m_bounds.push((start, *pos - 1));
                        op_seq.extend(current_method_seq);
                    }
                }
            }
        }
    }
    (op_seq, m_bounds)
}
