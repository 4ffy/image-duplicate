//! Structs and methods for dealing with a database of image hashes. [`HashDB`]
//! forms the main interface. `HashDB` is backed by a [`HashMap`] and supports
//! hashing image files as well as reading and writing to Zlib'd
//! [MessagePack][`rmp`].

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use image_hasher::HasherConfig;
use rmp_serde::{config::BytesMode, Serializer};
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use std::{
    collections::HashMap, fmt::Display, fs, hash::Hash, io::Write, path::Path,
};
use thiserror::Error;

const SUFFIXES: [&str; 7] = ["bmp", "gif", "jpg", "jpeg", "jxl", "png", "webp"];

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
        &self, formatter: &mut std::fmt::Formatter,
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

/// A database storing image hashes via an internal [`HashMap`].
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HashDB(HashMap<String, ImageHash>);

impl HashDB {
    /// Create a new hash database.
    pub fn new() -> Self {
        HashDB(HashMap::new())
    }

    /// Hash an image and insert it into the database.
    pub fn insert_image<P: AsRef<Path>>(
        &mut self, file: P,
    ) -> Result<(), HashDBError> {
        // If I am to hash in parallel, each will need its own hasher, probably.
        let hasher = HasherConfig::new().to_hasher();

        let image = image::open(&file)?;
        let temp = image.resize(256, 256, image_hasher::FilterType::Nearest);

        let name = file.as_ref().canonicalize()?.to_string_lossy().into_owned();
        let hash = hasher.hash_image(&temp);

        self.0.insert(name, hash.into());
        Ok(())
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

    /// Read image files from the given directory. Add entries for any images
    /// that do not exist the database. Then, remove entries from the database
    /// that no longer have any corresponding images on the filesystem.
    pub fn read_dir<P: AsRef<Path>>(
        &mut self, root: P,
    ) -> Result<(), HashDBError> {
        for entry in fs::read_dir(root)?
            .filter_map(|x| x.ok())
            .filter(|x| has_image_suffix(x.path()))
        {
            self.insert_image(entry.path())?;
        }
        Ok(())
    }

    /// [`read_dir`][HashDB.read_dir] but scan the directory recursively.
    pub fn read_dir_recursive<P: AsRef<Path>>(
        &mut self, _root: P,
    ) -> Result<(), HashDBError> {
        unimplemented!()
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
    #[error("could not decode database: {0}")]
    DecodeError(#[from] rmp_serde::decode::Error),

    /// Wrapper around [`rmp_serde::encode::Error`].
    #[error("could not encode database: {0}")]
    EncodeError(#[from] rmp_serde::encode::Error),

    /// Wrapper around [`image::ImageError`]
    #[error("could not read image: {0}")]
    ImageError(#[from] image::ImageError),

    /// Wrapper around [`std::io::Error`].
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}
