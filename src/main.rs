use clap::Parser;
use ki;

use tracing;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// A simple CLI for the ki library
///
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_env("KI_LOG"))
        .init();

    let s = tracing::span!(tracing::Level::INFO, "main");
    let _enter = s.enter();

    let args = Args::parse();
    for i in 0..args.count {
        tracing::span!(tracing::Level::INFO, "greeting", count= %i, name = %args.name).in_scope(
            || {
                tracing::event!(tracing::Level::INFO, "greeting");
                println!("{} Hello {}!", ki::add(2, 2), args.name);
                tracing::event!(tracing::Level::INFO, name = "after", thing = "woof", "blep");
            },
        );
    }
}
