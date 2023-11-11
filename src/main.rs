mod opcode;
mod instruction;
mod reference;
mod block;
mod graph;

use dex::DexReader;


fn main() {
    let dex = DexReader::from_file("resources/classes2.dex").unwrap();
    let cfg = graph::DexControlFlowGraph::from_dex(dex);
    for cls in cfg.classes() {
        println!("{}", cls.jtype());
        for method in cls.methods() {
            println!("  {}", method.name());
        }
    }
}
