use std::collections::HashMap;
use std::io::{Error, Write};
use std::str::FromStr;

use anyhow::anyhow;
use borsh::schema::{BorshSchemaContainer, Definition, Fields};
use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::json;
use thiserror::Error;

fn deserialize_type<T: BorshDeserialize + Into<serde_json::Value>>(
    buf: &mut &[u8],
    type_name: &str,
) -> std::io::Result<serde_json::Value> {
    T::deserialize(buf)
        .map(Into::into)
        .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, type_name))
}

fn deserialize_declaration_from_schema(
    buf: &mut &[u8],
    schema: &BorshSchemaContainer,
    declaration: &borsh::schema::Declaration,
) -> std::io::Result<serde_json::Value> {
    match &declaration[..] {
        "u8" => deserialize_type::<u8>(buf, "u8"),
        "u16" => deserialize_type::<u16>(buf, "u16"),
        "u32" => deserialize_type::<u32>(buf, "u32"),
        "u64" => deserialize_type::<u64>(buf, "u64"),
        "u128" => u128::deserialize(buf)
            .map(|x| x.to_string().into())
            .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, "u128")),
        "i8" => deserialize_type::<i8>(buf, "i8"),
        "i16" => deserialize_type::<i16>(buf, "i16"),
        "i32" => deserialize_type::<i32>(buf, "i32"),
        "i64" => deserialize_type::<i64>(buf, "i64"),
        "i128" => i128::deserialize(buf)
            .map(|x| x.to_string().into())
            .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, "i128")),
        "f32" => deserialize_type::<f32>(buf, "f32"),
        "f64" => deserialize_type::<f64>(buf, "f64"),
        "string" => deserialize_type::<String>(buf, "string"),
        "bool" => deserialize_type::<bool>(buf, "bool"),

        _ => {
            if let Some(d) = schema.definitions.get(declaration) {
                match d {
                    Definition::Array {
                        length,
                        ref elements,
                    } => {
                        let mut v = Vec::<serde_json::Value>::with_capacity(*length as usize);
                        for _ in 0..*length {
                            let e = deserialize_declaration_from_schema(buf, schema, elements)?;
                            v.push(e);
                        }
                        Ok(v.into())
                    }
                    Definition::Sequence { elements } => {
                        let length = u32::deserialize(buf)?;
                        let mut v = Vec::<serde_json::Value>::with_capacity(length as usize);
                        for _ in 0..length {
                            let e = deserialize_declaration_from_schema(buf, schema, elements)?;
                            v.push(e);
                        }
                        Ok(v.into())
                    }
                    Definition::Tuple { elements } => {
                        // try_collect not stable :'(
                        let mut v = Vec::<serde_json::Value>::with_capacity(elements.len());
                        for element in elements {
                            let e = deserialize_declaration_from_schema(buf, schema, element)?;
                            v.push(e);
                        }
                        Ok(v.into())
                    }
                    Definition::Enum { variants } => {
                        let variant_index = u8::deserialize(buf)?;
                        let (variant_name, variant_declaration) = &variants[variant_index as usize];
                        deserialize_declaration_from_schema(buf, schema, variant_declaration)
                            .map(|v| json!({ variant_name: v }))
                    }
                    Definition::Struct { fields } => match fields {
                        Fields::NamedFields(fields) => {
                            let mut object = HashMap::<String, serde_json::Value>::new();
                            for (key, value_declaration) in fields {
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
                                let e = deserialize_declaration_from_schema(buf, schema, element)?;
                                v.push(e);
                            }
                            Ok(v.into())
                        }
                        Fields::Empty => Ok(Vec::<u8>::new().into()),
                    },
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

fn serialize_signed<T: BorshSerialize + TryFrom<i64>>(
    writer: &mut impl Write,
    value: &serde_json::Value,
) -> anyhow::Result<()>
where
    <T as TryFrom<i64>>::Error: std::error::Error + Send + Sync + 'static,
{
    let v = value
        .as_i64()
        .ok_or(ExpectationError::Number)
        .map(T::try_from)??;
    BorshSerialize::serialize(&v, writer)?;
    Ok(())
}

fn serialize_unsigned<T: BorshSerialize + TryFrom<u64>>(
    writer: &mut impl Write,
    value: &serde_json::Value,
) -> anyhow::Result<()>
where
    <T as TryFrom<u64>>::Error: std::error::Error + Send + Sync + 'static,
{
    let v = value
        .as_u64()
        .ok_or(ExpectationError::Number)
        .map(T::try_from)??;
    BorshSerialize::serialize(&v, writer)?;
    Ok(())
}

fn serialize_string<T: BorshSerialize + FromStr>(
    writer: &mut impl Write,
    value: &serde_json::Value,
) -> anyhow::Result<()>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    let v = value
        .as_str()
        .ok_or(ExpectationError::String)
        .map(T::from_str)??;
    BorshSerialize::serialize(&v, writer)?;
    Ok(())
}

fn serialize_declaration_with_schema(
    writer: &mut impl Write,
    value: &serde_json::Value,
    schema: &BorshSchemaContainer,
    declaration: &borsh::schema::Declaration,
) -> anyhow::Result<()> {
    match &declaration[..] {
        "u8" => serialize_unsigned::<u8>(writer, value),
        "u16" => serialize_unsigned::<u16>(writer, value),
        "u32" => serialize_unsigned::<u32>(writer, value),
        "u64" => serialize_unsigned::<u64>(writer, value),
        "u128" => serialize_string::<u128>(writer, value),
        "i8" => serialize_signed::<i8>(writer, value),
        "i16" => serialize_signed::<i16>(writer, value),
        "i32" => serialize_signed::<i32>(writer, value),
        "i64" => serialize_signed::<i64>(writer, value),
        "i128" => serialize_string::<i128>(writer, value),
        "string" => serialize_string::<String>(writer, value),
        "bool" => {
            let v = value.as_bool().ok_or(ExpectationError::Boolean)?;
            BorshSerialize::serialize(&v, writer)?;
            Ok(())
        }
        _ => {
            if let Some(definition) = schema.definitions.get(declaration) {
                match definition {
                    Definition::Array { length, elements } => {
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
                        let sequence = value.as_array().ok_or(ExpectationError::Array)?;
                        BorshSerialize::serialize(&(sequence.len() as u32), writer)?;
                        for item in sequence {
                            serialize_declaration_with_schema(writer, item, schema, elements)?;
                        }
                        Ok(())
                    }
                    Definition::Tuple { elements } => {
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
                        let (input_variant, variant_values) = value
                            .as_object()
                            .and_then(|o| o.keys().next().map(|s| (s.as_str(), Some(&o[s]))))
                            .or_else(|| value.as_str().map(|s| (s, None)))
                            .ok_or(ExpectationError::Object)?;

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
                                serialize_declaration_with_schema(
                                    writer, value, schema, &fields[0],
                                )?;
                                return Ok(());
                            }

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
