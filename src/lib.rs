#![warn(missing_docs)]
//! The main image duplicate program.

use anyhow::{anyhow, Result};
use clap::Parser;
use hashdb::HashDB;
use std::path::PathBuf;

pub mod hashdb;

/// Arguments to the image duplicate program.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Directory to scan for images
    pub path: PathBuf,

    /// Location of database file (default: \<PATH\>/.image_hash.db)
    #[arg(short = 'D', long)]
    pub db: Option<PathBuf>,

    /// Scan directory for images recursively
    #[arg(short = 'R', long)]
    pub recursive: bool,

    /// Force rebuild hash database
    #[arg(short = 'b', long)]
    pub rebuild: bool,

    /// Do not dump hash database to file
    #[arg(short = 'd', long)]
    pub no_dump: bool,

    /// Read database file only; do not update contents
    #[arg(short = 'u', long, conflicts_with = "rebuild")]
    pub no_update: bool,

    /// Image similarity threshold
    #[arg(short, long, default_value_t = 10)]
    pub threshold: u32,
}

/// Run the image duplicate program.
pub fn run(args: &Args) -> Result<()> {
    let root = args.path.clone();
    if !root.is_dir() {
        return Err(anyhow!("Directory not found: {:?}", root));
    }

    if args.threshold < 0 {
        return Err(anyhow!("threshold must be >=0"));
    }

    let db_file = match &args.db {
        Some(path) => path.clone(),
        None => root.join(".image_hash.db"),
    };
    println!("Database file is {:?}.", db_file);

    let mut hashdb = match db_file.is_file() && !args.rebuild {
        true => {
            println!("Reading from {:?}...", db_file);
            HashDB::from_file(&db_file)?
        }
        false => {
            println!("Creating new database...");
            HashDB::new()
        }
    };

    if !args.no_update {
        println!("Hashing images in {:?}...", root);
        match args.recursive {
            true => hashdb.read_dir_recursive(root)?,
            false => hashdb.read_dir(root)?,
        }
    }

    if !args.no_dump {
        println!("Dumping database to {:?}...", db_file);
        hashdb.to_file(&db_file)?;
    }

    println!("Finding duplicate images...");
    dbg!(hashdb.find_duplicates(args.threshold));

    Ok(())
}
