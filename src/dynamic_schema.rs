use std::collections::HashMap;
use std::io::{Error, Write};

use anyhow::anyhow;
use borsh::schema::{BorshSchemaContainer, Definition, Fields};
use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::json;
use thiserror::Error;

pub fn deserialize_declaration_from_schema(
    buf: &mut &[u8],
    schema: &BorshSchemaContainer,
    declaration: &borsh::schema::Declaration,
) -> std::io::Result<serde_json::Value> {
    match &declaration[..] {
        "u8" => u8::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "u8")),
        "u16" => u16::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "u16")),
        "u32" => u32::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "u32")),
        "u64" => u64::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "u64")),
        "u128" => u128::deserialize(buf)
            .map(|x| x.to_string().into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "u128")),
        "i8" => i8::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "i8")),
        "i16" => i16::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "i16")),
        "i32" => i32::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "i32")),
        "i64" => i64::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "i64")),
        "i128" => i128::deserialize(buf)
            .map(|x| x.to_string().into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "i128")),
        "string" => String::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "string")),
        "bool" => bool::deserialize(buf)
            .map(|x| x.into())
            .map_err(|_e| Error::new(std::io::ErrorKind::InvalidData, "bool")),

        _ => {
            if let Some(d) = schema.definitions.get(declaration) {
                match d {
                    Definition::Array {
                        length,
                        ref elements,
                    } => {
                        dbg!("array");
                        let mut v = Vec::<serde_json::Value>::with_capacity(*length as usize);
                        for _ in 0..*length {
                            let e = deserialize_declaration_from_schema(buf, schema, elements)?;
                            v.push(e);
                        }
                        Ok(v.into())
                    }
                    Definition::Sequence { elements } => {
                        dbg!("sequence");
                        let length = u32::deserialize(buf)?;
                        let mut v = Vec::<serde_json::Value>::with_capacity(length as usize);
                        for _ in 0..length {
                            let e = deserialize_declaration_from_schema(buf, schema, elements)?;
                            v.push(e);
                        }
                        Ok(v.into())
                    }
                    Definition::Tuple { elements } => {
                        dbg!("tuple");
                        // try_collect not stable :'(
                        let mut v = Vec::<serde_json::Value>::with_capacity(elements.len());
                        for element in elements {
                            let e = deserialize_declaration_from_schema(buf, schema, element)?;
                            v.push(e);
                        }
                        Ok(v.into())
                    }
                    Definition::Enum { variants } => {
                        dbg!("enum");
                        let variant_index = u8::deserialize(buf)?;
                        let (variant_name, variant_declaration) = &variants[variant_index as usize];
                        deserialize_declaration_from_schema(buf, schema, variant_declaration)
                            .map(|v| json!({ variant_name: v }))
                    }
                    Definition::Struct { fields } => {
                        dbg!("struct");
                        match fields {
                            Fields::NamedFields(fields) => {
                                let mut object = HashMap::<String, serde_json::Value>::new();
                                for &(ref key, ref value_declaration) in fields {
                                    let value = deserialize_declaration_from_schema(
                                        buf,
                                        schema,
                                        value_declaration,
                                    )?;
                                    object.insert(key.to_string(), value);
                                }
                                Ok(serde_json::to_value(object)?)
                            }
                            Fields::UnnamedFields(elements) => {
                                let mut v = Vec::<serde_json::Value>::with_capacity(elements.len());
                                for element in elements {
                                    let e =
                                        deserialize_declaration_from_schema(buf, schema, element)?;
                                    v.push(e);
                                }
                                Ok(v.into())
                            }
                            Fields::Empty => Ok(Vec::<u8>::new().into()),
                        }
                    }
                }
            } else {
                todo!("Unknown type to deserialize: {declaration}")
            }
        }
    }
}

pub fn deserialize_from_schema(
    buf: &mut &[u8],
    schema: &BorshSchemaContainer,
) -> std::io::Result<serde_json::Value> {
    deserialize_declaration_from_schema(buf, schema, &schema.declaration)
}

#[derive(Error, Debug)]
pub enum ExpectationError {
    #[error("Expected number")]
    Number,
    #[error("Expected string")]
    String,
    #[error("Expected boolean")]
    Boolean,
    #[error("Expected array")]
    Array,
    #[error("Expected array of length {0}")]
    ArrayOfLength(u32),
    #[error("Expected object")]
    Object,
}

pub fn serialize_with_schema(
    writer: &mut impl Write,
    value: &serde_json::Value,
    schema: &BorshSchemaContainer,
) -> anyhow::Result<()> {
    serialize_declaration_with_schema(writer, value, schema, &schema.declaration)
}

pub fn serialize_declaration_with_schema(
    writer: &mut impl Write,
    value: &serde_json::Value,
    schema: &BorshSchemaContainer,
    declaration: &borsh::schema::Declaration,
) -> anyhow::Result<()> {
    match &declaration[..] {
        "u8" => {
            let v = value
                .as_u64()
                .ok_or(ExpectationError::Number)
                .map(u8::try_from)??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "u16" => {
            let v = value
                .as_u64()
                .ok_or(ExpectationError::Number)
                .map(u16::try_from)??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "u32" => {
            let v = value
                .as_u64()
                .ok_or(ExpectationError::Number)
                .map(u32::try_from)??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "u64" => {
            let v = value.as_u64().ok_or(ExpectationError::Number)?;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "u128" => {
            let v: u128 = value
                .as_str()
                .ok_or(ExpectationError::String)
                .map(|x| x.parse())??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "i8" => {
            let v = value
                .as_i64()
                .ok_or(ExpectationError::Number)
                .map(i8::try_from)??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "i16" => {
            let v = value
                .as_i64()
                .ok_or(ExpectationError::Number)
                .map(i16::try_from)??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "i32" => {
            let v = value
                .as_i64()
                .ok_or(ExpectationError::Number)
                .map(i32::try_from)??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "i64" => {
            let v = value.as_u64().ok_or(ExpectationError::Number)?;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "i128" => {
            let v: i128 = value
                .as_str()
                .ok_or(ExpectationError::String)
                .map(|x| x.parse())??;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "string" => {
            let v = value.as_str().ok_or(ExpectationError::String)?;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        "bool" => {
            let v = value.as_bool().ok_or(ExpectationError::Boolean)?;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        _ => {
            if let Some(definition) = schema.definitions.get(declaration) {
                match definition {
                    Definition::Array { length, elements } => {
                        dbg!("Definition::Array");
                        let array = value.as_array().ok_or(ExpectationError::Array)?;
                        if array.len() != *length as usize {
                            return Err(ExpectationError::ArrayOfLength(*length).into());
                        }
                        for value in array {
                            serialize_declaration_with_schema(writer, value, schema, elements)?;
                        }
                        Ok(())
                    }
                    Definition::Sequence { elements } => {
                        dbg!("Definition::Sequence");
                        let sequence = value.as_array().ok_or(ExpectationError::Array)?;
                        BorshSerialize::serialize(&(sequence.len() as u32), writer)?;
                        for item in sequence {
                            serialize_declaration_with_schema(writer, item, schema, elements)?;
                        }
                        Ok(())
                    }
                    Definition::Tuple { elements } => {
                        dbg!("Definition::Tuple");
                        let tuple = value.as_array().ok_or(ExpectationError::Array)?;
                        if tuple.len() != elements.len() {
                            // TODO: double-check the lack of casting to u32
                            return Err(
                                ExpectationError::ArrayOfLength(elements.len() as u32).into()
                            );
                        }
                        for (declaration, value) in elements.iter().zip(tuple) {
                            serialize_declaration_with_schema(writer, value, schema, declaration)?;
                        }
                        Ok(())
                    }
                    Definition::Enum { variants } => {
                        dbg!("enum {:?}", variants);
                        dbg!("{value:?}");
                        let (input_variant, variant_values) = value
                            .as_object()
                            .and_then(|o| o.keys().next().map(|s| (s.as_str(), Some(&o[s]))))
                            .or_else(|| value.as_str().map(|s| (s, None)))
                            .ok_or(ExpectationError::Object)?;

                        dbg!("{variant_values:?}");

                        let (variant_index, variant_declaration) = variants
                            .iter()
                            .enumerate()
                            .find_map(|(i, (k, v))| {
                                if k == input_variant {
                                    Some((i, v))
                                } else {
                                    None
                                }
                            })
                            .ok_or_else(|| {
                                anyhow!(
                                    "Specified variant {input_variant} does not exist in schema"
                                )
                            })?;

                        BorshSerialize::serialize(&(variant_index as u8), writer)?;
                        serialize_declaration_with_schema(
                            writer,
                            variant_values.unwrap_or(&json!({})),
                            schema,
                            variant_declaration,
                        )?;
                        Ok(())
                    }
                    Definition::Struct { fields } => match fields {
                        Fields::NamedFields(fields) => {
                            let object = value.as_object().ok_or(ExpectationError::Object)?;
                            for (key, value_declaration) in fields {
                                let property_value = object
                                    .get(key)
                                    .ok_or_else(|| anyhow!("Expected property {key}"))?;
                                serialize_declaration_with_schema(
                                    writer,
                                    property_value,
                                    schema,
                                    value_declaration,
                                )?;
                            }
                            Ok(())
                        }
                        Fields::UnnamedFields(fields) => {
                            if fields.len() == 1 {
                                dbg!("One unnamed field");
                                serialize_declaration_with_schema(
                                    writer, value, schema, &fields[0],
                                )?;
                                return Ok(());
                            }

                            dbg!("Multiple unnamed fields");

                            let array = value.as_array().ok_or(ExpectationError::Array)?;
                            if array.len() != fields.len() {
                                return Err(
                                    ExpectationError::ArrayOfLength(fields.len() as u32).into()
                                );
                            }
                            for (declaration, value) in fields.iter().zip(array) {
                                serialize_declaration_with_schema(
                                    writer,
                                    value,
                                    schema,
                                    declaration,
                                )?;
                            }
                            Ok(())
                        }
                        Fields::Empty => {
                            // Ignore everything
                            Ok(())
                        }
                    },
                }
            } else {
                todo!("Unknown declaration to serialize: {declaration}")
            }
        }
    }
}
