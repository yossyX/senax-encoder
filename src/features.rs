#[cfg(feature = "ahash")]
use ahash::{AHashMap, AHashSet};
#[allow(unused_imports)]
use bytes::{Buf, BufMut, Bytes, BytesMut};
#[cfg(feature = "chrono")]
use chrono::{DateTime, Local, NaiveDate, NaiveTime, Timelike, Utc};
#[cfg(feature = "fxhash")]
use fxhash::{FxHashMap, FxHashSet};
#[cfg(feature = "indexmap")]
use indexmap::{IndexMap, IndexSet};
#[cfg(feature = "rust_decimal")]
use rust_decimal::Decimal;
#[cfg(feature = "serde_json")]
use serde_json::{Map, Number, Value};
#[cfg(feature = "smol_str")]
use smol_str::SmolStr;
#[cfg(feature = "ulid")]
use ulid::Ulid;
#[cfg(feature = "uuid")]
use uuid::Uuid;

#[allow(unused_imports)]
use crate::core::*;
#[allow(unused_imports)]
use crate::*;

// --- IndexSet ---
#[cfg(feature = "indexmap")]
impl<T: Encoder + Eq + std::hash::Hash> Encoder for IndexSet<T> {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.encode(writer)?;
        }
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.pack(writer)?;
        }
        Ok(())
    }
}
#[cfg(feature = "indexmap")]
impl<T: Decoder + Eq + std::hash::Hash + 'static> Decoder for IndexSet<T> {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::decode(reader)?;
        Ok(vec.into_iter().collect())
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::unpack(reader)?;
        Ok(vec.into_iter().collect())
    }
}

// --- IndexMap ---
#[cfg(feature = "indexmap")]
impl<K: Encoder + Eq + std::hash::Hash, V: Encoder> Encoder for IndexMap<K, V> {
    #[cfg(feature = "encode")]
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

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_MAP);
        let len = self.len();
        len.pack(writer)?;
        for (k, v) in self {
            k.pack(writer)?;
            v.pack(writer)?;
        }
        Ok(())
    }
}
#[cfg(feature = "indexmap")]
impl<K: Decoder + Eq + std::hash::Hash, V: Decoder> Decoder for IndexMap<K, V> {
    #[cfg(feature = "encode")]
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
        let mut map = IndexMap::with_capacity(len);
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
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
        let len = usize::unpack(reader)?;
        let mut map = IndexMap::with_capacity(len);
        for _ in 0..len {
            let k = K::unpack(reader)?;
            let v = V::unpack(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

// --- DateTime<Utc> ---
/// Encodes a `chrono::DateTime<Utc>` as seconds and nanoseconds since the Unix epoch.
#[cfg(feature = "chrono")]
impl Encoder for DateTime<Utc> {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_CHRONO_DATETIME);
        let timestamp_seconds = self.timestamp();
        let timestamp_nanos = self.timestamp_subsec_nanos();
        timestamp_seconds.encode(writer)?;
        timestamp_nanos.encode(writer)?;
        Ok(())
    }

    /// Packs a `chrono::DateTime<Utc>` as seconds and nanoseconds without a type tag.
    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        if *self == DateTime::<Utc>::default() {
            writer.put_u8(TAG_NONE);
        } else {
            writer.put_u8(TAG_CHRONO_DATETIME);
            let timestamp_seconds = self.timestamp();
            let timestamp_nanos = self.timestamp_subsec_nanos();
            timestamp_seconds.pack(writer)?;
            timestamp_nanos.pack(writer)?;
        }
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == DateTime::<Utc>::default()
    }
}
/// Decodes a `chrono::DateTime<Utc>` from the senax binary format.
#[cfg(feature = "chrono")]
impl Decoder for DateTime<Utc> {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_CHRONO_DATETIME {
            return Err(EncoderError::Decode(format!(
                "Expected DateTime<Utc> tag ({}), got {}",
                TAG_CHRONO_DATETIME, tag
            )));
        }
        let timestamp_seconds = i64::decode(reader)?;
        let timestamp_nanos = u32::decode(reader)?;
        DateTime::from_timestamp(timestamp_seconds, timestamp_nanos).ok_or_else(|| {
            EncoderError::Decode(format!(
                "Invalid timestamp: {} seconds, {} nanos",
                timestamp_seconds, timestamp_nanos
            ))
        })
    }

    /// Unpacks a `chrono::DateTime<Utc>` from the pack format.
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NONE => Ok(DateTime::<Utc>::default()),
            TAG_CHRONO_DATETIME => {
                let timestamp_seconds = i64::unpack(reader)?;
                let timestamp_nanos = u32::unpack(reader)?;
                DateTime::from_timestamp(timestamp_seconds, timestamp_nanos).ok_or_else(|| {
                    EncoderError::Decode(format!(
                        "Invalid timestamp: {} seconds, {} nanos",
                        timestamp_seconds, timestamp_nanos
                    ))
                })
            }
            _ => Err(EncoderError::Decode(format!(
                "Expected DateTime<Utc> tag ({} or {}), got {}",
                TAG_NONE, TAG_CHRONO_DATETIME, tag
            ))),
        }
    }
}

// --- DateTime<Local> ---
#[cfg(feature = "chrono")]
impl Encoder for DateTime<Local> {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_CHRONO_DATETIME);
        let timestamp_seconds = self.timestamp();
        let timestamp_nanos = self.timestamp_subsec_nanos();
        timestamp_seconds.encode(writer)?;
        timestamp_nanos.encode(writer)?;
        Ok(())
    }

    /// Packs a `chrono::DateTime<Local>` as seconds and nanoseconds without a type tag.
    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        if *self == DateTime::<Utc>::default() {
            writer.put_u8(TAG_NONE);
        } else {
            writer.put_u8(TAG_CHRONO_DATETIME);
            let timestamp_seconds = self.timestamp();
            let timestamp_nanos = self.timestamp_subsec_nanos();
            timestamp_seconds.pack(writer)?;
            timestamp_nanos.pack(writer)?;
        }
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == DateTime::<Local>::default()
    }
}
#[cfg(feature = "chrono")]
impl Decoder for DateTime<Local> {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_CHRONO_DATETIME {
            return Err(EncoderError::Decode(format!(
                "Expected DateTime<Local> tag ({}), got {}",
                TAG_CHRONO_DATETIME, tag
            )));
        }
        let timestamp_seconds = i64::decode(reader)?;
        let timestamp_nanos = u32::decode(reader)?;
        let utc_dt =
            DateTime::from_timestamp(timestamp_seconds, timestamp_nanos).ok_or_else(|| {
                EncoderError::Decode(format!(
                    "Invalid timestamp: {} seconds, {} nanos",
                    timestamp_seconds, timestamp_nanos
                ))
            })?;
        Ok(utc_dt.with_timezone(&Local))
    }

    /// Unpacks a `chrono::DateTime<Local>` from the pack format.
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NONE => Ok(DateTime::<Local>::default()),
            TAG_CHRONO_DATETIME => {
                let timestamp_seconds = i64::unpack(reader)?;
                let timestamp_nanos = u32::unpack(reader)?;
                let utc_dt = DateTime::from_timestamp(timestamp_seconds, timestamp_nanos).ok_or_else(|| {
                    EncoderError::Decode(format!(
                        "Invalid timestamp: {} seconds, {} nanos",
                        timestamp_seconds, timestamp_nanos
                    ))
                })?;
                Ok(utc_dt.with_timezone(&Local))
            }
            _ => Err(EncoderError::Decode(format!(
                "Expected DateTime<Local> tag ({} or {}), got {}",
                TAG_NONE, TAG_CHRONO_DATETIME, tag
            ))),
        }
    }
}

// --- NaiveDate ---
#[cfg(feature = "chrono")]
impl Encoder for NaiveDate {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_CHRONO_NAIVE_DATE);
        // Store as days since 1970-01-01
        let days_from_epoch = self
            .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days();
        days_from_epoch.encode(writer)?;
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == NaiveDate::default()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_CHRONO_NAIVE_DATE);
        // Store as days since 1970-01-01
        let days_from_epoch = self
            .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days();
        days_from_epoch.pack(writer)?;
        Ok(())
    }
}
#[cfg(feature = "chrono")]
impl Decoder for NaiveDate {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_CHRONO_NAIVE_DATE {
            return Err(EncoderError::Decode(format!(
                "Expected NaiveDate tag ({}), got {}",
                TAG_CHRONO_NAIVE_DATE, tag
            )));
        }
        let days_from_epoch = i64::decode(reader)?;
        let epoch_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        epoch_date
            .checked_add_signed(chrono::TimeDelta::try_days(days_from_epoch).unwrap())
            .ok_or_else(|| {
                EncoderError::Decode(format!("Invalid days from epoch: {}", days_from_epoch))
            })
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_CHRONO_NAIVE_DATE {
            return Err(EncoderError::Decode(format!(
                "Expected NaiveDate tag ({}), got {}",
                TAG_CHRONO_NAIVE_DATE, tag
            )));
        }
        let days_from_epoch = i64::unpack(reader)?;
        let epoch_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        epoch_date
            .checked_add_signed(chrono::TimeDelta::try_days(days_from_epoch).unwrap())
            .ok_or_else(|| {
                EncoderError::Decode(format!("Invalid days from epoch: {}", days_from_epoch))
            })
    }
}

// --- NaiveTime ---
#[cfg(feature = "chrono")]
impl Encoder for NaiveTime {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_CHRONO_NAIVE_TIME);
        // Store seconds and nanoseconds from 00:00:00 separately
        let seconds_from_midnight = self.num_seconds_from_midnight();
        let nanoseconds = self.nanosecond();
        seconds_from_midnight.encode(writer)?;
        nanoseconds.encode(writer)?;
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == NaiveTime::default()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_CHRONO_NAIVE_TIME);
        // Store seconds and nanoseconds from 00:00:00 separately
        let seconds_from_midnight = self.num_seconds_from_midnight();
        let nanoseconds = self.nanosecond();
        seconds_from_midnight.pack(writer)?;
        nanoseconds.pack(writer)?;
        Ok(())
    }
}
#[cfg(feature = "chrono")]
impl Decoder for NaiveTime {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_CHRONO_NAIVE_TIME {
            return Err(EncoderError::Decode(format!(
                "Expected NaiveTime tag ({}), got {}",
                TAG_CHRONO_NAIVE_TIME, tag
            )));
        }
        let seconds_from_midnight = u32::decode(reader)?;
        let nanoseconds = u32::decode(reader)?;
        NaiveTime::from_num_seconds_from_midnight_opt(seconds_from_midnight, nanoseconds)
            .ok_or_else(|| {
                EncoderError::Decode(format!(
                    "Invalid seconds from midnight: {}, nanoseconds: {}",
                    seconds_from_midnight, nanoseconds
                ))
            })
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_CHRONO_NAIVE_TIME {
            return Err(EncoderError::Decode(format!(
                "Expected NaiveTime tag ({}), got {}",
                TAG_CHRONO_NAIVE_TIME, tag
            )));
        }
        let seconds_from_midnight = u32::unpack(reader)?;
        let nanoseconds = u32::unpack(reader)?;
        NaiveTime::from_num_seconds_from_midnight_opt(seconds_from_midnight, nanoseconds)
            .ok_or_else(|| {
                EncoderError::Decode(format!(
                    "Invalid seconds from midnight: {}, nanoseconds: {}",
                    seconds_from_midnight, nanoseconds
                ))
            })
    }
}

// --- Decimal ---
#[cfg(feature = "rust_decimal")]
impl Encoder for Decimal {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_DECIMAL);
        // Get Decimal's internal representation and encode it
        let mantissa = self.mantissa();
        let scale = self.scale();
        mantissa.encode(writer)?;
        scale.encode(writer)?;
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == Decimal::default()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_DECIMAL);
        // Get Decimal's internal representation and encode it
        let mantissa = self.mantissa();
        let scale = self.scale();
        mantissa.pack(writer)?;
        scale.pack(writer)?;
        Ok(())
    }
}
#[cfg(feature = "rust_decimal")]
impl Decoder for Decimal {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_DECIMAL {
            return Err(EncoderError::Decode(format!(
                "Expected Decimal tag ({}), got {}",
                TAG_DECIMAL, tag
            )));
        }
        let mantissa = i128::decode(reader)?;
        let scale = u32::decode(reader)?;

        Decimal::try_from_i128_with_scale(mantissa, scale).map_err(|e| {
            EncoderError::Decode(format!(
                "Invalid decimal: mantissa={}, scale={}, error={}",
                mantissa, scale, e
            ))
        })
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_DECIMAL {
            return Err(EncoderError::Decode(format!(
                "Expected Decimal tag ({}), got {}",
                TAG_DECIMAL, tag
            )));
        }
        let mantissa = i128::unpack(reader)?;
        let scale = u32::unpack(reader)?;

        Decimal::try_from_i128_with_scale(mantissa, scale).map_err(|e| {
            EncoderError::Decode(format!(
                "Invalid decimal: mantissa={}, scale={}, error={}",
                mantissa, scale, e
            ))
        })
    }
}

// --- UUID ---
#[cfg(feature = "uuid")]
impl Encoder for Uuid {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_UUID);
        // Write UUID as u128 little-endian in fixed 16 bytes
        let uuid_u128 = self.as_u128();
        writer.put_u128_le(uuid_u128);
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == Uuid::default()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        if *self == Uuid::default() {
            writer.put_u8(TAG_NONE);
        } else {
            writer.put_u8(TAG_UUID);
            // Write UUID as u128 little-endian in fixed 16 bytes
            let uuid_u128 = self.as_u128();
            writer.put_u128_le(uuid_u128);
        }
        Ok(())
    }
}
#[cfg(feature = "uuid")]
impl Decoder for Uuid {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_UUID {
            return Err(EncoderError::Decode(format!(
                "Expected UUID tag ({}), got {}",
                TAG_UUID, tag
            )));
        }
        if reader.remaining() < 16 {
            return Err(EncoderError::InsufficientData);
        }
        let uuid_u128 = reader.get_u128_le();
        Ok(Uuid::from_u128(uuid_u128))
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NONE => Ok(Uuid::default()),
            TAG_UUID => {
                if reader.remaining() < 16 {
                    return Err(EncoderError::InsufficientData);
                }
                let uuid_u128 = reader.get_u128_le();
                Ok(Uuid::from_u128(uuid_u128))
            }
            _ => Err(EncoderError::Decode(format!(
                "Expected UUID tag ({} or {}), got {}",
                TAG_NONE, TAG_UUID, tag
            ))),
        }
    }
}

// --- ULID ---
#[cfg(feature = "ulid")]
impl Encoder for Ulid {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_UUID); // Use same tag as UUID
                                 // Write ULID as u128 little-endian in fixed 16 bytes
        let ulid_u128 = self.0;
        writer.put_u128_le(ulid_u128);
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == Ulid::default()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        if *self == Ulid::default() {
            writer.put_u8(TAG_NONE);
        } else {
            writer.put_u8(TAG_UUID); // Use same tag as UUID
                                     // Write ULID as u128 little-endian in fixed 16 bytes
            let ulid_u128 = self.0;
            writer.put_u128_le(ulid_u128);
        }
        Ok(())
    }
}
#[cfg(feature = "ulid")]
impl Decoder for Ulid {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        if tag != TAG_UUID {
            return Err(EncoderError::Decode(format!(
                "Expected ULID tag ({}), got {}",
                TAG_UUID, tag
            )));
        }
        if reader.remaining() < 16 {
            return Err(EncoderError::InsufficientData);
        }
        let ulid_u128 = reader.get_u128_le();
        Ok(Ulid(ulid_u128))
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_NONE => Ok(Ulid::default()),
            TAG_UUID => {
                if reader.remaining() < 16 {
                    return Err(EncoderError::InsufficientData);
                }
                let ulid_u128 = reader.get_u128_le();
                Ok(Ulid(ulid_u128))
            }
            _ => Err(EncoderError::Decode(format!(
                "Expected ULID tag ({} or {}), got {}",
                TAG_NONE, TAG_UUID, tag
            ))),
        }
    }
}

// --- serde_json::Value ---
#[cfg(feature = "serde_json")]
impl Encoder for Value {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        match self {
            Value::Null => {
                writer.put_u8(TAG_JSON_NULL);
                Ok(())
            }
            Value::Bool(b) => {
                writer.put_u8(TAG_JSON_BOOL);
                b.encode(writer)?;
                Ok(())
            }
            Value::Number(n) => {
                writer.put_u8(TAG_JSON_NUMBER);
                // Preserve integer/float distinction where possible
                if let Some(u) = n.as_u64() {
                    // Encode as tagged unsigned integer
                    writer.put_u8(0); // Unsigned integer (u64) marker
                    u.encode(writer)?;
                } else if let Some(i) = n.as_i64() {
                    // Encode as tagged signed integer
                    writer.put_u8(1); // Signed integer (i64) marker
                    i.encode(writer)?;
                } else {
                    // Encode as float
                    writer.put_u8(2); // Float marker
                    let float_val = n.as_f64().unwrap_or(0.0);
                    float_val.encode(writer)?;
                }
                Ok(())
            }
            Value::String(s) => {
                writer.put_u8(TAG_JSON_STRING);
                s.encode(writer)?;
                Ok(())
            }
            Value::Array(arr) => {
                writer.put_u8(TAG_JSON_ARRAY);
                let len = arr.len();
                len.encode(writer)?;
                for item in arr {
                    item.encode(writer)?;
                }
                Ok(())
            }
            Value::Object(obj) => {
                writer.put_u8(TAG_JSON_OBJECT);
                let len = obj.len();
                len.encode(writer)?;
                for (key, value) in obj {
                    key.encode(writer)?;
                    value.encode(writer)?;
                }
                Ok(())
            }
        }
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        *self == Value::default()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        match self {
            Value::Null => {
                writer.put_u8(TAG_JSON_NULL);
                Ok(())
            }
            Value::Bool(b) => {
                writer.put_u8(TAG_JSON_BOOL);
                b.pack(writer)?;
                Ok(())
            }
            Value::Number(n) => {
                writer.put_u8(TAG_JSON_NUMBER);
                // Preserve integer/float distinction where possible
                if let Some(u) = n.as_u64() {
                    // Encode as tagged unsigned integer
                    writer.put_u8(0); // Unsigned integer (u64) marker
                    u.pack(writer)?;
                } else if let Some(i) = n.as_i64() {
                    // Encode as tagged signed integer
                    writer.put_u8(1); // Signed integer (i64) marker
                    i.pack(writer)?;
                } else {
                    // Encode as float
                    writer.put_u8(2); // Float marker
                    let float_val = n.as_f64().unwrap_or(0.0);
                    float_val.pack(writer)?;
                }
                Ok(())
            }
            Value::String(s) => {
                writer.put_u8(TAG_JSON_STRING);
                s.pack(writer)?;
                Ok(())
            }
            Value::Array(arr) => {
                writer.put_u8(TAG_JSON_ARRAY);
                let len = arr.len();
                len.pack(writer)?;
                for item in arr {
                    item.pack(writer)?;
                }
                Ok(())
            }
            Value::Object(obj) => {
                writer.put_u8(TAG_JSON_OBJECT);
                let len = obj.len();
                len.pack(writer)?;
                for (key, value) in obj {
                    key.pack(writer)?;
                    value.pack(writer)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(feature = "serde_json")]
impl Decoder for Value {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_JSON_NULL => Ok(Value::Null),
            TAG_JSON_BOOL => {
                let b = bool::decode(reader)?;
                Ok(Value::Bool(b))
            }
            TAG_JSON_NUMBER => {
                if reader.remaining() == 0 {
                    return Err(EncoderError::InsufficientData);
                }
                let number_type = reader.get_u8();
                match number_type {
                    0 => {
                        // Unsigned integer
                        let u = u64::decode(reader)?;
                        Ok(Value::Number(Number::from(u)))
                    }
                    1 => {
                        // Signed integer
                        let i = i64::decode(reader)?;
                        Ok(Value::Number(Number::from(i)))
                    }
                    2 => {
                        // Float
                        let f = f64::decode(reader)?;
                        Ok(Value::Number(
                            Number::from_f64(f).unwrap_or(Number::from(0)),
                        ))
                    }
                    _ => Err(EncoderError::Decode(format!(
                        "Invalid JSON Number type marker: {}",
                        number_type
                    ))),
                }
            }
            TAG_JSON_STRING => {
                let s = String::decode(reader)?;
                Ok(Value::String(s))
            }
            TAG_JSON_ARRAY => {
                let len = usize::decode(reader)?;
                let mut arr = Vec::with_capacity(len);
                for _ in 0..len {
                    arr.push(Value::decode(reader)?);
                }
                Ok(Value::Array(arr))
            }
            TAG_JSON_OBJECT => {
                let len = usize::decode(reader)?;
                let mut obj = Map::with_capacity(len);
                for _ in 0..len {
                    let key = String::decode(reader)?;
                    let value = Value::decode(reader)?;
                    obj.insert(key, value);
                }
                Ok(Value::Object(obj))
            }
            _ => Err(EncoderError::Decode(format!(
                "Expected JSON Value tag (202-207), got {}",
                tag
            ))),
        }
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        match tag {
            TAG_JSON_NULL => Ok(Value::Null),
            TAG_JSON_BOOL => {
                let b = bool::unpack(reader)?;
                Ok(Value::Bool(b))
            }
            TAG_JSON_NUMBER => {
                if reader.remaining() == 0 {
                    return Err(EncoderError::InsufficientData);
                }
                let number_type = reader.get_u8();
                match number_type {
                    0 => {
                        // Unsigned integer
                        let u = u64::unpack(reader)?;
                        Ok(Value::Number(Number::from(u)))
                    }
                    1 => {
                        // Signed integer
                        let i = i64::unpack(reader)?;
                        Ok(Value::Number(Number::from(i)))
                    }
                    2 => {
                        // Float
                        let f = f64::unpack(reader)?;
                        Ok(Value::Number(
                            Number::from_f64(f).unwrap_or(Number::from(0)),
                        ))
                    }
                    _ => Err(EncoderError::Decode(format!(
                        "Invalid JSON Number type marker: {}",
                        number_type
                    ))),
                }
            }
            TAG_JSON_STRING => {
                let s = String::unpack(reader)?;
                Ok(Value::String(s))
            }
            TAG_JSON_ARRAY => {
                let len = usize::unpack(reader)?;
                let mut arr = Vec::with_capacity(len);
                for _ in 0..len {
                    arr.push(Value::unpack(reader)?);
                }
                Ok(Value::Array(arr))
            }
            TAG_JSON_OBJECT => {
                let len = usize::unpack(reader)?;
                let mut obj = Map::with_capacity(len);
                for _ in 0..len {
                    let key = String::unpack(reader)?;
                    let value = Value::unpack(reader)?;
                    obj.insert(key, value);
                }
                Ok(Value::Object(obj))
            }
            _ => Err(EncoderError::Decode(format!(
                "Expected JSON Value tag (202-207), got {}",
                tag
            ))),
        }
    }
}

// --- FxHashMap ---
#[cfg(feature = "fxhash")]
impl<K: Encoder + Eq + std::hash::Hash, V: Encoder> Encoder for FxHashMap<K, V> {
    #[cfg(feature = "encode")]
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

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_MAP);
        let len = self.len();
        len.pack(writer)?;
        for (k, v) in self {
            k.pack(writer)?;
            v.pack(writer)?;
        }
        Ok(())
    }
}
#[cfg(feature = "fxhash")]
impl<K: Decoder + Eq + std::hash::Hash, V: Decoder> Decoder for FxHashMap<K, V> {
    #[cfg(feature = "encode")]
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
        let mut map = FxHashMap::with_capacity_and_hasher(len, Default::default());
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
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
        let len = usize::unpack(reader)?;
        let mut map = FxHashMap::with_capacity_and_hasher(len, Default::default());
        for _ in 0..len {
            let k = K::unpack(reader)?;
            let v = V::unpack(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

// --- AHashMap ---
#[cfg(feature = "ahash")]
impl<K: Encoder + Eq + std::hash::Hash, V: Encoder> Encoder for AHashMap<K, V> {
    #[cfg(feature = "encode")]
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

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        writer.put_u8(TAG_MAP);
        let len = self.len();
        len.pack(writer)?;
        for (k, v) in self {
            k.pack(writer)?;
            v.pack(writer)?;
        }
        Ok(())
    }
}
#[cfg(feature = "ahash")]
impl<K: Decoder + Eq + std::hash::Hash, V: Decoder> Decoder for AHashMap<K, V> {
    #[cfg(feature = "encode")]
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
        let mut map = AHashMap::with_capacity(len);
        for _ in 0..len {
            let k = K::decode(reader)?;
            let v = V::decode(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
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
        let len = usize::unpack(reader)?;
        let mut map = AHashMap::with_capacity(len);
        for _ in 0..len {
            let k = K::unpack(reader)?;
            let v = V::unpack(reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

// --- FxHashSet ---
#[cfg(feature = "fxhash")]
impl<T: Encoder + Eq + std::hash::Hash> Encoder for FxHashSet<T> {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.encode(writer)?;
        }
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.pack(writer)?;
        }
        Ok(())
    }
}
#[cfg(feature = "fxhash")]
impl<T: Decoder + Eq + std::hash::Hash + 'static> Decoder for FxHashSet<T> {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::decode(reader)?;
        Ok(vec.into_iter().collect())
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::unpack(reader)?;
        Ok(vec.into_iter().collect())
    }
}

// --- AHashSet ---
#[cfg(feature = "ahash")]
impl<T: Encoder + Eq + std::hash::Hash> Encoder for AHashSet<T> {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.encode(writer)?;
        }
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        encode_vec_length(self.len(), writer)?;
        for v in self {
            v.pack(writer)?;
        }
        Ok(())
    }
}
#[cfg(feature = "ahash")]
impl<T: Decoder + Eq + std::hash::Hash + 'static> Decoder for AHashSet<T> {
    #[cfg(feature = "encode")]
    fn decode(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::decode(reader)?;
        Ok(vec.into_iter().collect())
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        let vec: Vec<T> = Vec::unpack(reader)?;
        Ok(vec.into_iter().collect())
    }
}

// --- SmolStr ---
#[cfg(feature = "smol_str")]
impl Encoder for SmolStr {
    #[cfg(feature = "encode")]
    fn encode(&self, writer: &mut BytesMut) -> Result<()> {
        let len = self.len();
        let max_short = (TAG_STRING_LONG - TAG_STRING_BASE - 1) as usize;
        if len <= max_short {
            let tag = TAG_STRING_BASE + len as u8;
            writer.put_u8(tag);
            writer.put_slice(self.as_bytes());
        } else {
            writer.put_u8(TAG_STRING_LONG);
            len.encode(writer)?;
            writer.put_slice(self.as_bytes());
        }
        Ok(())
    }

    #[cfg(feature = "encode")]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[cfg(feature = "pack")]
    fn pack(&self, writer: &mut BytesMut) -> Result<()> {
        let len = self.len();
        let max_short = (TAG_STRING_LONG - TAG_STRING_BASE - 1) as usize;
        if len <= max_short {
            let tag = TAG_STRING_BASE + len as u8;
            writer.put_u8(tag);
            writer.put_slice(self.as_bytes());
        } else {
            writer.put_u8(TAG_STRING_LONG);
            len.pack(writer)?;
            writer.put_slice(self.as_bytes());
        }
        Ok(())
    }
}
#[cfg(feature = "smol_str")]
impl Decoder for SmolStr {
    #[cfg(feature = "encode")]
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
        let string = String::from_utf8(bytes).map_err(|e| EncoderError::Decode(e.to_string()))?;
        Ok(SmolStr::new(string))
    }
    #[cfg(feature = "pack")]
    fn unpack(reader: &mut Bytes) -> Result<Self> {
        if reader.remaining() == 0 {
            return Err(EncoderError::InsufficientData);
        }
        let tag = reader.get_u8();
        let len = if (TAG_STRING_BASE..TAG_STRING_LONG).contains(&tag) {
            (tag - TAG_STRING_BASE) as usize
        } else if tag == TAG_STRING_LONG {
            usize::unpack(reader)?
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
        let string = String::from_utf8(bytes).map_err(|e| EncoderError::Decode(e.to_string()))?;
        Ok(SmolStr::new(string))
    }
}
