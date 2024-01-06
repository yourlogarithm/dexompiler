mod dex_parsing;
mod manifest_parsing;
mod cli;

use clap::Parser;
use manifest_parsing::parse_permissions;
use dex_parsing::Instruction;
use cli::Args;

use std::{fs::{OpenOptions, self}, sync::{Mutex, Arc}, collections::HashMap, io::Read, fmt, error::Error};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use dex::{DexReader, Dex};
use serde::{Serialize, Serializer};
use indicatif::ParallelProgressIterator;
use std::io::BufWriter;
use std::path::Path;
use zip::ZipArchive;


pub struct MutexWrapper<T: ?Sized>(pub Mutex<T>);

impl<T: ?Sized + Serialize> Serialize for MutexWrapper<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0
            .lock()
            .expect("mutex is poisoned")
            .serialize(serializer)
    }
}


#[derive(Debug)]
pub struct ParseApkError {
    path: String
}

impl Error for ParseApkError {}

impl fmt::Display for ParseApkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse apk at {}", self.path)
    }
}


fn parse_apk(path: &str) -> Result<(Vec<Dex<impl AsRef<[u8]>>>, Option<Vec<String>>), ParseApkError> {
    let file = match fs::File::open(Path::new(path)) {
        Ok(file) => file,
        _ => return Err(ParseApkError { path: path.to_string() })
    };
    let mut zip_handler = match ZipArchive::new(file) {
        Ok(zip_handler) => zip_handler,
        _ => return Err(ParseApkError { path: path.to_string() })
    };

    let mut dexes = vec![];
    let mut permissions = None;

    for i in 0..zip_handler.len() {
        let (file_name, contents) = {
            let mut current_file = match zip_handler.by_index(i) {
                Ok(file) => file,
                _ => continue
            };
            let mut contents = Vec::new();
            if let Ok(_) = current_file.read_to_end(&mut contents) {
                let is_xml = current_file.name().to_string();
                (is_xml, contents)
            } else {
                continue;
            }
        };

        if file_name == "AndroidManifest.xml" {
            permissions = parse_permissions(contents);
        } else if contents.starts_with(&[100, 101, 120, 10]) {
            if let Ok(dex) = DexReader::from_vec(contents) {
                dexes.push(dex);
            }
        }
    }

    Ok((dexes, permissions))
}


fn main() {
    let args: Args = Args::parse();

    println!("Parsing {} files up to {} opcodes, using {} threads", args.input.len(), args.sequence_cap, args.threads);

    rayon::ThreadPoolBuilder::new().num_threads(args.threads).build_global().unwrap();
    let accumulator = Arc::new(MutexWrapper(Mutex::new(HashMap::new())));
    args.input.par_iter().progress_count(args.input.len() as u64).for_each(|path| {
        if let Ok(dex) = DexReader::from_file(path) {
            let mut instruction_sequence = vec![];
            for class in dex.classes() {
                match class {
                    Ok(class) => {
                        for method in class.methods() {
                            match method.code() {
                                Some(code) => {
                                    let raw_bytecode = code.insns();
                                    let mut offset = 0;
                                    while offset < raw_bytecode.len() {
                                        match Instruction::try_from_raw_bytecode(raw_bytecode, &mut offset) {
                                            Ok(Some(inst)) => instruction_sequence.push(inst),
                                            Ok(None) => break,
                                            Err(_) => {
                                                eprintln!("Error parsing: {}::{}", class.jtype().to_java_type(), method.name());
                                                break;
                                            },
                                        }
                                    }
                                },
                                _ => continue
                            }
                        }
                    }
                    _ => continue
                }
            }
            // let (op_seq, method_bounds) = parse_dexes(dexes, args.sequence_cap);
            // if !op_seq.is_empty() {
            let mut accumulator = accumulator.0.lock().unwrap();
            accumulator.insert(path, instruction_sequence);
            // }
        } else {
            eprintln!("Error parsing: {}", path);
        }
    });

    println!("Writing to file");

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(args.output)
        .unwrap();
    let buffered_file = BufWriter::new(file);

    // serde_json::to_writer(buffered_file, &accumulator).unwrap();
}
