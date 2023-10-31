use dex::{code::CodeItem, Dex};
use petgraph::prelude::DiGraph;

use crate::instruction::Instruction;


#[derive(Debug)]
pub struct ControlFlowGraph {
    graph: DiGraph<Block, ()>
}

impl ControlFlowGraph {
    pub(crate) fn from_dex(dex: Dex<impl AsRef<[u8]>>) -> Self {
        for class in dex.classes() {
            if let Ok(class) = class {
                for method in class.methods() {
                    println!("{:?}.{}", class.jtype().type_descriptor(), method.name());
                    if let Some(code) = method.code() {
                        let mut offset = 0;
                        let mut instructions = vec![];
                        let raw_bytecode = code.insns();
                        while offset < raw_bytecode.len() {
                            if let Some((inst, length)) = Instruction::try_from_raw_bytecode(&raw_bytecode, offset, &dex).unwrap() {
                                offset += length as usize;
                                println!("    {:?}", inst);
                                instructions.push(inst);
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
        todo!()
    }
}


#[derive(Debug)]
struct Block {
    id: usize,
    instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
struct BlockParsingError;


impl Block {}