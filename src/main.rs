use clap::Parser;
use image_duplicate::{self, Args};
use std::process;

fn main() {
    let args = Args::parse();
    if let Err(e) = image_duplicate::run(&args) {
        eprintln!("{e}");
        process::exit(1);
    }
}
