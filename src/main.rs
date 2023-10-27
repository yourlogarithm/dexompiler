mod opcode;
mod instruction;
mod reference;

use dex::DexReader;
use instruction::Instruction;


fn main() {
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
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
                            println!("    {:?}", inst);
                            offset += length as usize;
                            instructions.push(inst);
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
}
