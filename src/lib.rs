#![warn(missing_docs)]
//! The main image duplicate program.

use anyhow::{anyhow, Result};
use clap::Parser;
use hashdb::{HashDB, HashEntry};
use std::{fs, path::PathBuf};

pub mod hashdb;

/// Arguments to the image duplicate program.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Directory to scan for images
    pub path: PathBuf,

    /// Location of database file
    #[arg(short, long)]
    pub db: Option<PathBuf>,

    /// Scan directories for images recursively
    #[arg(short, long)]
    pub recursive: bool,
}

/// Run the image duplicate program.
pub fn run(args: &Args) -> Result<()> {
    let root = args.path.clone();
    if !root.is_dir() {
        return Err(anyhow!("Directory not found: {:?}", root));
    }

    let db = match &args.db {
        Some(path) => path.clone(),
        None => root.join(".image_hash.db"),
    };

    let mut hashdb = HashDB::new();
    for entry in fs::read_dir(root)?.filter_map(|x| x.ok()) {
        hashdb.insert(&HashEntry::read_file(entry.path())?);
    }

    hashdb.to_file(&db)?;
    let hashdb2 = HashDB::from_file(&db)?;

    dbg!(&hashdb);
    dbg!(&hashdb2);
    assert_eq!(&hashdb, &hashdb2);

    Ok(())
}
