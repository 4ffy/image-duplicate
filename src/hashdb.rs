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

//! Structs and methods for dealing with a database of image hashes. [`HashDB`]
//! forms the main interface. `HashDB` is backed by a [`HashMap`] and supports
//! hashing image files as well as reading and writing to Zlib'd
//! [MessagePack][`rmp`].

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use image_hasher::HasherConfig;
use permutator::LargeCombinationIterator;
use rayon::iter::{ParallelBridge, ParallelIterator};
use rmp_serde::{Serializer, config::BytesMode};
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs,
    hash::Hash,
    io::Write,
    path::Path,
};
use thiserror::Error;
use walkdir::WalkDir;

const SUFFIXES: [&str; 7] = ["bmp", "gif", "jpg", "jpeg", "jxl", "png", "webp"];

/// Wrapper around [`image_hasher::ImageHash`] for serialization.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ImageHash(image_hasher::ImageHash);

impl From<image_hasher::ImageHash> for ImageHash {
    fn from(value: image_hasher::ImageHash) -> Self {
        Self(value)
    }
}

impl Serialize for ImageHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes())
    }
}

impl<'de> Deserialize<'de> for ImageHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_bytes(ImageHashVisitor)
    }
}

/// Helper for deserializing [`ImageHash`].
struct ImageHashVisitor;

impl<'de> Visitor<'de> for ImageHashVisitor {
    type Value = ImageHash;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        formatter.write_str("bytes representing an image hash")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match image_hasher::ImageHash::<Box<[u8]>>::from_bytes(v) {
            Ok(v) => Ok(v.into()),
            Err(_) => Err(E::invalid_value(
                de::Unexpected::Other("image hash bytes"),
                &self,
            )),
        }
    }
}

/// A database storing image hashes via an internal [`HashMap`] that pairs the
/// canonicalized filename of the image with its perceptual hash.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HashDB(HashMap<String, ImageHash>);

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

fn hash_image<P: AsRef<Path>>(
    file: P,
) -> Result<(String, ImageHash), HashDBError> {
    let hasher = HasherConfig::new().to_hasher();

    let image = match image::open(&file) {
        Ok(i) => Ok(i),
        Err(e) => {
            Err(HashDBError::ImageError(format!("{:?}", file.as_ref()), e))
        }
    }?;

    let temp = image
        .resize(256, 256, image_hasher::FilterType::Nearest)
        .blur(3.0);

    let name = file.as_ref().canonicalize()?.to_string_lossy().into_owned();
    let hash = hasher.hash_image(&temp);

    Ok((name, hash.into()))
}

impl HashDB {
    /// Create a new hash database.
    pub fn new() -> Self {
        HashDB(HashMap::new())
    }

    /// Read image files from the given directory. Add entries for any images
    /// that do not exist the database. Then, remove entries from the database
    /// that no longer have any corresponding images on the filesystem.
    pub fn read_dir<P: AsRef<Path>>(
        &mut self,
        root: P,
    ) -> Result<(), HashDBError> {
        // I have to clone the keys from the DB because if I use references, It
        // borrows the database and I can't insert any new entries.
        let db_images: HashSet<String> =
            self.0.keys().map(|x| x.clone()).collect();

        let fs_images: HashSet<String> = fs::read_dir(&root)?
            .filter_map(|x| x.ok())
            .filter_map(|x| {
                let p = x.path();
                match has_image_suffix(&p) {
                    true => p.canonicalize().ok(),
                    false => None,
                }
            })
            .map(|x| x.to_string_lossy().into_owned())
            .collect();

        // Images on filesystem but not in DB - Add to DB
        let hashes: Vec<(String, ImageHash)> = fs_images
            .difference(&db_images)
            .par_bridge()
            .map(|img| hash_image(img))
            .collect::<Result<Vec<_>, _>>()?;
        for (name, hash) in hashes {
            self.0.insert(name, hash);
        }

        // Images in DB but not on filesystem - Remove from DB
        for file in db_images.difference(&fs_images) {
            self.0.remove(file);
        }

        Ok(())
    }

    /// [`read_dir`][HashDB::read_dir] but scan the directory recursively. This
    /// could be combined with `read_dir` via a recursive flag or whatever, but
    /// no.
    pub fn read_dir_recursive<P: AsRef<Path>>(
        &mut self,
        root: P,
    ) -> Result<(), HashDBError> {
        let db_images: HashSet<String> =
            self.0.keys().map(|x| x.clone()).collect();

        let fs_images: HashSet<String> = WalkDir::new(&root)
            .into_iter()
            .filter_map(|x| x.ok())
            .filter_map(|x| {
                let p = x.path();
                match has_image_suffix(&p) && p.is_file() {
                    true => p.canonicalize().ok(),
                    false => None,
                }
            })
            .map(|x| x.to_string_lossy().into_owned())
            .collect();

        // Images on filesystem but not in DB - Add to DB
        let hashes: Vec<(String, ImageHash)> = fs_images
            .difference(&db_images)
            .par_bridge()
            .map(|img| hash_image(img))
            .collect::<Result<Vec<_>, _>>()?;
        for (name, hash) in hashes {
            self.0.insert(name, hash);
        }

        // Images in DB but not on filesystem - Remove from DB
        for file in db_images.difference(&fs_images) {
            self.0.remove(file);
        }

        Ok(())
    }

    /// Search through all pairs of images in the database for all images that
    /// have a Hamming distance (according to [`image_hasher::ImageHash::dist`])
    /// below the given threshold.
    pub fn find_duplicates(&self, threshold: u32) -> Vec<(String, String)> {
        let entries: Vec<(&String, &ImageHash)> = self.0.iter().collect();
        LargeCombinationIterator::new(&entries, 2)
            .filter_map(|comb| {
                let (name_1, hash_1) = *comb[0];
                let (name_2, hash_2) = *comb[1];
                if hash_1.0.dist(&hash_2.0) < threshold {
                    Some((name_1.clone(), name_2.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Write the database to a Zlib'd [MessagePack][rmp] file.
    pub fn to_file<P: AsRef<Path>>(&self, file: P) -> Result<(), HashDBError> {
        // Use this method over `rmp_serde::to_vec` to avoid overhead on packing
        // bytes. (If this breaks decoding, maybe live with the overhead?)
        let mut buf: Vec<u8> = Vec::new();
        self.serialize(
            &mut Serializer::new(&mut buf).with_bytes(BytesMode::ForceAll),
        )?;
        let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
        z.write_all(&buf)?;
        fs::write(file, z.finish()?)?;
        Ok(())
    }

    /// Read a database from a Zlib'd [MessagePack][rmp] file.
    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, HashDBError> {
        Ok(rmp_serde::from_read(ZlibDecoder::new(
            fs::read(file)?.as_slice(),
        ))?)
    }
}

impl Display for HashDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (k, v) in self.0.iter() {
            write!(f, "{}\t{k}\n", v.0.to_base64())?;
        }
        Ok(())
    }
}

/// Errors that can happen when dealing with [`HashDB`].
#[derive(Debug, Error)]
pub enum HashDBError {
    /// Wrapper around [`rmp_serde::decode::Error`].
    #[error("Could not decode database: {0}")]
    DecodeError(#[from] rmp_serde::decode::Error),

    /// Wrapper around [`rmp_serde::encode::Error`].
    #[error("Could not encode database: {0}")]
    EncodeError(#[from] rmp_serde::encode::Error),

    /// Wrapper around [`image::ImageError`].
    #[error("Could not read {0}: {1}")]
    ImageError(String, image::ImageError),

    /// Wrapper around [`std::io::Error`].
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}
