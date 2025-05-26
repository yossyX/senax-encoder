//! # senax-encoder
//!
//! A fast, compact, and schema-evolution-friendly binary serialization library for Rust.
//!
//! - Supports struct/enum encoding with field/variant IDs for forward/backward compatibility
//! - Efficient encoding for primitives, collections, Option, String, bytes, and popular crates (chrono, uuid, ulid, rust_decimal, indexmap, fxhash, ahash, smol_str)
//! - Custom derive macros for ergonomic usage
//! - Feature-gated support for optional dependencies
//!
//! ## Attribute Macros
//!
//! You can control encoding/decoding behavior using the following attributes:
//!
//! - `#[senax(id = N)]` — Assigns a custom field or variant ID (u64). Ensures stable wire format across versions.
//! - `#[senax(default)]` — If a field is missing during decoding, its value is set to `Default::default()` instead of causing an error. For `Option<T>`, this means `None`.
//! - `#[senax(skip_encode)]` — This field is not written during encoding. On decode, it is set to `Default::default()`.
//! - `#[senax(skip_decode)]` — This field is ignored during decoding and always set to `Default::default()`. It is still encoded if present.
//! - `#[senax(skip_default)]` — This field is not written during encoding if its value equals the default value. On decode, missing fields are set to `Default::default()`.
//! - `#[senax(rename = "name")]` — Use the given string as the logical field/variant name for ID calculation. Useful for renaming fields/variants while keeping the same wire format.
//!
//! ## Feature Flags
//!
//! The following optional features enable support for popular crates and types:
//!
//! - `chrono` — Enables encoding/decoding of `chrono::DateTime`, `NaiveDate`, and `NaiveTime` types.
//! - `uuid` — Enables encoding/decoding of `uuid::Uuid`.
//! - `ulid` — Enables encoding/decoding of `ulid::Ulid` (shares the same tag as UUID for binary compatibility).
//! - `rust_decimal` — Enables encoding/decoding of `rust_decimal::Decimal`.
//! - `indexmap` — Enables encoding/decoding of `IndexMap` and `IndexSet` collections.
//! - `fxhash` — Enables encoding/decoding of `fxhash::FxHashMap` and `fxhash::FxHashSet` (fast hash collections).
//! - `ahash` — Enables encoding/decoding of `ahash::AHashMap` and `ahash::AHashSet` (high-performance hash collections).
//! - `smol_str` — Enables encoding/decoding of `smol_str::SmolStr` (small string optimization).
//! - `serde_json` — Enables encoding/decoding of `serde_json::Value` (JSON values as dynamic type).
//!
//! ## Example
//! ```rust
//! use senax_encoder::{Encode, Decode};
//! use bytes::BytesMut;
//!
//! #[derive(Encode, Decode, PartialEq, Debug)]
//! struct MyStruct {
//!     id: u32,
//!     name: String,
//! }
//!
//! let value = MyStruct { id: 42, name: "hello".to_string() };
//! let mut buf = senax_encoder::encode(&value).unwrap();
//! let decoded: MyStruct = senax_encoder::decode(&mut buf).unwrap();
//! assert_eq!(value, decoded);
//! ```

pub mod core;
mod features;

use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use senax_encoder_derive::{Decode, Encode};
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;
use thiserror::Error;

/// Error type for all encoding and decoding operations in this crate.
///
/// This error type is returned by all `Encode` and `Decode` trait methods.
/// It covers I/O errors, encoding/decoding logic errors, and buffer underflow.
#[derive(Debug, Error)]
pub enum EncoderError {
    /// The value could not be encoded (e.g., unsupported type or logic error).
    #[error("Encode error: {0}")]
    Encode(String),
    /// The value could not be decoded (e.g., invalid data, type mismatch, or schema evolution error).
    #[error("Decode error: {0}")]
    Decode(String),
    /// The buffer did not contain enough data to complete the operation.
    #[error("Insufficient data in buffer")]
    InsufficientData,
}

/// The result type used throughout this crate for encode/decode operations.
///
/// All `Encode` and `Decode` trait methods return this type.
pub type Result<T> = std::result::Result<T, EncoderError>;

/// Convenience function to decode a value from bytes.
///
/// This is equivalent to calling `T::decode(reader)` but provides a more ergonomic API.
///
/// # Arguments
/// * `reader` - The buffer to read the encoded bytes from.
///
/// # Example
/// ```rust
/// use senax_encoder::{encode, decode, Encode, Decode};
/// use bytes::BytesMut;
///
/// #[derive(Encode, Decode, PartialEq, Debug)]
/// struct MyStruct {
///     id: u32,
///     name: String,
/// }
///
/// let value = MyStruct { id: 42, name: "hello".to_string() };
/// let mut buf = encode(&value).unwrap();
/// let decoded: MyStruct = decode(&mut buf).unwrap();
/// assert_eq!(value, decoded);
/// ```
pub fn decode<T: Decoder>(reader: &mut Bytes) -> Result<T> {
    T::decode(reader)
}

/// Convenience function to encode a value to bytes.
///
/// This is equivalent to calling `value.encode(writer)` but provides a more ergonomic API.
///
/// # Arguments
/// * `value` - The value to encode.
/// * `writer` - The buffer to write the encoded bytes into.
///
/// # Example
/// ```rust
/// use senax_encoder::{encode, decode, Encode, Decode};
/// use bytes::BytesMut;
///
/// #[derive(Encode, Decode, PartialEq, Debug)]
/// struct MyStruct {
///     id: u32,
///     name: String,
/// }
///
/// let value = MyStruct { id: 42, name: "hello".to_string() };
/// let mut buf = encode(&value).unwrap();
/// let decoded: MyStruct = decode(&mut buf).unwrap();
/// assert_eq!(value, decoded);
/// ```
pub fn encode<T: Encoder>(value: &T) -> Result<Bytes> {
    let mut writer = BytesMut::new();
    value.encode(&mut writer)?;
    Ok(writer.freeze())
}

/// Trait for types that can be encoded into the senax binary format.
///
/// Implement this trait for your type to enable serialization.
/// Most users should use `#[derive(Encode)]` instead of manual implementation.
///
/// # Errors
/// Returns `EncoderError` if the value cannot be encoded.
pub trait Encoder {
    /// Encode the value into the given buffer with schema evolution support.
    ///
    /// This method includes field IDs and type tags for forward/backward compatibility.
    /// Use this when you need schema evolution support.
    ///
    /// # Arguments
    /// * `writer` - The buffer to write the encoded bytes into.
    fn encode(&self, writer: &mut BytesMut) -> Result<()>;

    /// Pack the value into the given buffer without schema evolution support.
    ///
    /// This method omits field IDs and some type tags for compact encoding.
    /// Fields are encoded in declaration order. Use this when you need
    /// maximum performance and compact size, but don't need schema evolution.
    ///
    /// # Arguments
    /// * `writer` - The buffer to write the packed bytes into.
    ///
    /// # Note
    /// - For structs: Field IDs are omitted, fields are encoded in declaration order
    /// - For enums: Variant IDs are still included, but field IDs within variants are omitted
    /// - For primitives: Some type tags may be omitted for maximum compactness
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        self.encode(writer)
    }

    /// Returns true if this value equals its default value.
    /// Used by `#[senax(skip_default)]` attribute to skip encoding default values.
    fn is_default(&self) -> bool;
}

/// Trait for types that can be decoded from the senax binary format.
///
/// Implement this trait for your type to enable deserialization.
/// Most users should use `#[derive(Decode)]` instead of manual implementation.
///
/// # Errors
/// Returns `EncoderError` if the value cannot be decoded or the data is invalid.
pub trait Decoder: Sized {
    /// Decode the value from the given buffer with schema evolution support.
    ///
    /// This method expects field IDs and type tags for forward/backward compatibility.
    /// Use this when you need schema evolution support.
    ///
    /// # Arguments
    /// * `reader` - The buffer to read the encoded bytes from.
    fn decode(reader: &mut Bytes) -> Result<Self>;

    /// Unpack the value from the given buffer without schema evolution support.
    ///
    /// This method expects data packed without field IDs and some type tags.
    /// Fields are decoded in declaration order. Use this when you need
    /// maximum performance and the data was packed with `pack()`.
    ///
    /// # Arguments
    /// * `reader` - The buffer to read the packed bytes from.
    ///
    /// # Note
    /// - For structs: Field IDs are not expected, fields are decoded in declaration order
    /// - For enums: Variant IDs are still expected, but field IDs within variants are not
    /// - For primitives: Some type tags may be omitted, with relaxed validation
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        Self::decode(reader)
    }
}

/// Convenience function to pack a value to bytes.
///
/// This is equivalent to calling `value.pack(writer)` but provides a more ergonomic API.
/// Unlike `encode`, this does not store field IDs and is not schema-evolution-friendly.
///
/// # Arguments
/// * `value` - The value to pack.
///
/// # Example
/// ```rust
/// use senax_encoder::{pack, unpack, Encode, Decode};
/// use bytes::BytesMut;
///
/// #[derive(Encode, Decode, PartialEq, Debug)]
/// struct MyStruct {
///     id: u32,
///     name: String,
/// }
///
/// let value = MyStruct { id: 42, name: "hello".to_string() };
/// let mut buf = pack(&value).unwrap();
/// let decoded: MyStruct = unpack(&mut buf).unwrap();
/// assert_eq!(value, decoded);
/// ```
pub fn pack<T: Encoder>(value: &T) -> Result<Bytes> {
    let mut writer = BytesMut::new();
    value.pack(&mut writer)?;
    Ok(writer.freeze())
}

/// Convenience function to unpack a value from bytes.
///
/// This is equivalent to calling `T::unpack(reader)` but provides a more ergonomic API.
/// Unlike `decode`, this does not expect field IDs and is not schema-evolution-friendly.
///
/// # Arguments
/// * `reader` - The buffer to read the packed bytes from.
///
/// # Example
/// ```rust
/// use senax_encoder::{pack, unpack, Encode, Decode};
/// use bytes::BytesMut;
///
/// #[derive(Encode, Decode, PartialEq, Debug)]
/// struct MyStruct {
///     id: u32,
///     name: String,
/// }
///
/// let value = MyStruct { id: 42, name: "hello".to_string() };
/// let mut buf = pack(&value).unwrap();
/// let decoded: MyStruct = unpack(&mut buf).unwrap();
/// assert_eq!(value, decoded);
/// ```
pub fn unpack<T: Decoder>(reader: &mut Bytes) -> Result<T> {
    T::unpack(reader)
}
