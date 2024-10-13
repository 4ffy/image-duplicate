//! Structs and methods for dealing with a database of image hashes.
//! [`HashEntry`] and [`HashDB`] form the main interface. `HashDB` provides
//! methods to dump the database and read from a [MessagePack][rmp] file.

use image_hasher::HasherConfig;
use rmp_serde::{config::BytesMode, Serializer};
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use std::{collections::HashSet, fs, hash::Hash, path::Path};
use thiserror::Error;

/// Wrapper around [`md5::Digest`] for serialization.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Digest(md5::Digest);

impl From<md5::Digest> for Digest {
    fn from(value: md5::Digest) -> Self {
        Self(value)
    }
}

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0 .0)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_bytes(DigestVisitor)
    }
}

/// Helper for deserializing [`Digest`].
struct DigestVisitor;

impl<'de> Visitor<'de> for DigestVisitor {
    type Value = Digest;

    fn expecting(
        &self, formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        formatter.write_str("eight bytes representing md5 digest")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match TryInto::<[u8; 16]>::try_into(v) {
            Ok(v) => Ok(md5::Digest(v).into()),
            Err(_) => Err(E::invalid_length(v.len(), &self)),
        }
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

/// Pair a filename along with its [md5][Digest] and [hash][ImageHash]. Entries
/// are considered equivalent if their md5 hashes are equivalent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HashEntry {
    /// Canonicalized filename of the image that the entry is derived from.
    pub filename: String,

    /// md5 hash of the image.
    pub md5: Digest,

    /// Perceptual hash of the image.
    pub hash: ImageHash,
}

impl HashEntry {
    /// Read an image file and form a `HashEntry`.
    pub fn read_file<P: AsRef<Path>>(file: P) -> Result<Self, HashEntryError> {
        // If I am to hash in parallel, each will need its own hasher, probably.
        let hasher = HasherConfig::new().to_hasher();

        let image = image::open(&file)?;
        let temp = image.resize(256, 256, image_hasher::FilterType::Nearest);

        let filename = file.as_ref().canonicalize()?;
        let md5 = md5::compute(&image.as_bytes());
        let hash = hasher.hash_image(&temp);

        Ok(HashEntry {
            filename: filename.to_string_lossy().into_owned(),
            md5: md5.into(),
            hash: hash.into(),
        })
    }
}

impl PartialEq for HashEntry {
    fn eq(&self, other: &Self) -> bool {
        self.md5 == other.md5
    }
}

impl Eq for HashEntry {}

impl Hash for HashEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.md5.hash(state)
    }
}

/// Errors that can happen when dealing with [`HashEntry`].
#[derive(Debug, Error)]
pub enum HashEntryError {
    /// Wrapper around [`image::ImageError`].
    #[error("could not read image: {0}")]
    ImageError(#[from] image::ImageError),

    /// Wrapper around [`std::io::Error`].
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

/// A database storing [`HashEntry`] items via an internal [`HashSet`].
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HashDB(HashSet<HashEntry>);

impl HashDB {
    /// Create a new hash database.
    pub fn new() -> Self {
        HashDB(HashSet::new())
    }

    /// Write the database to a [MessagePack][rmp] file.
    pub fn to_file<P: AsRef<Path>>(&self, file: P) -> Result<(), HashDBError> {
        // Use this method over `rmp_serde::to_vec` to avoid overhead on packing
        // bytes. (If this breaks decoding, maybe live with the overhead?)
        let mut buf: Vec<u8> = Vec::new();
        self.serialize(
            &mut Serializer::new(&mut buf).with_bytes(BytesMode::ForceAll),
        )?;
        fs::write(file, buf)?;
        Ok(())
    }

    /// Read a database from a [MessagePack][rmp] file.
    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, HashDBError> {
        Ok(rmp_serde::from_read(fs::read(file)?.as_slice())?)
    }

    /// Insert a [`HashEntry`] into the database. This defers to
    /// [`HashSet::insert`] and thus has the same semantics.
    pub fn insert(&mut self, entry: &HashEntry) -> bool {
        self.0.insert(entry.clone())
    }
}

impl FromIterator<HashEntry> for HashDB {
    fn from_iter<T: IntoIterator<Item = HashEntry>>(iter: T) -> Self {
        let mut db = HashDB::new();
        for i in iter {
            db.insert(&i);
        }
        db
    }
}

impl IntoIterator for HashDB {
    type Item = HashEntry;
    type IntoIter = std::collections::hash_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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

    /// Wrapper around [`std::io::Error`].
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}
