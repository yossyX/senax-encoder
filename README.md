# senax-encoder

A fast, compact, and schema-evolution-friendly binary serialization library for Rust.

- Supports struct/enum encoding with field/variant IDs for forward/backward compatibility
- Efficient encoding for primitives, collections, Option, String, bytes, and popular crates (chrono, uuid, ulid, rust_decimal, indexmap, serde_json)
- Custom derive macros for ergonomic usage
- Feature-gated support for optional dependencies

## Features

- Compact, efficient encoding for a wide range of types (primitives, collections, Option, String, bytes, chrono, uuid, ulid, rust_decimal, indexmap, serde_json)
- Schema evolution and version compatibility via field/variant IDs and tag-based format
- Attribute macros for fine-grained control (custom IDs, default values, skip encode/decode, renaming, compact ID encoding)
- Feature flags for optional support of popular crates
- Suitable for network protocols, storage, and applications requiring forward/backward compatibility

## Attribute Macros

You can control encoding/decoding behavior using the following attributes:

- `#[senax(id = N)]` — Assigns a custom field or variant ID (u32 or u8, see below). Ensures stable wire format across versions.
- `#[senax(default)]` — If a field is missing during decoding, its value is set to `Default::default()` instead of causing an error. For `Option<T>`, this means `None`.
- `#[senax(skip_encode)]` — This field is not written during encoding. On decode, it is set to `Default::default()`.
- `#[senax(skip_decode)]` — This field is ignored during decoding and always set to `Default::default()`. It is still encoded if present.
- `#[senax(skip_default)]` — This field is not written during encoding if its value equals the default value. On decode, missing fields are set to `Default::default()`.
- `#[senax(rename = "name")]` — Use the given string as the logical field/variant name for ID calculation. Useful for renaming fields/variants while keeping the same wire format.
- `#[senax(u8)]` — On structs/enums, encodes field/variant IDs as `u8` instead of `u32` (for compactness, up to 255 IDs; 0 is reserved for terminator).

## Feature Flags

The following optional features enable support for popular crates and types:

- `chrono` — Enables encoding/decoding of `chrono::DateTime`, `NaiveDate`, and `NaiveTime` types.
- `uuid` — Enables encoding/decoding of `uuid::Uuid`.
- `ulid` — Enables encoding/decoding of `ulid::Ulid` (shares the same tag as UUID for binary compatibility).
- `rust_decimal` — Enables encoding/decoding of `rust_decimal::Decimal`.
- `indexmap` — Enables encoding/decoding of `IndexMap` and `IndexSet` collections.
- `serde_json` — Enables encoding/decoding of `serde_json::Value` for dynamic JSON data.

## Example

```rust
use senax_encoder::{Encoder, Decoder, Encode, Decode};
use bytes::BytesMut;

#[derive(Encode, Decode, PartialEq, Debug)]
struct MyStruct {
    id: u32,
    name: String,
}

let value = MyStruct { id: 42, name: "hello".to_string() };
let mut buf = BytesMut::new();
value.encode(&mut buf).unwrap();
let decoded = MyStruct::decode(&mut buf.freeze()).unwrap();
assert_eq!(value, decoded);
```

## Quick Start

Add to your `Cargo.toml`:
```toml
[dependencies]
senax-encoder = "0.1"
```

Basic usage:
```rust
use senax_encoder::{Encoder, Decoder, Encode, Decode};
use bytes::{BytesMut, Bytes};

#[derive(Encode, Decode, Debug, PartialEq)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

let user = User { id: 42, name: "Alice".into(), email: Some("alice@example.com".into()) };
let mut buf = BytesMut::new();
user.encode(&mut buf).unwrap();
let mut bytes = buf.freeze();
let decoded = User::decode(&mut bytes).unwrap();
assert_eq!(user, decoded);
```

## Usage

### 1. Derive macros for automatic implementation
```rust
#[derive(Encode, Decode)]
struct MyStruct {
    #[senax(id=1)]
    foo: u32,
    bar: Option<String>,
}
```

### 2. Binary encode/decode
```rust
let mut buf = BytesMut::new();
value.encode(&mut buf)?;
let mut bytes = buf.freeze();
let value2 = MyStruct::decode(&mut bytes)?;
```

### 3. Schema evolution (adding/removing/changing fields)
- Field IDs are **automatically generated from field names (CRC32)** by default.
  - Use `#[senax(id=...)]` only if you need to resolve a collision.
- Because mapping is by field ID (u32):
  - **Old struct → new struct**:
    - New fields of type `Option` become `None` if missing.
    - New required fields without `default` will cause a decode error if missing.
  - **New struct → old struct**: unknown fields are automatically skipped.
- **No field names are stored, only u32 IDs, so field addition/removal/reordering/type changes are robust.**

### 4. Feature flags
- Enable only the types you need: `indexmap`, `chrono`, `rust_decimal`, `uuid`, `ulid`, `serde_json`, etc.
- Minimizes dependencies and build time.

## Supported Types

### Core Types (always available)
- Primitives: `u8~u128`, `i8~i128`, `f32`, `f64`, `bool`, `String`, `Bytes`
- Option, Vec, arrays, HashMap, BTreeMap, Set, Tuple, Enum, Struct

### Feature-gated Types
When respective features are enabled:

- **chrono**: `DateTime<Utc>`, `DateTime<Local>`, `NaiveDate`, `NaiveTime`
- **uuid**: `Uuid`