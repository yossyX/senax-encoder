use crate::*;

/// Type tags used in the senax binary format.
///
/// These tags are written as the first byte of each encoded value to identify its type and optimize decoding.
/// Most users do not need to use these directly.
///
/// - Primitives, Option, String, Vec, arrays, maps, structs, enums, and feature types each have their own tag(s).
/// - Tags are stable and part of the wire format.

///< 0 for numbers, false for bool
pub const TAG_ZERO: u8 = 0;
///< 1 for numbers, true for bool
// 5-130: Values 2-127 (compact encoding for small unsigned integers)
pub const TAG_ONE: u8 = 1;
pub const TAG_U8_127: u8 = 127; // 127
pub const TAG_NONE: u8 = 128;
pub const TAG_SOME: u8 = 129;
pub const TAG_U8: u8 = 131;
pub const TAG_U16: u8 = 132;
pub const TAG_U32: u8 = 133;
pub const TAG_U64: u8 = 134;
pub const TAG_U128: u8 = 135;
///< Negative signed integer (bit-inverted encoding)
pub const TAG_NEGATIVE: u8 = 136;
pub const TAG_F32: u8 = 137;
pub const TAG_F64: u8 = 138;
///< Short string (length in tag) - String, SmolStr
pub const TAG_STRING_BASE: u8 = 139;
///< Long string (length encoded) - String, SmolStr
pub const TAG_STRING_LONG: u8 = 180;
///< Vec<u8> or Bytes
pub const TAG_BINARY: u8 = 181;
///< Unit struct
pub const TAG_STRUCT_UNIT: u8 = 182;
///< Named struct
pub const TAG_STRUCT_NAMED: u8 = 183;
///< Tuple struct
pub const TAG_STRUCT_UNNAMED: u8 = 184;
pub const TAG_ENUM: u8 = 185;
///< Enum with named fields
pub const TAG_ENUM_NAMED: u8 = 186;
///< Enum with tuple fields
pub const TAG_ENUM_UNNAMED: u8 = 187;
///< Short array/vec/set (length in tag) - includes HashSet, BTreeSet, IndexSet, FxHashSet, AHashSet
pub const TAG_ARRAY_VEC_SET_BASE: u8 = 188;
///< Long array/vec/set (length encoded) - includes HashSet, BTreeSet, IndexSet, FxHashSet, AHashSet
pub const TAG_ARRAY_VEC_SET_LONG: u8 = 194;
///< Tuple
pub const TAG_TUPLE: u8 = 195;
///< Map (HashMap, BTreeMap, IndexMap, FxHashMap, AHashMap)
pub const TAG_MAP: u8 = 196;
///< chrono::DateTime
pub const TAG_CHRONO_DATETIME: u8 = 197;
///< chrono::NaiveDate
pub const TAG_CHRONO_NAIVE_DATE: u8 = 198;
///< chrono::NaiveTime
pub const TAG_CHRONO_NAIVE_TIME: u8 = 199;
///< rust_decimal::Decimal
pub const TAG_DECIMAL: u8 = 200;
///< uuid::Uuid, ulid::Ulid
pub const TAG_UUID: u8 = 201;
pub const TAG_JSON_NULL: u8 = 202;
pub const TAG_JSON_BOOL: u8 = 203; // Uses existing TAG_ZERO/TAG_ONE for value
pub const TAG_JSON_NUMBER: u8 = 204;
pub const TAG_JSON_STRING: u8 = 205; // Uses existing string encoding
pub const TAG_JSON_ARRAY: u8 = 206;
pub const TAG_JSON_OBJECT: u8 = 207;

// --- bool ---
/// Encodes a `bool` as a single tag byte: `TAG_ZERO` for `false`, `TAG_ONE` for `true`.
impl Encoder for bool {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        let tag = if !*self { TAG_ZERO } else { TAG_ONE }; // 0: false, 1: true
        writer.put_u8(tag);
        Ok(())
    }

    fn is_default(&self) -> bool {
        !(*self)
    }
}
/// Decodes a `bool` from a single tag byte.
///
/// # Errors
/// Returns an error if the tag is not `TAG_ZERO` or `TAG_ONE`.
impl Decoder for bool {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_ZERO => Ok(false),
            TAG_ONE => Ok(true),
            other => Err(EncoderError::Decode(format!(
                "Expected bool tag ({} or {}), got {}",
                TAG_ZERO, TAG_ONE, other
            ))),
        }
    }

    /// Unpacks a `bool` from a single byte with relaxed validation.
    ///
    /// 0 is interpreted as `false`, any non-zero value is interpreted as `true`.
    /// No error checking is performed for invalid values.
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let value = reader.get_u8();
        Ok(value != TAG_ZERO)
    }
}

// --- Common decode functions ---
/// Decodes a `u8` value from a tag and buffer.
/// Used internally for compact integer decoding.
///
/// # Errors
/// Returns an error if the tag is not valid for a `u8`.
#[inline]
fn decode_u8_from_tag(tag: u8, reader: &mut Bytes) -> Result<u8> {
    if (TAG_ZERO..=TAG_U8_127).contains(&tag) {
        Ok(tag - TAG_ZERO)
    } else if tag == TAG_U8 {
        if reader.remaining() < 1 {
            return Err(EncoderError::InsufficientData);
        }
        let stored_val = reader.get_u8();
        stored_val.checked_add(128).ok_or_else(|| {
            EncoderError::Decode(format!("u8 TAG_U8 value overflow: {}", stored_val))
        })
    } else {
        Err(EncoderError::Decode(format!(
            "Unexpected tag for u8: {}",
            tag
        )))
    }
}
/// Decodes a `u16` value from a tag and buffer.
/// Used internally for compact integer decoding.
#[inline(never)]
fn decode_u16_from_tag(tag: u8, reader: &mut Bytes) -> Result<u16> {
    if (TAG_ZERO..=TAG_U8_127).contains(&tag) {
        Ok((tag - TAG_ZERO) as u16)
    } else if tag == TAG_U8 {
        if reader.remaining() < 1 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u8() as u16 + 128)
    } else if tag == TAG_U16 {
        if reader.remaining() < 2 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u16_le())
    } else {
        Err(EncoderError::Decode(format!(
            "Unexpected tag for u16: {}",
            tag
        )))
    }
}
/// Decodes a `u32` value from a tag and buffer.
/// Used internally for compact integer decoding.
#[inline]
fn decode_u32_from_tag(tag: u8, reader: &mut Bytes) -> Result<u32> {
    if (TAG_ZERO..=TAG_U8_127).contains(&tag) {
        Ok((tag - TAG_ZERO) as u32)
    } else if tag == TAG_U8 {
        if reader.remaining() < 1 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u8() as u32 + 128)
    } else if tag == TAG_U16 {
        if reader.remaining() < 2 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u16_le() as u32)
    } else if tag == TAG_U32 {
        if reader.remaining() < 4 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u32_le())
    } else {
        Err(EncoderError::Decode(format!(
            "Unexpected tag for u32: {}",
            tag
        )))
    }
}
/// Decodes a `u64` value from a tag and buffer.
/// Used internally for compact integer decoding.
#[inline]
fn decode_u64_from_tag(tag: u8, reader: &mut Bytes) -> Result<u64> {
    if (TAG_ZERO..=TAG_U8_127).contains(&tag) {
        Ok((tag - TAG_ZERO) as u64)
    } else if tag == TAG_U8 {
        if reader.remaining() < 1 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u8() as u64 + 128)
    } else if tag == TAG_U16 {
        if reader.remaining() < 2 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u16_le() as u64)
    } else if tag == TAG_U32 {
        if reader.remaining() < 4 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u32_le() as u64)
    } else if tag == TAG_U64 {
        if reader.remaining() < 8 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u64_le())
    } else {
        Err(EncoderError::Decode(format!(
            "Unexpected tag for u64: {}",
            tag
        )))
    }
}
/// Decodes a `u128` value from a tag and buffer.
/// Used internally for compact integer decoding.
#[inline(never)]
fn decode_u128_from_tag(tag: u8, reader: &mut Bytes) -> Result<u128> {
    if (TAG_ZERO..=TAG_U8_127).contains(&tag) {
        Ok((tag - TAG_ZERO) as u128)
    } else if tag == TAG_U8 {
        if reader.remaining() < 1 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u8() as u128 + 128)
    } else if tag == TAG_U16 {
        if reader.remaining() < 2 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u16_le() as u128)
    } else if tag == TAG_U32 {
        if reader.remaining() < 4 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u32_le() as u128)
    } else if tag == TAG_U64 {
        if reader.remaining() < 8 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u64_le() as u128)
    } else if tag == TAG_U128 {
        if reader.remaining() < 16 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u128_le())
    } else {
        Err(EncoderError::Decode(format!(
            "Unexpected tag for u128: {}",
            tag
        )))
    }
}

// --- Unsigned integer types ---
/// Encodes unsigned integers using a compact variable-length format.
///
/// - Values 0/1 are encoded as `TAG_ZERO`/`TAG_ONE` (1 byte)
/// - 2..=127 are encoded as a single tag byte (1 byte)
/// - Larger values use `TAG_U8`, `TAG_U16`, `TAG_U32`, `TAG_U64`, or `TAG_U128` with the value in little-endian
/// - The encoding is stable and compatible across platforms
impl Encoder for u8 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self <= 127 {
            writer.put_u8(TAG_ZERO + *self);
        } else {
            writer.put_u8(TAG_U8);
            writer.put_u8(*self - 128);
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
/// Decodes a `u8` from the compact format.
impl Decoder for u8 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        decode_u8_from_tag(tag, reader)
    }
}
/// See `u8` for format details.
impl Encoder for u16 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self <= 127 {
            writer.put_u8(TAG_ZERO + (*self as u8));
        } else if *self <= 255 + 128 {
            writer.put_u8(TAG_U8);
            writer.put_u8((*self - 128) as u8);
        } else {
            writer.put_u8(TAG_U16);
            writer.put_u16_le(*self);
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for u16 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        decode_u16_from_tag(tag, reader)
    }
}
impl Encoder for u32 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self <= 127 {
            writer.put_u8(TAG_ZERO + (*self as u8));
        } else if *self <= 255 + 128 {
            writer.put_u8(TAG_U8);
            writer.put_u8((*self - 128) as u8);
        } else if *self <= 65535 {
            writer.put_u8(TAG_U16);
            writer.put_u16_le(*self as u16);
        } else {
            writer.put_u8(TAG_U32);
            writer.put_u32_le(*self);
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for u32 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        decode_u32_from_tag(tag, reader)
    }
}
impl Encoder for u64 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self <= 127 {
            writer.put_u8(TAG_ZERO + (*self as u8));
        } else if *self <= 255 + 128 {
            writer.put_u8(TAG_U8);
            writer.put_u8((*self - 128) as u8);
        } else if *self <= 65535 {
            writer.put_u8(TAG_U16);
            writer.put_u16_le(*self as u16);
        } else if *self <= 4294967295 {
            writer.put_u8(TAG_U32);
            writer.put_u32_le(*self as u32);
        } else {
            writer.put_u8(TAG_U64);
            writer.put_u64_le(*self);
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for u64 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        decode_u64_from_tag(tag, reader)
    }
}
impl Encoder for u128 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self <= 127 {
            writer.put_u8(TAG_ZERO + (*self as u8));
        } else if *self <= 255 + 128 {
            writer.put_u8(TAG_U8);
            writer.put_u8((*self - 128) as u8);
        } else if *self <= 65535 {
            writer.put_u8(TAG_U16);
            writer.put_u16_le(*self as u16);
        } else if *self <= 4294967295 {
            writer.put_u8(TAG_U32);
            writer.put_u32_le(*self as u32);
        } else if *self <= 18446744073709551615 {
            writer.put_u8(TAG_U64);
            writer.put_u64_le(*self as u64);
        } else {
            writer.put_u8(TAG_U128);
            writer.put_u128_le(*self);
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for u128 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        decode_u128_from_tag(tag, reader)
    }
}
/// Encodes `usize` using the platform's pointer width, but always as a portable integer format.
impl Encoder for usize {
    #[inline]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if usize::BITS == u64::BITS {
            let v = *self as u64;
            v.encode(writer)
        } else if usize::BITS == u32::BITS {
            let v = *self as u32;
            v.encode(writer)
        } else if usize::BITS == u16::BITS {
            let v = *self as u16;
            v.encode(writer)
        } else {
            let v = *self as u128;
            v.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for usize {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if usize::BITS == u64::BITS {
            Ok(decode_u64_from_tag(tag, reader)? as usize)
        } else if usize::BITS == u32::BITS {
            Ok(decode_u32_from_tag(tag, reader)? as usize)
        } else if usize::BITS == u16::BITS {
            Ok(decode_u16_from_tag(tag, reader)? as usize)
        } else {
            Ok(decode_u128_from_tag(tag, reader)? as usize)
        }
    }
}

// --- Signed integer types (bit-inverted encoding) ---
/// Encodes signed integers using bit-inverted encoding for negative values.
///
/// - Non-negative values (>= 0) are encoded as unsigned integers
/// - Negative values use `TAG_NEGATIVE` and bit-inverted encoding
impl Encoder for i8 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self >= 0 {
            (*self as u8).encode(writer)
        } else {
            writer.put_u8(TAG_NEGATIVE);
            let inv = !(*self as u8);
            inv.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
/// Decodes a `i8` from the bit-inverted encoding.
///
/// # Errors
/// Returns an error if the tag is not valid for an `i8`.
impl Decoder for i8 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NEGATIVE => {
                let inv = u8::decode(reader)?;
                Ok(!inv as i8)
            }
            t => {
                let v = decode_u8_from_tag(t, reader)?;
                if v > i8::MAX as u8 {
                    return Err(EncoderError::Decode(format!(
                        "Value {} too large for i8",
                        v
                    )));
                }
                Ok(v as i8)
            }
        }
    }
}
// i16
impl Encoder for i16 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self >= 0 {
            (*self as u16).encode(writer)
        } else {
            writer.put_u8(TAG_NEGATIVE);
            let inv = !(*self as u16);
            inv.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for i16 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NEGATIVE => {
                let inv = u16::decode(reader)?;
                Ok(!inv as i16)
            }
            t => {
                let v = decode_u16_from_tag(t, reader)?;
                if v > i16::MAX as u16 {
                    return Err(EncoderError::Decode(format!(
                        "Value {} too large for i16",
                        v
                    )));
                }
                Ok(v as i16)
            }
        }
    }
}
// i32
impl Encoder for i32 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self >= 0 {
            (*self as u32).encode(writer)
        } else {
            writer.put_u8(TAG_NEGATIVE);
            let inv = !(*self as u32);
            inv.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for i32 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NEGATIVE => {
                let inv = u32::decode(reader)?;
                Ok(!inv as i32)
            }
            t => {
                let v = decode_u32_from_tag(t, reader)?;
                if v > i32::MAX as u32 {
                    return Err(EncoderError::Decode(format!(
                        "Value {} too large for i32",
                        v
                    )));
                }
                Ok(v as i32)
            }
        }
    }
}
// i64
impl Encoder for i64 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self >= 0 {
            (*self as u64).encode(writer)
        } else {
            writer.put_u8(TAG_NEGATIVE);
            let inv = !(*self as u64);
            inv.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for i64 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NEGATIVE => {
                let inv = u64::decode(reader)?;
                Ok(!inv as i64)
            }
            t => {
                let v = decode_u64_from_tag(t, reader)?;
                if v > i64::MAX as u64 {
                    return Err(EncoderError::Decode(format!(
                        "Value {} too large for i64",
                        v
                    )));
                }
                Ok(v as i64)
            }
        }
    }
}
// i128
impl Encoder for i128 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if *self >= 0 {
            (*self as u128).encode(writer)
        } else {
            writer.put_u8(TAG_NEGATIVE);
            let inv = !(*self as u128);
            inv.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for i128 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NEGATIVE => {
                let inv = u128::decode(reader)?;
                Ok(!inv as i128)
            }
            t => {
                let v = decode_u128_from_tag(t, reader)?;
                if v > i128::MAX as u128 {
                    return Err(EncoderError::Decode(format!(
                        "Value {} too large for i128",
                        v
                    )));
                }
                Ok(v as i128)
            }
        }
    }
}
// isize
impl Encoder for isize {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if usize::BITS == u64::BITS {
            let v = *self as i64;
            v.encode(writer)
        } else if usize::BITS == u32::BITS {
            let v = *self as i32;
            v.encode(writer)
        } else if usize::BITS == u16::BITS {
            let v = *self as i16;
            v.encode(writer)
        } else {
            let v = *self as i128;
            v.encode(writer)
        }
    }

    fn is_default(&self) -> bool {
        *self == 0
    }
}
impl Decoder for isize {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        if usize::BITS == u64::BITS {
            Ok(i64::decode(reader)? as isize)
        } else if usize::BITS == u32::BITS {
            Ok(i32::decode(reader)? as isize)
        } else if usize::BITS == u16::BITS {
            Ok(i16::decode(reader)? as isize)
        } else {
            Ok(i128::decode(reader)? as isize)
        }
    }
}

// --- f32/f64 ---
/// Encodes an `f32` as a tag and 4 bytes (little-endian IEEE 754).
impl Encoder for f32 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_F32);
        writer.put_f32_le(*self);
        Ok(())
    }

    /// Packs an `f32` as 4 bytes (little-endian IEEE 754) without a type tag.
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_f32_le(*self);
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0.0
    }
}
/// Decodes an `f32` from either 4 or 8 bytes (accepts f64 for compatibility with precision loss).
impl Decoder for f32 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag == TAG_F32 {
            if reader.remaining() < 4 {
                return Err(EncoderError::InsufficientData);
            }
            let mut bytes = [0u8; 4];
            reader.copy_to_slice(&mut bytes);
            Ok(f32::from_le_bytes(bytes))
        } else if tag == TAG_F64 {
            if reader.remaining() < 8 {
                return Err(EncoderError::InsufficientData);
            }
            let mut bytes = [0u8; 8];
            reader.copy_to_slice(&mut bytes);
            Ok(f64::from_le_bytes(bytes) as f32)
        } else {
            Err(EncoderError::Decode(format!(
                "Expected f32/f64 tag ({} or {}), got {}",
                TAG_F32, TAG_F64, tag
            )))
        }
    }

    /// Unpacks an `f32` from 4 bytes (little-endian IEEE 754) without expecting a type tag.
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() < 4 {
            return Err(EncoderError::InsufficientData);
        }
        let mut bytes = [0u8; 4];
        reader.copy_to_slice(&mut bytes);
        Ok(f32::from_le_bytes(bytes))
    }
}
/// Encodes an `f64` as a tag and 8 bytes (little-endian IEEE 754).
impl Encoder for f64 {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_F64);
        writer.put_f64_le(*self);
        Ok(())
    }

    /// Packs an `f64` as 8 bytes (little-endian IEEE 754) without a type tag.
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_f64_le(*self);
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == 0.0
    }
}
/// Decodes an `f64` from 8 bytes (f32 cross-decoding not supported).
impl Decoder for f64 {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag == TAG_F64 {
            if reader.remaining() < 8 {
                return Err(EncoderError::InsufficientData);
            }
            let mut bytes = [0u8; 8];
            reader.copy_to_slice(&mut bytes);
            Ok(f64::from_le_bytes(bytes))
        } else {
            Err(EncoderError::Decode(format!(
                "Expected f64 tag ({}), got {}. f32 to f64 cross-decoding is not supported due to precision concerns.",
                TAG_F64, tag
            )))
        }
    }

    /// Unpacks an `f64` from 8 bytes (little-endian IEEE 754) without expecting a type tag.
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() < 8 {
            return Err(EncoderError::InsufficientData);
        }
        let mut bytes = [0u8; 8];
        reader.copy_to_slice(&mut bytes);
        Ok(f64::from_le_bytes(bytes))
    }
}

// --- String ---
/// Encodes a `String` as UTF-8 with a length prefix (short strings use a single tag byte).
impl Encoder for String {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        let len = self.len();
        let max_short = (TAG_STRING_LONG - TAG_STRING_BASE - 1) as usize;
        if len <= max_short {
            let tag = TAG_STRING_BASE + len as u8; // 9..=29
            writer.put_u8(tag);
            writer.put_slice(self.as_bytes());
        } else {
            writer.put_u8(TAG_STRING_LONG);
            len.encode(writer)?;
            writer.put_slice(self.as_bytes());
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
/// Decodes a `String` from the senax binary format.
impl Decoder for String {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        let len = if (TAG_STRING_BASE..TAG_STRING_LONG).contains(&tag) {
            (tag - TAG_STRING_BASE) as usize
        } else if tag == TAG_STRING_LONG {
            usize::decode(reader)?
        } else {
            return Err(EncoderError::Decode(format!(
                "Expected String tag ({}..={}), got {}",
                TAG_STRING_BASE, TAG_STRING_LONG, tag
            )));
        };
        if reader.remaining() < len {
            return Err(EncoderError::InsufficientData);
        }
        let mut bytes = vec![0u8; len];
        if len > 0 {
            reader.copy_to_slice(&mut bytes);
        }
        String::from_utf8(bytes).map_err(|e| EncoderError::Decode(e.to_string()))
    }
}

// --- Option ---
/// Encodes an `Option<T>` as a tag byte followed by the value if present.
impl<T: Encoder> Encoder for Option<T> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        match self {
            Some(value) => {
                writer.put_u8(TAG_SOME);
                value.encode(writer)
            }
            None => {
                writer.put_u8(TAG_NONE);
                Ok(())
            }
        }
    }

    fn is_default(&self) -> bool {
        self.is_none()
    }
}
/// Decodes an `Option<T>` from the senax binary format.
impl<T: Decoder> Decoder for Option<T> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData); // Not even a tag
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NONE => Ok(None),
            TAG_SOME => {
                if reader.remaining() == 0 {
                    // Check before T::decode if only TAG_SOME was present
                    return Err(EncoderError::InsufficientData);
                }
                Ok(Some(T::decode(reader)?))
            }
            other => Err(EncoderError::Decode(format!(
                "Expected Option tag ({} or {}), got {}",
                TAG_NONE, TAG_SOME, other
            ))),
        }
    }
}

// --- Vec<T> ---
/// Encodes a `Vec<T>` as a length-prefixed sequence. `Vec<u8>` is optimized as binary.
impl<T: Encoder + 'static> Encoder for Vec<T> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>() {
            // Safety: We've verified T is u8, so this cast is safe
            let vec_u8 = unsafe { &*(self as *const Vec<T> as *const Vec<u8>) };
            encode_vec_u8(vec_u8, writer)
        } else {
            encode_vec_length(self.len(), writer)?;
            for item in self {
                item.encode(writer)?;
            }
            Ok(())
        }
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
/// Decodes a `Vec<T>` from the senax binary format.
impl<T: Decoder + 'static> Decoder for Vec<T> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();

        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>() {
            if tag == TAG_BINARY {
                let vec_u8 = decode_vec_u8(reader)?;
                // Safety: We've verified T is u8, so this cast is safe
                let ptr = vec_u8.as_ptr() as *mut T;
                let len = vec_u8.len();
                let cap = vec_u8.capacity();
                std::mem::forget(vec_u8);
                unsafe { Ok(Vec::from_raw_parts(ptr, len, cap)) }
            } else {
                Err(EncoderError::Decode(format!(
                    "Expected Vec<u8> tag ({}), got {}",
                    TAG_BINARY, tag
                )))
            }
        } else {
            let len = decode_vec_length(tag, reader)?;
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len {
                vec.push(T::decode(reader)?);
            }
            Ok(vec)
        }
    }
}

// --- Array ---
/// Encodes a fixed-size array as a length-prefixed sequence.
impl<T: Encoder, const N: usize> Encoder for [T; N] {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(N, writer)?;
        for item in self {
            item.encode(writer)?;
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.iter().all(|item| item.is_default())
    }
}
/// Decodes a fixed-size array from the senax binary format.
impl<T: Decoder, const N: usize> Decoder for [T; N] {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        let len = decode_vec_length(tag, reader)?;
        if len != N {
            return Err(EncoderError::Decode(format!(
                "Array length mismatch: expected {}, got {}",
                N, len
            )));
        }
        let mut array = Vec::with_capacity(N);
        for _ in 0..N {
            array.push(T::decode(reader)?);
        }
        array
            .try_into()
            .map_err(|_| EncoderError::Decode("Failed to convert Vec to array".to_string()))
    }
}

// --- Tuple ---
/// Implements encoding/decoding for tuples up to 10 elements.
///
/// Each tuple is encoded as a length-prefixed sequence of its elements.
macro_rules! impl_tuple {
    () => {
impl Encoder for () {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
                writer.put_u8(TAG_TUPLE);
                0usize.encode(writer)?;
        Ok(())
    }

    fn is_default(&self) -> bool {
        true
    }
}
impl Decoder for () {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
                if tag != TAG_TUPLE {
                    return Err(EncoderError::Decode(format!("Expected Tuple tag ({}), got {}", TAG_TUPLE, tag)));
                }
                let len = usize::decode(reader)?;
                if len != 0 {
                    return Err(EncoderError::Decode(format!("Expected 0-tuple but got {}-tuple", len)));
        }
        Ok(())
    }
}
    };
    ($($T:ident : $idx:tt),+) => {
        impl<$($T: Encoder),+> Encoder for ($($T,)+) {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
                writer.put_u8(TAG_TUPLE);
                let count = count_args!($($T),+);
                count.encode(writer)?;
                $(
                    self.$idx.encode(writer)?;
                )+
        Ok(())
    }

    fn is_default(&self) -> bool {
        $(self.$idx.is_default())&&+
    }
}
        impl<$($T: Decoder),+> Decoder for ($($T,)+) {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
                if tag != TAG_TUPLE {
                    return Err(EncoderError::Decode(format!("Expected Tuple tag ({}), got {}", TAG_TUPLE, tag)));
                }
                let len = usize::decode(reader)?;
                let expected_len = count_args!($($T),+);
                if len != expected_len {
                    return Err(EncoderError::Decode(format!("Expected {}-tuple but got {}-tuple", expected_len, len)));
                }
                Ok(($(
                    $T::decode(reader)?,
                )+))
            }
        }
    };
}

macro_rules! count_args {
    () => { 0 };
    ($head:ident $(, $tail:ident)*) => { 1 + count_args!($($tail),*) };
}

// Generate tuple implementations for 0 to 12 elements
impl_tuple!();
impl_tuple!(T0: 0);
impl_tuple!(T0: 0, T1: 1);
impl_tuple!(T0: 0, T1: 1, T2: 2);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7, T8: 8);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7, T8: 8, T9: 9);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7, T8: 8, T9: 9, T10: 10);
impl_tuple!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7, T8: 8, T9: 9, T10: 10, T11: 11);

// --- Map (HashMap) ---
/// Encodes a map as a length-prefixed sequence of key-value pairs.
impl<K: Encoder, V: Encoder> Encoder for HashMap<K, V> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_MAP);
        let len = self.len();
        len.encode(writer)?;
        for (k, v) in self {
            k.encode(writer)?;
            v.encode(writer)?;
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
/// Decodes a map from the senax binary format.
impl<K: Decoder + Eq + std::hash::Hash, V: Decoder> Decoder for HashMap<K, V> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_MAP {
            return Err(EncoderError::Decode(format!(
                "Expected Map tag ({}), got {}",
                TAG_MAP, tag
            )));
        }
        let len = usize::decode(reader)?;
        let mut map = HashMap::with_capacity(len);
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

/// Writes a `u32` in little-endian format without a tag.
///
/// This is used internally for struct/enum field/variant IDs.
pub fn write_u32_le(writer: &mut BytesMut, value: u32) -> Result<()> {
    writer.put_u32_le(value);
    Ok(())
}

/// Reads a `u32` in little-endian format without a tag.
///
/// This is used internally for struct/enum field/variant IDs.
pub fn read_u32_le(reader: &mut Bytes) -> Result<u32> {
    if reader.remaining() < 4 {
        return Err(EncoderError::InsufficientData);
    }
    Ok(reader.get_u32_le())
}

/// Writes a `u64` in little-endian format without a tag.
///
/// This is used internally for struct/enum field/variant IDs.
pub fn write_u64_le(writer: &mut BytesMut, value: u64) -> Result<()> {
    writer.put_u64_le(value);
    Ok(())
}

/// Reads a `u64` in little-endian format without a tag.
///
/// This is used internally for struct/enum field/variant IDs.
pub fn read_u64_le(reader: &mut Bytes) -> Result<u64> {
    if reader.remaining() < 8 {
        return Err(EncoderError::InsufficientData);
    }
    Ok(reader.get_u64_le())
}

/// Skips a value of any type in the senax binary format.
///
/// This is used for forward/backward compatibility when unknown fields/variants are encountered.
///
/// # Errors
/// Returns an error if the value cannot be skipped (e.g., insufficient data).
pub fn skip_value(reader: &mut Bytes) -> Result<()> {
    if reader.remaining() == 0 {
        return Err(EncoderError::InsufficientData);
    }
    let tag = reader.get_u8();
    match tag {
        TAG_ZERO..=TAG_U8_127 => Ok(()),
        TAG_U8 => {
            if reader.remaining() < 1 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(1);
            Ok(())
        }
        TAG_U16 => {
            if reader.remaining() < 2 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(2);
            Ok(())
        }
        TAG_U32 => {
            if reader.remaining() < 4 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(4);
            Ok(())
        }
        TAG_U64 => {
            if reader.remaining() < 8 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(8);
            Ok(())
        }
        TAG_U128 => {
            if reader.remaining() < 16 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(16);
            Ok(())
        }
        TAG_F32 => {
            if reader.remaining() < 4 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(4);
            Ok(())
        }
        TAG_F64 => {
            if reader.remaining() < 8 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(8);
            Ok(())
        }
        TAG_STRING_BASE..=TAG_STRING_LONG => {
            let len = if tag < TAG_STRING_LONG {
                (tag - TAG_STRING_BASE) as usize
            } else {
                usize::decode(reader)?
            };
            if reader.remaining() < len {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(len);
            Ok(())
        }
        TAG_BINARY => {
            let len = usize::decode(reader)?;
            if reader.remaining() < len {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(len);
            Ok(())
        }
        TAG_ARRAY_VEC_SET_BASE..=TAG_ARRAY_VEC_SET_LONG => {
            let len = if tag < TAG_ARRAY_VEC_SET_LONG {
                (tag - TAG_ARRAY_VEC_SET_BASE) as usize
            } else {
                usize::decode(reader)?
            };
            for _ in 0..len {
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_STRUCT_UNIT => Ok(()),
        TAG_STRUCT_NAMED => {
            loop {
                let field_id = read_field_id_optimized(reader)?;
                if field_id == 0 {
                    break;
                }
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_STRUCT_UNNAMED => {
            let field_count = usize::decode(reader)?;
            for _ in 0..field_count {
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_ENUM => {
            let _variant_id = read_field_id_optimized(reader)?;
            Ok(())
        }
        TAG_ENUM_NAMED => {
            let _variant_id = read_field_id_optimized(reader)?;
            loop {
                let field_id = read_field_id_optimized(reader)?;
                if field_id == 0 {
                    break;
                }
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_ENUM_UNNAMED => {
            let _variant_id = read_field_id_optimized(reader)?;
            let field_count = usize::decode(reader)?;
            for _ in 0..field_count {
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_TUPLE => {
            let len = usize::decode(reader)?;
            for _ in 0..len {
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_MAP => {
            let len = usize::decode(reader)?;
            for _ in 0..len {
                skip_value(reader)?; // key
                skip_value(reader)?; // value
            }
            Ok(())
        }
        TAG_CHRONO_DATETIME => {
            if reader.remaining() < 12 {
                return Err(EncoderError::InsufficientData);
            } // Approximation for i64 + u32, could be more precise
            let _timestamp_seconds = i64::decode(reader)?;
            let _timestamp_nanos = u32::decode(reader)?;
            Ok(())
        }
        TAG_CHRONO_NAIVE_DATE => {
            if reader.remaining() < 8 {
                return Err(EncoderError::InsufficientData);
            } // Approximation for i64
            let _days_from_epoch = i64::decode(reader)?;
            Ok(())
        }
        TAG_CHRONO_NAIVE_TIME => {
            if reader.remaining() < 8 {
                return Err(EncoderError::InsufficientData);
            } // Approximation for u32 + u32
            let _seconds_from_midnight = u32::decode(reader)?;
            let _nanoseconds = u32::decode(reader)?;
            Ok(())
        }
        TAG_DECIMAL => {
            if reader.remaining() < 20 {
                return Err(EncoderError::InsufficientData);
            } // Approximation for i128 + u32
            let _mantissa = i128::decode(reader)?;
            let _scale = u32::decode(reader)?;
            Ok(())
        }
        TAG_UUID => {
            // Covers ULID as well
            if reader.remaining() < 16 {
                return Err(EncoderError::InsufficientData);
            }
            reader.advance(16);
            Ok(())
        }
        TAG_JSON_NULL => Ok(()),
        TAG_JSON_BOOL => Ok(()),
        TAG_JSON_NUMBER => {
            // Number has type marker + actual number
            if reader.remaining() == 0 {
                return Err(EncoderError::InsufficientData);
            }
            let number_type = reader.get_u8();
            match number_type {
                0 => {
                    u64::decode(reader)?;
                }
                1 => {
                    i64::decode(reader)?;
                }
                2 => {
                    f64::decode(reader)?;
                }
                _ => {
                    return Err(EncoderError::Decode(format!(
                        "Invalid JSON Number type marker: {}",
                        number_type
                    )));
                }
            }
            Ok(())
        }
        TAG_JSON_STRING => {
            // String uses regular string encoding
            String::decode(reader)?;
            Ok(())
        }
        TAG_JSON_ARRAY => {
            let len = usize::decode(reader)?;
            for _ in 0..len {
                skip_value(reader)?;
            }
            Ok(())
        }
        TAG_JSON_OBJECT => {
            let len = usize::decode(reader)?;
            for _ in 0..len {
                String::decode(reader)?; // key
                skip_value(reader)?; // value
            }
            Ok(())
        }
        TAG_NONE | TAG_SOME => {
            // These should have been handled by Option<T> decode or skip_value for T
            // For TAG_NONE, it's fine. For TAG_SOME, we need to skip the inner value.
            if tag == TAG_SOME {
                skip_value(reader)?;
            }
            Ok(())
        }
        _ => Err(EncoderError::Decode(format!(
            "skip_value: unknown or unhandled tag {}",
            tag
        ))),
    }
}

// --- HashSet, BTreeSet, IndexSet ---
/// Encodes a set as a length-prefixed sequence of elements.
impl<T: Encoder + Eq + std::hash::Hash> Encoder for HashSet<T> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.encode(writer)?;
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
/// Decodes a set from the senax binary format.
impl<T: Decoder + Eq + std::hash::Hash + 'static> Decoder for HashSet<T> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::decode(reader)?;
        Ok(vec.into_iter().collect())
    }
}
// --- BTreeSet ---
impl<T: Encoder + Ord> Encoder for BTreeSet<T> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.encode(writer)?;
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
impl<T: Decoder + Ord + 'static> Decoder for BTreeSet<T> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::decode(reader)?;
        Ok(vec.into_iter().collect())
    }
}
// --- BTreeMap ---
impl<K: Encoder + Ord, V: Encoder> Encoder for BTreeMap<K, V> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_MAP);
        let len = self.len();
        len.encode(writer)?;
        for (k, v) in self {
            k.encode(writer)?;
            v.encode(writer)?;
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
impl<K: Decoder + Ord, V: Decoder> Decoder for BTreeMap<K, V> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_MAP {
            return Err(EncoderError::Decode(format!(
                "Expected Map tag ({}), got {}",
                TAG_MAP, tag
            )));
        }
        let len = usize::decode(reader)?;
        let mut map = BTreeMap::new();
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

// --- Bytes ---
impl Encoder for Bytes {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_BINARY);
        let len = self.len();
        len.encode(writer)?;
        writer.put_slice(self);
        Ok(())
    }

    fn is_default(&self) -> bool {
        self.is_empty()
    }
}
impl Decoder for Bytes {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        let len = if tag == TAG_BINARY {
            usize::decode(reader)?
        } else if (TAG_STRING_BASE..TAG_STRING_LONG).contains(&tag) {
            (tag - TAG_STRING_BASE) as usize
        } else if tag == TAG_STRING_LONG {
            usize::decode(reader)?
        } else {
            return Err(EncoderError::Decode(format!(
                "Expected Bytes tag ({} or {}..={}), got {}",
                TAG_BINARY, TAG_STRING_BASE, TAG_STRING_LONG, tag
            )));
        };

        if reader.remaining() < len {
            return Err(EncoderError::InsufficientData);
        }

        Ok(reader.split_to(len))
    }
}

// --- Arc<T> ---
/// Encodes an `Arc<T>` by encoding the inner value.
impl<T: Encoder> Encoder for Arc<T> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        (**self).encode(writer)
    }

    fn is_default(&self) -> bool {
        T::is_default(self)
    }
}

/// Decodes an `Arc<T>` by decoding the inner value and wrapping it in an Arc.
impl<T: Decoder> Decoder for Arc<T> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        Ok(Arc::new(T::decode(reader)?))
    }
}

/// Writes a `u64` in little-endian format without a tag.
///
/// This is used internally for struct/enum field/variant IDs.
pub fn write_field_id_optimized(writer: &mut BytesMut, field_id: u64) -> Result<()> {
    if field_id == 0 {
        // Terminator
        writer.put_u8(0);
    } else if (1..=250).contains(&field_id) {
        // Small field ID: write as u8
        writer.put_u8(field_id as u8);
    } else {
        // Large field ID: write 255 marker then u64
        writer.put_u8(255);
        writer.put_u64_le(field_id);
    }
    Ok(())
}

/// Reads a field ID using optimized encoding.
///
/// Returns Ok(0) for terminator, Ok(field_id) for valid field ID.
pub fn read_field_id_optimized(reader: &mut Bytes) -> Result<u64> {
    if reader.remaining() < 1 {
        return Err(EncoderError::InsufficientData);
    }

    let first_byte = reader.get_u8();

    if first_byte == 0 {
        // Terminator
        Ok(0)
    } else if first_byte == 255 {
        // Large field ID follows
        if reader.remaining() < 8 {
            return Err(EncoderError::InsufficientData);
        }
        Ok(reader.get_u64_le())
    } else {
        // Small field ID
        Ok(first_byte as u64)
    }
}

/// Implementation for references - delegates to the referenced value
impl<T: Encoder> Encoder for &T {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        (*self).encode(writer)
    }

    fn is_default(&self) -> bool {
        (*self).is_default()
    }
}

// --- Box<T> ---
/// Encodes a `Box<T>` by encoding the inner value.
impl<T: Encoder> Encoder for Box<T> {
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        (**self).encode(writer)
    }

    fn is_default(&self) -> bool {
        T::is_default(self)
    }
}

/// Decodes a `Box<T>` by decoding the inner value and wrapping it in a Box.
impl<T: Decoder> Decoder for Box<T> {
    fn decode(reader: &mut Bytes) -> Result<Self> {
        Ok(Box::new(T::decode(reader)?))
    }
}

/// Encodes a `Vec<u8>` using the optimized binary format.
fn encode_vec_u8(vec: &[u8], writer: &mut BytesMut) -> Result<()> {
    writer.put_u8(TAG_BINARY);
    let len = vec.len();
    len.encode(writer)?;
    let bytes = unsafe { std::slice::from_raw_parts(vec.as_ptr(), vec.len()) };
    writer.put_slice(bytes);
    Ok(())
}

/// Decodes a `Vec<u8>` from the optimized binary format.
fn decode_vec_u8(reader: &mut Bytes) -> Result<Vec<u8>> {
    let len = usize::decode(reader)?;
    let mut vec = vec![0u8; len];
    if len > 0 {
        reader.copy_to_slice(&mut vec);
    }
    Ok(vec)
}

/// Encodes the length for array/vec/set format.
#[inline(never)]
pub(crate) fn encode_vec_length(len: usize, writer: &mut BytesMut) -> Result<()> {
    let max_short = (TAG_ARRAY_VEC_SET_LONG - TAG_ARRAY_VEC_SET_BASE - 1) as usize;
    if len <= max_short {
        let tag = TAG_ARRAY_VEC_SET_BASE + len as u8;
        writer.put_u8(tag);
    } else {
        writer.put_u8(TAG_ARRAY_VEC_SET_LONG);
        len.encode(writer)?;
    }
    Ok(())
}

/// Decodes the length for array/vec/set format.
#[inline(never)]
pub(crate) fn decode_vec_length(tag: u8, reader: &mut Bytes) -> Result<usize> {
    if (TAG_ARRAY_VEC_SET_BASE..TAG_ARRAY_VEC_SET_LONG).contains(&tag) {
        Ok((tag - TAG_ARRAY_VEC_SET_BASE) as usize)
    } else if tag == TAG_ARRAY_VEC_SET_LONG {
        usize::decode(reader)
    } else {
        Err(EncoderError::Decode(format!(
            "Expected Vec tag ({}..={}), got {}",
            TAG_ARRAY_VEC_SET_BASE, TAG_ARRAY_VEC_SET_LONG, tag
        )))
    }
}
