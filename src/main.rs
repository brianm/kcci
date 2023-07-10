use clap::{Parser, Subcommand};
use kcci::ingest;
use log::info;
use std::io::{self, BufReader};

/// A simple CLI for the kcci library
///
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Ingest,
}

fn main() {
    env_logger::init_from_env("KCCI_LOG");
    let args = Args::parse();
    match args.command {
        Commands::Ingest => {
            let mut reader = std::io::stdin().lock();
            let out = ingest::parse_paste(&mut reader).unwrap();
            for c in out {
                println!("{:?}", c);
            }
        }
    }
}
