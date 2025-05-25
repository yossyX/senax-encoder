extern crate proc_macro;

use crc::{Crc, CRC_64_ECMA_182};
use itertools::izip;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, GenericArgument, Ident, PathArguments,
    Type,
};

/// CRC-64 hasher for field name to ID conversion
const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);

/// Calculate a unique field ID from a field name using CRC-64
///
/// This function generates a deterministic 64-bit ID from a field name by computing
/// the CRC-64 hash. This ensures consistent field IDs across compilation runs
/// while providing excellent distribution and minimizing collision probability.
///
/// # Arguments
///
/// * `name` - The field name to hash
///
/// # Returns
///
/// A 64-bit field ID (never 0, as 0 is reserved as a terminator)
fn calculate_id_from_name(name: &str) -> u64 {
    let crc64_hash = CRC64.checksum(name.as_bytes());
    // Ensure it's not 0 (0 is reserved as terminator)
    if crc64_hash == 0 {
        u64::MAX
    } else {
        crc64_hash
    }
}

/// Check if a variant has the #[default] attribute
fn has_default_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("default"))
}

/// Field attributes parsed from `#[senax(...)]` annotations
///
/// This struct represents the various attributes that can be applied to fields
/// in structs and enum variants using the `#[senax(...)]` attribute macro.
///
/// # Fields
///
/// * `id` - The unique identifier for this field (computed from name or explicitly set)
/// * `default` - Whether to use default values when the field is missing during decode
/// * `skip_encode` - Whether to exclude this field from encoding
/// * `skip_decode` - Whether to ignore this field during decoding
/// * `skip_default` - Whether to use default value if field is missing
/// * `rename` - Optional alternative name for ID calculation (maintains compatibility when renaming)
#[derive(Debug, Clone)]
#[allow(dead_code)] // The rename field is used indirectly in ID calculation
struct FieldAttributes {
    id: u64,
    default: bool,
    skip_encode: bool,
    skip_decode: bool,
    skip_default: bool,
    rename: Option<String>,
}

/// Extract and parse `#[senax(...)]` attribute values from field attributes
///
/// This function parses the senax attributes applied to a field and returns
/// a `FieldAttributes` struct containing all the parsed values.
///
/// # Arguments
///
/// * `attrs` - The attributes array from the field
/// * `field_name` - The name of the field (used for ID calculation if no explicit ID is provided)
///
/// # Returns
///
/// A `FieldAttributes` struct with parsed values. If no explicit ID is provided,
/// the ID is calculated using CRC64 hash of either the rename value or the field name.
///
/// # Supported Attributes
///
/// * `#[senax(id=1234)]` - Explicit field ID
/// * `#[senax(default)]` - Use default value if field is missing during decode
/// * `#[senax(skip_encode)]` - Skip this field during encoding
/// * `#[senax(skip_decode)]` - Skip this field during decoding
/// * `#[senax(skip_default)]` - Skip encoding if field value is default, use default if missing during decode
/// * `#[senax(rename="name")]` - Alternative name for ID calculation
///
/// Multiple attributes can be combined: `#[senax(id=123, default, skip_encode)]`
fn get_field_attributes(attrs: &[Attribute], field_name: &str) -> FieldAttributes {
    let mut id = None;
    let mut default = false;
    let mut skip_encode = false;
    let mut skip_decode = false;
    let mut skip_default = false;
    let mut rename = None;

    for attr in attrs {
        if attr.path().is_ident("senax") {
            // Try to parse #[senax(id=1234, default, skip_encode, skip_decode, skip_default, rename="name")]
            let parsed = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let mut parsed_id = None;
                let mut parsed_default = false;
                let mut parsed_skip_encode = false;
                let mut parsed_skip_decode = false;
                let mut parsed_skip_default = false;
                let mut parsed_rename = None;

                while !input.is_empty() {
                    let ident = input.parse::<syn::Ident>()?;

                    if ident == "id" {
                        input.parse::<syn::Token![=]>()?;
                        let lit = input.parse::<syn::LitInt>()?;
                        if let Ok(id_val) = lit.base10_parse::<u64>() {
                            if id_val == 0 {
                                return Err(syn::Error::new(
                                    lit.span(),
                                    "Field ID 0 is reserved as a terminator",
                                ));
                            }
                            parsed_id = Some(id_val);
                        } else {
                            return Err(syn::Error::new(lit.span(), "Failed to parse ID value"));
                        }
                    } else if ident == "default" {
                        parsed_default = true;
                    } else if ident == "skip_encode" {
                        parsed_skip_encode = true;
                    } else if ident == "skip_decode" {
                        parsed_skip_decode = true;
                    } else if ident == "skip_default" {
                        parsed_skip_default = true;
                    } else if ident == "rename" {
                        input.parse::<syn::Token![=]>()?;
                        let lit_str = input.parse::<syn::LitStr>()?;
                        parsed_rename = Some(lit_str.value());
                    } else {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("Unknown attribute: {}", ident),
                        ));
                    }

                    // Consume comma if present, otherwise end
                    if input.peek(syn::Token![,]) {
                        input.parse::<syn::Token![,]>()?;
                    }
                }

                Ok((
                    parsed_id,
                    parsed_default,
                    parsed_skip_encode,
                    parsed_skip_decode,
                    parsed_skip_default,
                    parsed_rename,
                ))
            });

            if let Ok((
                parsed_id,
                parsed_default,
                parsed_skip_encode,
                parsed_skip_decode,
                parsed_skip_default,
                parsed_rename,
            )) = parsed
            {
                if let Some(id_val) = parsed_id {
                    id = Some(id_val);
                }
                default = default || parsed_default;
                skip_encode = skip_encode || parsed_skip_encode;
                skip_decode = skip_decode || parsed_skip_decode;
                skip_default = skip_default || parsed_skip_default;
                if let Some(rename_val) = parsed_rename {
                    rename = Some(rename_val);
                }
            } else {
                eprintln!(
                    "Warning: #[senax(...)] attribute for field '{}' is not in the correct format.",
                    field_name
                );
            }
        }
    }

    // ID calculation: Use explicit ID if provided, otherwise calculate CRC64 from rename or field name
    let calculated_id = id.unwrap_or_else(|| {
        let name_for_id = if let Some(ref rename_val) = rename {
            rename_val.as_str()
        } else {
            field_name
        };
        calculate_id_from_name(name_for_id)
    });

    FieldAttributes {
        id: calculated_id,
        default,
        skip_encode,
        skip_decode,
        skip_default,
        rename,
    }
}

/// Check if a type is `Option<T>`
///
/// This helper function determines whether a given type is wrapped in an `Option`.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path
            .path
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "Option")
    } else {
        false
    }
}

/// Extract the inner type `T` from `Option<T>`
///
/// This helper function extracts the wrapped type from an `Option` type.
/// Returns `None` if the type is not an `Option`.
fn extract_inner_type_from_option(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if type_path
            .path
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "Option")
        {
            if let PathArguments::AngleBracketed(args) =
                &type_path.path.segments.last().unwrap().arguments
            {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return Some(inner_ty);
                }
            }
        }
    }
    None
}

/// Derive macro for implementing the `Encode` trait
///
/// This procedural macro automatically generates an implementation of the `Encode` trait
/// for structs and enums. It supports various field attributes for customizing the
/// encoding behavior.
///
/// # Supported Attributes
///
/// * `#[senax(id=N)]` - Set explicit field/variant ID
/// * `#[senax(skip_encode)]` - Skip field during encoding
/// * `#[senax(rename="name")]` - Use alternative name for ID calculation
///
/// # Examples
///
/// ```rust
/// #[derive(Encode)]
/// struct MyStruct {
///     #[senax(id=1)]
///     field1: i32,
///     #[senax(skip_encode)]
///     field2: String,
/// }
/// ```
#[proc_macro_derive(Encode, attributes(senax))]
pub fn derive_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut default_variant_checks = Vec::new();

    let encode_fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => {
                let mut field_encode = Vec::new();
                let mut used_ids_struct = HashSet::new();
                for f in &fields.named {
                    let field_name_str = f.ident.as_ref().unwrap().to_string();
                    let field_attrs = get_field_attributes(&f.attrs, &field_name_str);

                    // Skip fields marked with skip_encode
                    if field_attrs.skip_encode {
                        continue;
                    }

                    if !used_ids_struct.insert(field_attrs.id) {
                        panic!("Field ID (0x{:016X}) is duplicated for struct '{}'. Please specify a different ID for field '{}' using #[senax(id=...)].", field_attrs.id, name, field_name_str);
                    }

                    let field_ident = &f.ident;
                    let ty = &f.ty;
                    let is_option = is_option_type(ty);
                    let field_id = field_attrs.id;

                    if is_option {
                        field_encode.push(quote! {
                            if let Some(val) = &self.#field_ident {
                                senax_encoder::write_field_id_optimized(writer, #field_id)?;
                                senax_encoder::Encoder::encode(&val, writer)?;
                            }
                        });
                    } else if field_attrs.skip_default {
                        // For skip_default fields, check if the value is default before encoding
                        field_encode.push(quote! {
                            if senax_encoder::Encoder::is_default(&self.#field_ident) == false {
                                senax_encoder::write_field_id_optimized(writer, #field_id)?;
                                senax_encoder::Encoder::encode(&self.#field_ident, writer)?;
                            }
                        });
                    } else {
                        field_encode.push(quote! {
                            senax_encoder::write_field_id_optimized(writer, #field_id)?;
                            senax_encoder::Encoder::encode(&self.#field_ident, writer)?;
                        });
                    }
                }
                quote! {
                    writer.put_u8(senax_encoder::TAG_STRUCT_NAMED);
                    #(#field_encode)*
                    senax_encoder::write_field_id_optimized(writer, 0)?;
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_encode = fields.unnamed.iter().enumerate().map(|(i, _)| {
                    let index = syn::Index::from(i);
                    quote! {
                        senax_encoder::Encoder::encode(&self.#index, writer)?;
                    }
                });
                quote! {
                    writer.put_u8(senax_encoder::TAG_STRUCT_UNNAMED);
                    let count: usize = #field_count;
                    senax_encoder::Encoder::encode(&count, writer)?;
                    #(#field_encode)*
                }
            }
            Fields::Unit => quote! {
                writer.put_u8(senax_encoder::TAG_STRUCT_UNIT);
            },
        },
        Data::Enum(e) => {
            let mut variant_encode = Vec::new();
            let mut used_ids_enum = HashSet::new();

            for v in &e.variants {
                let variant_name_str = v.ident.to_string();
                let variant_attrs = get_field_attributes(&v.attrs, &variant_name_str);
                let variant_id = variant_attrs.id;
                let is_default_variant = has_default_attribute(&v.attrs);

                if !used_ids_enum.insert(variant_id) {
                    panic!("Variant ID (0x{:016X}) is duplicated for enum '{}'. Please specify a different ID for variant '{}' using #[senax(id=...)].", variant_id, name, variant_name_str);
                }

                let variant_ident = &v.ident;

                // Generate is_default check for this variant if it has #[default] attribute
                if is_default_variant {
                    match &v.fields {
                        Fields::Named(fields) => {
                            let field_idents: Vec<_> = fields
                                .named
                                .iter()
                                .map(|f| f.ident.as_ref().unwrap())
                                .collect();
                            let field_default_checks: Vec<_> = field_idents
                                .iter()
                                .map(|ident| {
                                    quote! { senax_encoder::Encoder::is_default(#ident) }
                                })
                                .collect();

                            if field_default_checks.is_empty() {
                                default_variant_checks.push(quote! {
                                    #name::#variant_ident { .. } => true,
                                });
                            } else {
                                default_variant_checks.push(quote! {
                                    #name::#variant_ident { #(#field_idents),* } => {
                                        #(#field_default_checks)&&*
                                    },
                                });
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let field_count = fields.unnamed.len();
                            let field_bindings: Vec<_> = (0..field_count)
                                .map(|i| Ident::new(&format!("field{}", i), Span::call_site()))
                                .collect();
                            let field_default_checks: Vec<_> = field_bindings
                                .iter()
                                .map(|binding| {
                                    quote! { senax_encoder::Encoder::is_default(#binding) }
                                })
                                .collect();

                            if field_default_checks.is_empty() {
                                default_variant_checks.push(quote! {
                                    #name::#variant_ident(..) => true,
                                });
                            } else {
                                default_variant_checks.push(quote! {
                                    #name::#variant_ident(#(#field_bindings),*) => {
                                        #(#field_default_checks)&&*
                                    },
                                });
                            }
                        }
                        Fields::Unit => {
                            default_variant_checks.push(quote! {
                                #name::#variant_ident => true,
                            });
                        }
                    }
                }

                match &v.fields {
                    Fields::Named(fields) => {
                        let field_idents: Vec<_> = fields
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();
                        let mut field_encode = Vec::new();
                        let mut used_ids_struct = HashSet::new();
                        for f in &fields.named {
                            let field_name_str = f.ident.as_ref().unwrap().to_string();
                            let field_attrs = get_field_attributes(&f.attrs, &field_name_str);

                            // Skip fields marked with skip_encode
                            if field_attrs.skip_encode {
                                continue;
                            }

                            if !used_ids_struct.insert(field_attrs.id) {
                                panic!("Field ID (0x{:016X}) is duplicated for enum variant '{}'. Please specify a different ID for field '{}' using #[senax(id=...)].", field_attrs.id, variant_ident, field_name_str);
                            }
                            let field_ident = &f.ident;
                            let ty = &f.ty;
                            let is_option = is_option_type(ty);
                            let field_id = field_attrs.id;
                            if is_option {
                                field_encode.push(quote! {
                                    if let Some(val) = #field_ident {
                                        senax_encoder::write_field_id_optimized(writer, #field_id)?;
                                        senax_encoder::Encoder::encode(&val, writer)?;
                                    }
                                });
                            } else if field_attrs.skip_default {
                                // For skip_default fields, check if the value is default before encoding
                                field_encode.push(quote! {
                                    if senax_encoder::Encoder::is_default(#field_ident) == false {
                                        senax_encoder::write_field_id_optimized(writer, #field_id)?;
                                        senax_encoder::Encoder::encode(&#field_ident, writer)?;
                                    }
                                });
                            } else {
                                field_encode.push(quote! {
                                    senax_encoder::write_field_id_optimized(writer, #field_id)?;
                                    senax_encoder::Encoder::encode(&#field_ident, writer)?;
                                });
                            }
                        }
                        variant_encode.push(quote! {
                            #name::#variant_ident { #(#field_idents),* } => {
                                writer.put_u8(senax_encoder::TAG_ENUM_NAMED);
                                senax_encoder::write_field_id_optimized(writer, #variant_id)?;
                                #(#field_encode)*
                                senax_encoder::write_field_id_optimized(writer, 0)?;
                            }
                        });
                    }
                    Fields::Unnamed(fields) => {
                        let field_count = fields.unnamed.len();
                        let field_bindings: Vec<_> = (0..field_count)
                            .map(|i| Ident::new(&format!("field{}", i), Span::call_site()))
                            .collect();
                        let field_bindings_ref = &field_bindings;
                        variant_encode.push(quote! {
                            #name::#variant_ident( #(#field_bindings_ref),* ) => {
                                writer.put_u8(senax_encoder::TAG_ENUM_UNNAMED);
                                senax_encoder::write_field_id_optimized(writer, #variant_id)?;
                                let count: usize = #field_count;
                                senax_encoder::Encoder::encode(&count, writer)?;
                                #(
                                    senax_encoder::Encoder::encode(&#field_bindings_ref, writer)?;
                                )*
                            }
                        });
                    }
                    Fields::Unit => {
                        variant_encode.push(quote! {
                            #name::#variant_ident => {
                                writer.put_u8(senax_encoder::TAG_ENUM);
                                senax_encoder::write_field_id_optimized(writer, #variant_id)?;
                            }
                        });
                    }
                }
            }
            quote! {
                match self {
                    #(#variant_encode)*
                }
            }
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    };

    let is_default_impl = match &input.data {
        Data::Enum(_) => {
            if default_variant_checks.is_empty() {
                quote! { false }
            } else {
                quote! {
                    match self {
                        #(#default_variant_checks)*
                        _ => false,
                    }
                }
            }
        }
        _ => quote! { false },
    };

    TokenStream::from(quote! {
        impl #impl_generics senax_encoder::Encoder for #name #ty_generics #where_clause {
            fn encode(&self, writer: &mut bytes::BytesMut) -> senax_encoder::Result<()> {
                use bytes::{Buf, BufMut};
                #encode_fields
                Ok(())
            }

            fn is_default(&self) -> bool {
                #is_default_impl
            }
        }
    })
}

/// Derive macro for implementing the `Decode` trait
///
/// This procedural macro automatically generates an implementation of the `Decode` trait
/// for structs and enums. It supports various field attributes for customizing the
/// decoding behavior and provides forward/backward compatibility.
///
/// # Supported Attributes
///
/// * `#[senax(id=N)]` - Set explicit field/variant ID
/// * `#[senax(default)]` - Use default value if field is missing
/// * `#[senax(skip_decode)]` - Skip field during decoding (use default value)
/// * `#[senax(skip_default)]` - Use default value if field is missing (same as default for decode)
/// * `#[senax(rename="name")]` - Use alternative name for ID calculation
///
/// # Examples
///
/// ```rust
/// #[derive(Decode)]
/// struct MyStruct {
///     #[senax(id=1)]
///     field1: i32,
///     #[senax(default)]
///     field2: String,
///     #[senax(skip_decode)]
///     field3: bool,
/// }
/// ```
#[proc_macro_derive(Decode, attributes(senax))]
pub fn derive_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let decode_fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => {
                let mut field_idents = Vec::new();
                let mut field_original_types = Vec::new();
                let mut field_ids_for_match = Vec::new();
                let mut field_is_option_flags = Vec::new();
                let mut field_attrs_list = Vec::new();
                let mut used_ids_struct_decode = HashMap::new();

                for f in &fields.named {
                    let field_name_str = f.ident.as_ref().unwrap().to_string();
                    let field_attrs = get_field_attributes(&f.attrs, &field_name_str);

                    if let Some(dup_field_name) =
                        used_ids_struct_decode.insert(field_attrs.id, field_name_str.clone())
                    {
                        panic!("Field ID (0x{:016X}) is duplicated for struct '{}'. Please specify a different ID for field '{}' and '{}' using #[senax(id=...)].", 
                              field_attrs.id, name, dup_field_name, field_name_str);
                    }

                    field_idents.push(f.ident.as_ref().unwrap().clone());
                    field_original_types.push(f.ty.clone());
                    field_ids_for_match.push(field_attrs.id);
                    field_is_option_flags.push(is_option_type(&f.ty));
                    field_attrs_list.push(field_attrs);
                }

                let field_value_definitions = field_idents
                    .iter()
                    .zip(field_original_types.iter())
                    .zip(field_attrs_list.iter())
                    .filter_map(|((ident, original_ty), attrs)| {
                        if attrs.skip_decode {
                            // Fields marked with skip_decode don't store values
                            None
                        } else if is_option_type(original_ty) {
                            Some(quote! { #ident: #original_ty, })
                        } else {
                            Some(quote! { #ident: Option<#original_ty>, })
                        }
                    });

                let match_arms = field_idents
                    .iter()
                    .zip(field_original_types.iter())
                    .zip(field_ids_for_match.iter())
                    .zip(field_attrs_list.iter())
                    .filter_map(|(((ident, original_ty), id_val), attrs)| {
                        if attrs.skip_decode {
                            // Fields marked with skip_decode don't generate match arms (values are skipped)
                            None
                        } else if is_option_type(original_ty) {
                            let inner_ty = extract_inner_type_from_option(original_ty)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "Failed to extract inner type from Option for field {}",
                                        ident
                                    )
                                });
                            Some(quote! {
                                x if x == #id_val => {
                                    field_values.#ident = Some(<#inner_ty as senax_encoder::Decoder>::decode(reader)?);
                                }
                            })
                        } else {
                            Some(quote! {
                                x if x == #id_val => {
                                    field_values.#ident = Some(<#original_ty as senax_encoder::Decoder>::decode(reader)?);
                                }
                            })
                        }
                    });

                let struct_assignments = field_idents.iter()
                    .zip(field_is_option_flags.iter())
                    .zip(field_attrs_list.iter())
                    .map(|((ident, is_opt_flag), attrs)| {
                        if attrs.skip_decode {
                            // Fields marked with skip_decode use default values
                            quote! {
                                #ident: Default::default(),
                            }
                        } else if *is_opt_flag {
                            quote! {
                                #ident: field_values.#ident,
                            }
                        } else if attrs.default || attrs.skip_default {
                            // Fields marked with default or skip_default use default value if missing
                            quote! {
                                #ident: field_values.#ident.unwrap_or_default(),
                            }
                        } else {
                            quote! {
                                #ident: field_values.#ident.ok_or_else(||
                                    senax_encoder::EncoderError::Decode(format!("Required field '{}' not found for struct {}", stringify!(#ident), stringify!(#name)))
                                )?,
                            }
                        }
                    });

                quote! {
                    if reader.remaining() == 0 {
                        return Err(senax_encoder::EncoderError::InsufficientData);
                    }
                    let tag = reader.get_u8();
                    if tag != senax_encoder::TAG_STRUCT_NAMED {
                        return Err(senax_encoder::EncoderError::Decode(format!("Expected struct named tag ({}), got {}", senax_encoder::TAG_STRUCT_NAMED, tag)));
                    }

                    #[derive(Default)]
                    struct FieldValues {
                        #( #field_value_definitions )*
                    }

                    let mut field_values = FieldValues::default();

                    loop {
                        let field_id = senax_encoder::read_field_id_optimized(reader)?;
                        if field_id == 0 {
                            break;
                        }
                        match field_id {
                            #( #match_arms )*
                            _unknown_id => { senax_encoder::skip_value(reader)?; }
                        }
                    }

                    Ok(#name {
                        #( #struct_assignments )*
                    })
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_deencode = fields.unnamed.iter().map(|f| {
                    let field_ty = &f.ty;
                    quote! {
                        <#field_ty as senax_encoder::Decoder>::decode(reader)?
                    }
                });
                quote! {
                    if reader.remaining() == 0 {
                        return Err(senax_encoder::EncoderError::InsufficientData);
                    }
                    let tag = reader.get_u8();
                    if tag != senax_encoder::TAG_STRUCT_UNNAMED {
                        return Err(senax_encoder::EncoderError::Decode(format!("Expected struct unnamed tag ({}), got {}", senax_encoder::TAG_STRUCT_UNNAMED, tag)));
                    }
                    let count = <usize as senax_encoder::Decoder>::decode(reader)?;
                    if count != #field_count {
                        return Err(senax_encoder::EncoderError::Decode(format!("Field count mismatch for struct {}: expected {}, got {}", stringify!(#name), #field_count, count)));
                    }
                    Ok(#name(
                        #(#field_deencode),*
                    ))
                }
            }
            Fields::Unit => quote! {
                if reader.remaining() == 0 {
                    return Err(senax_encoder::EncoderError::InsufficientData);
                }
                let tag = reader.get_u8();
                if tag != senax_encoder::TAG_STRUCT_UNIT {
                    return Err(senax_encoder::EncoderError::Decode(format!("Expected struct unit tag ({}), got {}", senax_encoder::TAG_STRUCT_UNIT, tag)));
                }
                Ok(#name)
            },
        },
        Data::Enum(e) => {
            let mut unit_variant_arms = Vec::new();
            let mut named_variant_arms = Vec::new();
            let mut unnamed_variant_arms = Vec::new();
            let mut used_ids_enum_decode = HashMap::new();

            for v in &e.variants {
                let variant_name_str = v.ident.to_string();
                let variant_attrs = get_field_attributes(&v.attrs, &variant_name_str);
                let variant_id = variant_attrs.id;

                if let Some(dup_variant) =
                    used_ids_enum_decode.insert(variant_id, variant_name_str.clone())
                {
                    panic!("Variant ID (0x{:016X}) is duplicated for enum '{}'. Please specify a different ID for variant '{}' and '{}' using #[senax(id=...)].", 
                          variant_id, name, dup_variant, variant_name_str);
                }

                let variant_ident = &v.ident;
                match &v.fields {
                    Fields::Named(fields) => {
                        let field_idents: Vec<_> = fields
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap().clone())
                            .collect();
                        let field_types: Vec<_> =
                            fields.named.iter().map(|f| f.ty.clone()).collect();
                        let field_attrs_list: Vec<_> = fields
                            .named
                            .iter()
                            .map(|f| {
                                get_field_attributes(
                                    &f.attrs,
                                    &f.ident.as_ref().unwrap().to_string(),
                                )
                            })
                            .collect();

                        let mut field_value_definitions_enum = Vec::new();
                        let mut match_arms_enum_named = Vec::new();
                        let mut struct_assignments_enum_named = Vec::new();

                        for (ident, ty, attrs) in izip!(
                            field_idents.iter(),
                            field_types.iter(),
                            field_attrs_list.iter()
                        ) {
                            if attrs.skip_decode {
                                // Fields marked with skip_decode don't store values
                            } else if is_option_type(ty) {
                                field_value_definitions_enum.push(quote! { #ident: #ty, });
                            } else {
                                field_value_definitions_enum.push(quote! { #ident: Option<#ty>, });
                            }

                            if attrs.skip_decode {
                                // Fields marked with skip_decode don't generate match arms
                            } else if is_option_type(ty) {
                                let inner_ty = extract_inner_type_from_option(ty).unwrap();
                                let field_id = attrs.id;
                                match_arms_enum_named.push(quote! {
                                    x if x == #field_id => { field_values.#ident = Some(<#inner_ty as senax_encoder::Decoder>::decode(reader)?); }
                                });
                            } else {
                                let field_id = attrs.id;
                                match_arms_enum_named.push(quote! {
                                    x if x == #field_id => { field_values.#ident = Some(<#ty as senax_encoder::Decoder>::decode(reader)?); }
                                });
                            }

                            if attrs.skip_decode {
                                // Fields marked with skip_decode use default values
                                struct_assignments_enum_named
                                    .push(quote! { #ident: Default::default(), });
                            } else if is_option_type(ty) {
                                struct_assignments_enum_named
                                    .push(quote! { #ident: field_values.#ident, });
                            } else if attrs.default || attrs.skip_default {
                                // Fields marked with default or skip_default use default value if missing
                                struct_assignments_enum_named.push(quote! {
                                    #ident: field_values.#ident.unwrap_or_default(),
                                });
                            } else {
                                struct_assignments_enum_named.push(quote! {
                                    #ident: field_values.#ident.ok_or_else(|| senax_encoder::EncoderError::Decode(format!("Required field '{}' not found for variant {}::{}", stringify!(#ident), stringify!(#name), stringify!(#variant_ident))))?,
                                });
                            }
                        }

                        named_variant_arms.push(quote! {
                            x if x == #variant_id => {
                                #[derive(Default)]
                                struct FieldValues { #(#field_value_definitions_enum)* }
                                let mut field_values = FieldValues::default();
                                loop {
                                    let field_id = {
                                        if reader.remaining() == 0 { break; }
                                        let id = senax_encoder::read_field_id_optimized(reader)?;
                                        if id == 0 { break; }
                                        id
                                    };
                                    match field_id {
                                        #(#match_arms_enum_named)*
                                        _unknown_id => { senax_encoder::skip_value(reader)?; }
                                    }
                                }
                                Ok(#name::#variant_ident { #(#struct_assignments_enum_named)* })
                            }
                        });
                    }
                    Fields::Unnamed(fields) => {
                        let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
                        let field_count = field_types.len();
                        unnamed_variant_arms.push(quote! {
                            x if x == #variant_id => {
                                let count = <usize as senax_encoder::Decoder>::decode(reader)?;
                                if count != #field_count {
                                    return Err(senax_encoder::EncoderError::Decode(format!("Field count mismatch for variant {}::{}: expected {}, got {}", stringify!(#name), stringify!(#variant_ident), #field_count, count)));
                                }
                                Ok(#name::#variant_ident(
                                    #(
                                        <#field_types as senax_encoder::Decoder>::decode(reader)?,
                                    )*
                                ))
                            }
                        });
                    }
                    Fields::Unit => {
                        unit_variant_arms.push(quote! {
                            x if x == #variant_id => {
                                Ok(#name::#variant_ident)
                            }
                        });
                    }
                }
            }
            quote! {
                if reader.remaining() == 0 {
                    return Err(senax_encoder::EncoderError::InsufficientData);
                }
                let tag = reader.get_u8();
                match tag {
                    senax_encoder::TAG_ENUM => {
                        let variant_id = senax_encoder::read_field_id_optimized(reader)?;
                        match variant_id {
                            #(#unit_variant_arms)*
                            _ => Err(senax_encoder::EncoderError::Decode(format!("Unknown unit variant ID: 0x{:016X} for enum {}", variant_id, stringify!(#name))))
                        }
                    }
                    senax_encoder::TAG_ENUM_NAMED => {
                        let variant_id = senax_encoder::read_field_id_optimized(reader)?;
                        match variant_id {
                            #(#named_variant_arms)*
                            _ => Err(senax_encoder::EncoderError::Decode(format!("Unknown named variant ID: 0x{:016X} for enum {}", variant_id, stringify!(#name))))
                        }
                    }
                    senax_encoder::TAG_ENUM_UNNAMED => {
                        let variant_id = senax_encoder::read_field_id_optimized(reader)?;
                        match variant_id {
                             #(#unnamed_variant_arms)*
                            _ => Err(senax_encoder::EncoderError::Decode(format!("Unknown unnamed variant ID: 0x{:016X} for enum {}", variant_id, stringify!(#name))))
                        }
                    }
                    unknown_tag => Err(senax_encoder::EncoderError::Decode(format!("Unknown enum tag: {} for enum {}", unknown_tag, stringify!(#name))))
                }
            }
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    };

    TokenStream::from(quote! {
        impl #impl_generics senax_encoder::Decoder for #name #ty_generics #where_clause {
            fn decode(reader: &mut bytes::Bytes) -> senax_encoder::Result<Self> {
                use bytes::{Buf, BufMut};
                #decode_fields
            }
        }
    })
}
