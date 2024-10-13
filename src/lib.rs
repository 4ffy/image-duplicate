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
}

/// Run the image duplicate program.
pub fn run(args: &Args) -> Result<()> {
    let root = args.path.clone();
    if !root.is_dir() {
        return Err(anyhow!("Directory not found: {:?}", root));
    }

    let db_file = match &args.db {
        Some(path) => path.clone(),
        None => root.join(".image_hash.db"),
    };

    let mut hashdb = match db_file.is_file() && !args.rebuild {
        true => HashDB::from_file(&db_file)?,
        false => HashDB::new(),
    };

    if !args.no_update {
        match args.recursive {
            true => hashdb.read_dir_recursive(root)?,
            false => hashdb.read_dir(root)?,
        }
    }

    dbg!(&hashdb);

    if !args.no_dump {
        hashdb.to_file(&db_file)?;
        let hashdb2 = HashDB::from_file(&db_file)?;
        assert_eq!(&hashdb, &hashdb2);
    }

    Ok(())
}
