#![cfg(feature = "pack")]

use bytes::BufMut;
use senax_encoder::{encode, pack, unpack, Decode, Encode};
use std::collections::HashMap;

#[derive(Encode, Decode, PartialEq, Debug)]
struct SimpleStruct {
    id: u32,
    name: String,
    active: bool,
}

#[derive(Encode, Decode, PartialEq, Debug)]
struct SimpleStructForComparison {
    id: u32,
    name: String,
    active: bool,
}

#[derive(Encode, Decode, PartialEq, Debug)]
struct TupleStruct(u32, String, bool);

#[derive(Encode, Decode, PartialEq, Debug)]
struct UnitStruct;

#[test]
fn test_pack_unpack_named_struct() {
    let original = SimpleStruct {
        id: 42,
        name: "hello".to_string(),
        active: true,
    };

    // Pack the struct
    let packed_bytes = pack(&original).unwrap();

    // Unpack the struct
    let mut reader = packed_bytes;
    let unpacked: SimpleStruct = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_pack_unpack_tuple_struct() {
    let original = TupleStruct(123, "world".to_string(), false);

    // Pack the struct
    let packed_bytes = pack(&original).unwrap();

    // Unpack the struct
    let mut reader = packed_bytes;
    let unpacked: TupleStruct = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_pack_unpack_unit_struct() {
    let original = UnitStruct;

    // Pack the struct
    let packed_bytes = pack(&original).unwrap();

    // Unpack the struct
    let mut reader = packed_bytes;
    let unpacked: UnitStruct = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_pack_vs_encode_size_difference() {
    let original_pack = SimpleStruct {
        id: 42,
        name: "hello".to_string(),
        active: true,
    };

    let original_encode = SimpleStructForComparison {
        id: 42,
        name: "hello".to_string(),
        active: true,
    };

    // Pack (no field IDs)
    let packed_bytes = pack(&original_pack).unwrap();

    // Encode (with field IDs)
    let encoded_bytes = encode(&original_encode).unwrap();

    // Pack should be smaller since it doesn't include field IDs
    println!("Packed size: {}", packed_bytes.len());
    println!("Encoded size: {}", encoded_bytes.len());
    assert!(packed_bytes.len() < encoded_bytes.len());
}

#[test]
fn test_pack_unpack_field_order_dependency() {
    // This test demonstrates that pack/unpack is order-dependent
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct StructA {
        first: u32,
        second: String,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct StructB {
        second: String, // Different order!
        first: u32,
    }

    let original_a = StructA {
        first: 42,
        second: "hello".to_string(),
    };

    // Pack as StructA
    let packed_bytes = pack(&original_a).unwrap();

    // Try to unpack as StructB (different field order)
    let mut reader = packed_bytes;
    let result: Result<StructB, _> = unpack(&mut reader);

    // This should fail or produce incorrect results because field order matters
    // We can't directly test the failure since the types are different,
    // but this demonstrates the concept
    assert!(result.is_err() || result.unwrap().first != 42);
}

#[test]
fn test_nested_structs_pack_unpack() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Inner {
        value: u32,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Outer {
        inner: Inner,
        name: String,
    }

    let original = Outer {
        inner: Inner { value: 123 },
        name: "nested".to_string(),
    };

    let packed_bytes = pack(&original).unwrap();
    let mut reader = packed_bytes;
    let unpacked: Outer = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_enum_named_variant_pack_unpack() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    enum TestEnum {
        Named { id: u32, name: String },
        Unnamed(u32, String),
        Unit,
    }

    let original = TestEnum::Named {
        id: 42,
        name: "hello".to_string(),
    };

    // Pack the enum
    let packed_bytes = pack(&original).unwrap();

    // Unpack the enum
    let mut reader = packed_bytes;
    let unpacked: TestEnum = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_enum_pack_vs_encode_size_difference() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    enum TestEnum {
        Named { id: u32, name: String, active: bool },
    }

    let original = TestEnum::Named {
        id: 42,
        name: "hello".to_string(),
        active: true,
    };

    // Pack (no field IDs for enum fields)
    let packed_bytes = pack(&original).unwrap();

    // Encode (with field IDs for enum fields)
    let encoded_bytes = encode(&original).unwrap();

    // Pack should be smaller since it doesn't include field IDs for enum fields
    println!("Enum packed size: {}", packed_bytes.len());
    println!("Enum encoded size: {}", encoded_bytes.len());
    assert!(packed_bytes.len() < encoded_bytes.len());
}

#[test]
fn test_enum_field_order_dependency() {
    // This test demonstrates that pack/unpack is order-dependent for enum fields too
    #[derive(Encode, Decode, PartialEq, Debug)]
    enum EnumA {
        Named { first: u32, second: String },
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    enum EnumB {
        Named { second: String, first: u32 }, // Different field order!
    }

    let original_a = EnumA::Named {
        first: 42,
        second: "hello".to_string(),
    };

    // Pack as EnumA
    let packed_bytes = pack(&original_a).unwrap();

    // Try to unpack as EnumB (different field order)
    let mut reader = packed_bytes;
    let result: Result<EnumB, _> = unpack(&mut reader);

    // This should fail or produce incorrect results because field order matters
    if let Ok(unpacked) = result {
        let EnumB::Named { first, .. } = unpacked;
        assert_ne!(first, 42); // Field order mismatch should cause incorrect values
    }
}

#[test]
fn test_primitive_pack_unpack() {
    // Test bool with relaxed validation
    let bool_val = true;
    let packed_bool = pack(&bool_val).unwrap();
    let mut reader = packed_bool;
    let unpacked_bool: bool = unpack(&mut reader).unwrap();
    assert_eq!(bool_val, unpacked_bool);

    // Test bool unpack with non-standard values (should work with any non-zero)
    let mut writer = bytes::BytesMut::new();
    writer.put_u8(42); // Non-standard true value
    let mut reader = writer.freeze();
    let unpacked_bool: bool = unpack(&mut reader).unwrap();
    assert_eq!(true, unpacked_bool);

    // Test f32 without tag
    let f32_val = 3.14159f32;
    let packed_f32 = pack(&f32_val).unwrap();
    let mut reader = packed_f32;
    let unpacked_f32: f32 = unpack(&mut reader).unwrap();
    assert_eq!(f32_val, unpacked_f32);

    // Test f64 without tag
    let f64_val = 2.718281828459045f64;
    let packed_f64 = pack(&f64_val).unwrap();
    let mut reader = packed_f64;
    let unpacked_f64: f64 = unpack(&mut reader).unwrap();
    assert_eq!(f64_val, unpacked_f64);
}

#[test]
fn test_primitive_pack_vs_encode_size() {
    // Test that pack is smaller than encode for primitives
    let f32_val = 3.14159f32;
    let packed_f32 = pack(&f32_val).unwrap();
    let encoded_f32 = encode(&f32_val).unwrap();

    println!("f32 packed size: {}", packed_f32.len());
    println!("f32 encoded size: {}", encoded_f32.len());
    assert!(packed_f32.len() < encoded_f32.len()); // 4 bytes vs 5 bytes (tag + data)

    let f64_val = 2.718281828459045f64;
    let packed_f64 = pack(&f64_val).unwrap();
    let encoded_f64 = encode(&f64_val).unwrap();

    println!("f64 packed size: {}", packed_f64.len());
    println!("f64 encoded size: {}", encoded_f64.len());
    assert!(packed_f64.len() < encoded_f64.len()); // 8 bytes vs 9 bytes (tag + data)
}

#[test]
#[cfg(feature = "chrono")]
fn test_datetime_pack_unpack() {
    use chrono::{DateTime, Local, Utc};

    // Test DateTime<Utc>
    let utc_dt = DateTime::from_timestamp(1640995200, 123456789).unwrap();
    let packed_utc = pack(&utc_dt).unwrap();
    let mut reader = packed_utc;
    let unpacked_utc: DateTime<Utc> = unpack(&mut reader).unwrap();
    assert_eq!(utc_dt, unpacked_utc);

    // Test DateTime<Local>
    let local_dt = utc_dt.with_timezone(&Local);
    let packed_local = pack(&local_dt).unwrap();
    let mut reader = packed_local;
    let unpacked_local: DateTime<Local> = unpack(&mut reader).unwrap();
    assert_eq!(local_dt, unpacked_local);
}

#[test]
#[cfg(feature = "chrono")]
fn test_datetime_pack_vs_encode_size() {
    use chrono::DateTime;

    let dt = DateTime::from_timestamp(1640995200, 123456789).unwrap();

    let packed_dt = pack(&dt).unwrap();
    let encoded_dt = encode(&dt).unwrap();

    println!("DateTime packed size: {}", packed_dt.len());
    println!("DateTime encoded size: {}", encoded_dt.len());
    assert!(packed_dt.len() < encoded_dt.len()); // Should be 1 byte smaller (no tag)
}

#[test]
fn test_complex_nested_struct_pack_unpack() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Address {
        street: String,
        city: String,
        zip_code: u32,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Person {
        id: u64,
        name: String,
        age: u8,
        email: Option<String>,
        addresses: Vec<Address>,
        scores: HashMap<String, f64>,
        is_active: bool,
    }

    let original = Person {
        id: 12345,
        name: "田中太郎".to_string(),
        age: 30,
        email: Some("tanaka@example.com".to_string()),
        addresses: vec![
            Address {
                street: "1-2-3 Shibuya".to_string(),
                city: "Tokyo".to_string(),
                zip_code: 1500001,
            },
            Address {
                street: "4-5-6 Osaka".to_string(),
                city: "Osaka".to_string(),
                zip_code: 5300001,
            },
        ],
        scores: {
            let mut map = HashMap::new();
            map.insert("math".to_string(), 95.5);
            map.insert("english".to_string(), 87.2);
            map.insert("science".to_string(), 92.8);
            map
        },
        is_active: true,
    };

    // Pack the complex struct
    let packed_bytes = pack(&original).unwrap();

    // Unpack the complex struct
    let mut reader = packed_bytes;
    let unpacked: Person = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_complex_enum_with_nested_data_pack_unpack() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct UserProfile {
        username: String,
        followers: u32,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct ProductInfo {
        name: String,
        price: f64,
        categories: Vec<String>,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    enum ComplexEnum {
        User {
            profile: UserProfile,
            posts: Vec<String>,
            metadata: HashMap<String, String>,
        },
        Product {
            info: ProductInfo,
            reviews: Option<Vec<String>>,
            rating: f32,
        },
        System {
            version: String,
            config: HashMap<String, i32>,
        },
        Empty,
    }

    // Test User variant
    let user_variant = ComplexEnum::User {
        profile: UserProfile {
            username: "user123".to_string(),
            followers: 1500,
        },
        posts: vec!["Hello world!".to_string(), "Second post".to_string()],
        metadata: {
            let mut map = HashMap::new();
            map.insert("theme".to_string(), "dark".to_string());
            map.insert("language".to_string(), "ja".to_string());
            map
        },
    };

    let packed_user = pack(&user_variant).unwrap();
    let mut reader = packed_user;
    let unpacked_user: ComplexEnum = unpack(&mut reader).unwrap();
    assert_eq!(user_variant, unpacked_user);

    // Test Product variant
    let product_variant = ComplexEnum::Product {
        info: ProductInfo {
            name: "Laptop".to_string(),
            price: 999.99,
            categories: vec!["electronics".to_string(), "computers".to_string()],
        },
        reviews: Some(vec!["Great product!".to_string(), "Good value".to_string()]),
        rating: 4.5,
    };

    let packed_product = pack(&product_variant).unwrap();
    let mut reader = packed_product;
    let unpacked_product: ComplexEnum = unpack(&mut reader).unwrap();
    assert_eq!(product_variant, unpacked_product);

    // Test Empty variant
    let empty_variant = ComplexEnum::Empty;
    let packed_empty = pack(&empty_variant).unwrap();
    let mut reader = packed_empty;
    let unpacked_empty: ComplexEnum = unpack(&mut reader).unwrap();
    assert_eq!(empty_variant, unpacked_empty);
}

#[test]
fn test_deeply_nested_struct_pack_unpack() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Level4 {
        value: String,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Level3 {
        level4: Level4,
        numbers: Vec<i32>,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Level2 {
        level3: Level3,
        optional_data: Option<HashMap<String, f64>>,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Level1 {
        level2: Level2,
        flags: Vec<bool>,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct RootLevel {
        level1: Level1,
        metadata: HashMap<String, String>,
        id: u64,
    }

    let original = RootLevel {
        level1: Level1 {
            level2: Level2 {
                level3: Level3 {
                    level4: Level4 {
                        value: "deep nested value".to_string(),
                    },
                    numbers: vec![1, 2, 3, 4, 5],
                },
                optional_data: Some({
                    let mut map = HashMap::new();
                    map.insert("pi".to_string(), 3.14159);
                    map.insert("e".to_string(), 2.71828);
                    map
                }),
            },
            flags: vec![true, false, true, true],
        },
        metadata: {
            let mut map = HashMap::new();
            map.insert("created_by".to_string(), "test".to_string());
            map.insert("version".to_string(), "1.0".to_string());
            map
        },
        id: 999999,
    };

    let packed_bytes = pack(&original).unwrap();
    let mut reader = packed_bytes;
    let unpacked: RootLevel = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}

#[test]
fn test_complex_struct_pack_vs_encode_size() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct ComplexStruct {
        id: u64,
        name: String,
        values: Vec<f64>,
        metadata: HashMap<String, String>,
        optional_field: Option<String>,
        nested: Option<Box<ComplexStruct>>,
    }

    let original = ComplexStruct {
        id: 12345,
        name: "complex_test".to_string(),
        values: vec![1.1, 2.2, 3.3, 4.4, 5.5],
        metadata: {
            let mut map = HashMap::new();
            map.insert("key1".to_string(), "value1".to_string());
            map.insert("key2".to_string(), "value2".to_string());
            map.insert("key3".to_string(), "value3".to_string());
            map
        },
        optional_field: Some("optional_value".to_string()),
        nested: Some(Box::new(ComplexStruct {
            id: 67890,
            name: "nested".to_string(),
            values: vec![9.9, 8.8],
            metadata: HashMap::new(),
            optional_field: None,
            nested: None,
        })),
    };

    // Pack (no field IDs)
    let packed_bytes = pack(&original).unwrap();

    // Encode (with field IDs)
    let encoded_bytes = encode(&original).unwrap();

    println!("Complex struct packed size: {}", packed_bytes.len());
    println!("Complex struct encoded size: {}", encoded_bytes.len());

    // Pack should be significantly smaller due to no field IDs
    assert!(packed_bytes.len() < encoded_bytes.len());

    // Verify the data can be unpacked correctly
    let mut reader = packed_bytes;
    let unpacked: ComplexStruct = unpack(&mut reader).unwrap();
    assert_eq!(original, unpacked);
}

#[test]
fn test_array_of_complex_structs_pack_unpack() {
    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Item {
        id: u32,
        name: String,
        tags: Vec<String>,
        properties: HashMap<String, f64>,
    }

    #[derive(Encode, Decode, PartialEq, Debug)]
    struct Container {
        items: Vec<Item>,
        total_count: usize,
        summary: HashMap<String, u32>,
    }

    let original = Container {
        items: vec![
            Item {
                id: 1,
                name: "item1".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("weight".to_string(), 1.5);
                    map.insert("height".to_string(), 10.0);
                    map
                },
            },
            Item {
                id: 2,
                name: "item2".to_string(),
                tags: vec!["tag3".to_string()],
                properties: {
                    let mut map = HashMap::new();
                    map.insert("weight".to_string(), 2.3);
                    map.insert("width".to_string(), 5.0);
                    map
                },
            },
            Item {
                id: 3,
                name: "item3".to_string(),
                tags: vec![],
                properties: HashMap::new(),
            },
        ],
        total_count: 3,
        summary: {
            let mut map = HashMap::new();
            map.insert("processed".to_string(), 3);
            map.insert("errors".to_string(), 0);
            map
        },
    };

    let packed_bytes = pack(&original).unwrap();
    let mut reader = packed_bytes;
    let unpacked: Container = unpack(&mut reader).unwrap();

    assert_eq!(original, unpacked);
}
