mod opcode;
mod instruction;
mod reference;
mod block;
mod graph;

use dex::DexReader;
use instruction::Instruction;


fn main() {
    let dex = DexReader::from_file("resources/classes2.dex").unwrap();
    let cfg = graph::ControlFlowGraph::from_dex(dex);
}
