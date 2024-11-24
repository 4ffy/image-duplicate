// image-dupicate - GUI for handling visually similar images in a directory
// Copyright (C) 2024 Cameron Norton
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! The main image duplicate program.

use anyhow::{anyhow, Result};
use clap::Parser;
use gui::GUI;
use hashdb::HashDB;
use std::path::PathBuf;

mod gui;
mod hashdb;

/// GUI for scanning and handling visually similar images in a directory.
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
    #[arg(short, long, default_value_t = 9)]
    pub threshold: u32,
}

/// Run the image duplicate program.
pub fn run(args: &Args) -> Result<()> {
    let path = args.path.clone();
    if !path.is_dir() {
        return Err(anyhow!("Directory not found: {path:?}"));
    }

    let db_file = match &args.db {
        Some(path) => path.clone(),
        None => path.join(".image_hash.db"),
    };
    eprintln!("Database file is {db_file:?}");

    let mut hashdb = match db_file.is_file() && !args.rebuild {
        true => {
            eprintln!("Reading database file...");
            HashDB::from_file(&db_file)?
        }
        false => {
            eprintln!("Creating new database...");
            HashDB::new()
        }
    };

    if !args.no_update {
        eprintln!("Hashing images in {path:?}...");
        match args.recursive {
            true => hashdb.read_dir_recursive(path)?,
            false => hashdb.read_dir(path)?,
        }
    }

    if !args.no_dump {
        eprintln!("Dumping database to {db_file:?}...");
        hashdb.to_file(&db_file)?;
    }

    eprintln!("Finding duplicate images...");
    let duplicates = hashdb.find_duplicates(args.threshold);

    let gui = GUI::build(duplicates)?;
    gui.run()?;

    Ok(())
}
