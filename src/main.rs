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
