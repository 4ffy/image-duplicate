#![warn(missing_docs)]
//! The main image duplicate program.

use anyhow::{anyhow, Result};
use clap::Parser;
use hashdb::{HashDB, HashEntry};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub mod hashdb;

const SUFFIXES: [&str; 7] = ["bmp", "gif", "jpg", "jpeg", "jxl", "png", "webp"];

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

fn has_image_suffix<P: AsRef<Path>>(file: P) -> bool {
    // uhh...
    match file.as_ref().extension() {
        Some(x) => match x.to_str() {
            Some(x) => SUFFIXES.contains(&x),
            None => false,
        },
        None => false,
    }
}

fn read_dir<P: AsRef<Path>>(root: P) -> Result<HashDB> {
    Ok(fs::read_dir(root)?
        .filter_map(|x| x.ok())
        .filter_map(|x| {
            let p = x.path();
            match has_image_suffix(&p) && p.is_file() {
                true => Some(p),
                false => None,
            }
        })
        .filter_map(|x| HashEntry::read_file(x).ok())
        .collect())
}

fn read_dir_recursive<P: AsRef<Path>>(root: P) -> Result<HashDB> {
    Ok(WalkDir::new(root)
        .into_iter()
        .filter_map(|x| x.ok())
        .filter_map(|x| {
            let p = x.path();
            match has_image_suffix(&p) && p.is_file() {
                true => Some(p.to_owned()),
                false => None,
            }
        })
        .filter_map(|x| HashEntry::read_file(x).ok())
        .collect())
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

    let hashdb = match args.recursive {
        true => read_dir_recursive(&root)?,
        false => read_dir(&root)?,
    };

    hashdb.to_file(&db)?;
    let hashdb2 = HashDB::from_file(&db)?;

    dbg!(&hashdb);
    dbg!(&hashdb2);
    assert_eq!(&hashdb, &hashdb2);

    Ok(())
}
