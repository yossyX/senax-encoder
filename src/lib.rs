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
//! ### External Crate Support
//! - `chrono` — Enables encoding/decoding of `chrono::DateTime`, `NaiveDate`, and `NaiveTime` types.
//! - `uuid` — Enables encoding/decoding of `uuid::Uuid`.
//! - `ulid` — Enables encoding/decoding of `ulid::Ulid` (shares the same tag as UUID for binary compatibility).
//! - `rust_decimal` — Enables encoding/decoding of `rust_decimal::Decimal`.
//! - `indexmap` — Enables encoding/decoding of `IndexMap` and `IndexSet` collections.
//! - `fxhash` — Enables encoding/decoding of `fxhash::FxHashMap` and `fxhash::FxHashSet` (fast hash collections).
//! - `ahash` — Enables encoding/decoding of `ahash::AHashMap` and `ahash::AHashSet` (high-performance hash collections).
//! - `smol_str` — Enables encoding/decoding of `smol_str::SmolStr` (small string optimization).
//! - `serde_json` — Enables encoding/decoding of `serde_json::Value` (JSON values as dynamic type).

pub mod core;
mod features;

use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use senax_encoder_derive::{Decode, Encode, Pack, Unpack};
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

/// Errors that can occur during encoding or decoding operations.
#[derive(Debug, thiserror::Error)]
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
    /// Struct-specific decode error
    #[error(transparent)]
    StructDecode(#[from] StructDecodeError),
    /// Enum-specific decode error
    #[error(transparent)]
    EnumDecode(#[from] EnumDecodeError),
}

/// The result type used throughout this crate for encode/decode operations.
///
/// All `Encode` and `Decode` trait methods return this type.
pub type Result<T> = std::result::Result<T, EncoderError>;

/// Derive-specific error types for struct operations
#[derive(Debug, thiserror::Error)]
pub enum StructDecodeError {
    #[error("Expected struct named tag ({expected}), got {actual}")]
    InvalidTag { expected: u8, actual: u8 },
    #[error("Required field '{field}' not found for struct {struct_name}")]
    MissingRequiredField {
        field: &'static str,
        struct_name: &'static str,
    },
    #[error("Field count mismatch for struct {struct_name}: expected {expected}, got {actual}")]
    FieldCountMismatch {
        struct_name: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("Structure hash mismatch for {struct_name}: expected 0x{expected:016X}, got 0x{actual:016X}")]
    StructureHashMismatch {
        struct_name: &'static str,
        expected: u64,
        actual: u64,
    },
}

/// Derive-specific error types for enum operations
#[derive(Debug, thiserror::Error)]
pub enum EnumDecodeError {
    #[error("Unknown enum tag: {tag} for enum {enum_name}")]
    UnknownTag { tag: u8, enum_name: &'static str },
    #[error("Unknown variant ID: 0x{variant_id:016X} for enum {enum_name}")]
    UnknownVariantId {
        variant_id: u64,
        enum_name: &'static str,
    },
    #[error("Unknown unit variant ID: 0x{variant_id:016X} for enum {enum_name}")]
    UnknownUnitVariantId {
        variant_id: u64,
        enum_name: &'static str,
    },
    #[error("Unknown named variant ID: 0x{variant_id:016X} for enum {enum_name}")]
    UnknownNamedVariantId {
        variant_id: u64,
        enum_name: &'static str,
    },
    #[error("Unknown unnamed variant ID: 0x{variant_id:016X} for enum {enum_name}")]
    UnknownUnnamedVariantId {
        variant_id: u64,
        enum_name: &'static str,
    },
    #[error("Required field '{field}' not found for variant {enum_name}::{variant_name}")]
    MissingRequiredField {
        field: &'static str,
        enum_name: &'static str,
        variant_name: &'static str,
    },
    #[error("Field count mismatch for variant {enum_name}::{variant_name}: expected {expected}, got {actual}")]
    FieldCountMismatch {
        enum_name: &'static str,
        variant_name: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("Structure hash mismatch for variant {enum_name}::{variant_name}: expected 0x{expected:016X}, got 0x{actual:016X}")]
    StructureHashMismatch {
        enum_name: &'static str,
        variant_name: &'static str,
        expected: u64,
        actual: u64,
    },
}

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

    /// Returns true if this value equals its default value.
    /// Used by `#[senax(skip_default)]` attribute to skip encoding default values.
    fn is_default(&self) -> bool;
}

/// Trait for types that can be packed into a compact binary format.
///
/// This trait provides compact serialization without schema evolution support.
/// Use this when you need maximum performance and don't require forward/backward compatibility.
///
/// # Errors
/// Returns `EncoderError` if the value cannot be packed.
pub trait Packer {
    /// Pack the value into the given buffer without schema evolution support.
    ///
    /// This method stores data in a compact format without field IDs or type tags.
    /// The format is not schema-evolution-friendly but offers better performance.
    ///
    /// # Arguments
    /// * `writer` - The buffer to write the packed bytes into.
    fn pack(&self, writer: &mut BytesMut) -> Result<()>;
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
}

/// Trait for types that can be unpacked from a compact binary format.
///
/// This trait provides compact deserialization without schema evolution support.
/// Use this when you need maximum performance and don't require forward/backward compatibility.
///
/// # Errors
/// Returns `EncoderError` if the value cannot be unpacked or the data is invalid.
pub trait Unpacker: Sized {
    /// Unpack the value from the given buffer without schema evolution support.
    ///
    /// This method reads data from a compact format without field IDs or type tags.
    /// The format is not schema-evolution-friendly but offers better performance.
    ///
    /// # Arguments
    /// * `reader` - The buffer to read the packed bytes from.
    fn unpack(reader: &mut Bytes) -> Result<Self>;
}

/// Convenience function to pack a value to bytes.
///
/// This is equivalent to calling `value.pack(writer)` but provides a more ergonomic API.
/// The packed format is compact but not schema-evolution-friendly.
///
/// # Arguments
/// * `value` - The value to pack.
///
/// # Example
/// ```rust
/// use senax_encoder::{pack, unpack, Pack, Unpack};
/// use bytes::BytesMut;
///
/// #[derive(Pack, Unpack, PartialEq, Debug)]
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
pub fn pack<T: Packer>(value: &T) -> Result<Bytes> {
    let mut writer = BytesMut::new();
    value.pack(&mut writer)?;
    Ok(writer.freeze())
}

/// Convenience function to unpack a value from bytes.
///
/// This is equivalent to calling `T::unpack(reader)` but provides a more ergonomic API.
/// The packed format is compact but not schema-evolution-friendly.
///
/// # Arguments
/// * `reader` - The buffer to read the packed bytes from.
///
/// # Example
/// ```rust
/// use senax_encoder::{pack, unpack, Pack, Unpack};
/// use bytes::BytesMut;
///
/// #[derive(Pack, Unpack, PartialEq, Debug)]
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
pub fn unpack<T: Unpacker>(reader: &mut Bytes) -> Result<T> {
    T::unpack(reader)
}
