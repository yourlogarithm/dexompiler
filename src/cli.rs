use clap::Parser;
use num_cpus;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Output file
    #[arg(short, long)]
    pub output: String,
    
    /// Input files
    #[arg(short, long, num_args = 1..=2097152)]
    pub input: Vec<String>,

    /// Number of threads to use
    #[arg(short, long, default_value_t = num_cpus::get())]
    pub threads: usize,
}