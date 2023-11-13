mod opcode;
mod instruction;
mod block;
mod graph;

use std::{fs::OpenOptions, sync::{Mutex, Arc}, collections::HashSet, io::Write};

use rayon::prelude::{IntoParallelRefIterator, IndexedParallelIterator, ParallelIterator};
use dex::DexReader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let accumulator = Arc::new(Mutex::new(HashSet::new()));

    args.par_iter().skip(1).for_each(|path| {
        if let Ok(dex) = DexReader::from_file(path) {
            eprintln!("{}", path);
            let blocks = graph::into_blocks(dex);
            for block in blocks {
                let mut block = block.borrow_mut();
                block.visit(&accumulator);
            }
        } else {
            eprintln!("{}: failed to read dex file", path);
        }
    });

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("blocks.txt")
        .unwrap();
    let accumulator = accumulator.lock().unwrap();
    for sequence in accumulator.iter() {
        file.write_all(sequence.as_bytes()).unwrap();
        file.write_all(b"\n").unwrap();
    }
}
