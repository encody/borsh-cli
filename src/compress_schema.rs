use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Write,
    ops::DerefMut,
};

use borsh::{
    schema::{BorshSchemaContainer, Definition, Fields},
    BorshDeserialize, BorshSchema, BorshSerialize,
};
use serde::{Deserialize, Serialize};

use crate::dynamic_schema::{self, serialize_with_schema};

fn next_name(next_name_code: &mut u32) -> String {
    let mut c = None;
    while let None = c {
        c = char::from_u32(*next_name_code);
        *next_name_code += 1;
    }

    c.unwrap().to_string()
}

trait InnerTypes {
    fn get_inner_definitions(&self) -> Vec<&str>;
}

impl InnerTypes for Definition {
    fn get_inner_definitions(&self) -> Vec<&str> {
        match self {
            Definition::Array { elements, .. } => vec![elements.as_str()],
            Definition::Sequence { elements } => vec![elements.as_str()],
            Definition::Tuple { elements } => elements.iter().map(|d| d.as_str()).collect(),
            Definition::Enum { variants } => variants.iter().map(|(_, d)| d.as_str()).collect(),
            Definition::Struct { fields } => fields.get_inner_definitions(),
        }
    }
}

impl InnerTypes for Fields {
    fn get_inner_definitions(&self) -> Vec<&str> {
        match self {
            Fields::NamedFields(named_fields) => {
                named_fields.iter().map(|(_, d)| d.as_str()).collect()
            }
            Fields::UnnamedFields(unnamed_fields) => {
                unnamed_fields.iter().map(|d| d.as_str()).collect()
            }
            Fields::Empty => vec![],
        }
    }
}

fn add_definitions_rec(
    new_definitions: &mut HashMap<String, Definition>,
    current: &str,
    old_definitions: &HashMap<String, Definition>,
    old_to_new_map: &HashMap<&str, String>,
) {
    let get = |n: &str| {
        old_to_new_map
            .get(n)
            .map(|s| s.to_string())
            .unwrap_or(n.to_string())
    };
    let new_name = old_to_new_map.get(current).cloned();

    let old_definition = old_definitions.get(current);

    if let (Some(new_name), Some(old_definition)) = (new_name, old_definition) {
        match old_definition {
            Definition::Array { length, elements } => {
                new_definitions.insert(
                    new_name,
                    Definition::Array {
                        length: *length,
                        elements: get(elements),
                    },
                );
            }
            Definition::Sequence { elements } => {
                new_definitions.insert(
                    new_name,
                    Definition::Sequence {
                        elements: get(elements),
                    },
                );
            }
            Definition::Tuple { elements } => {
                new_definitions.insert(
                    new_name,
                    Definition::Tuple {
                        elements: elements.iter().map(|t| get(t)).collect(),
                    },
                );
            }
            Definition::Enum { variants } => {
                let new_variants = variants
                    .iter()
                    .map(|(name, d)| (name.to_string(), get(d)))
                    .collect();
                new_definitions.insert(
                    new_name,
                    Definition::Enum {
                        variants: new_variants,
                    },
                );
            }
            Definition::Struct { fields } => {
                let new_fields = match fields {
                    Fields::NamedFields(named_fields) => Fields::NamedFields(
                        named_fields
                            .iter()
                            .map(|(name, definition)| {
                                let new = get(definition);
                                (name.clone(), new.clone())
                            })
                            .collect(),
                    ),
                    Fields::UnnamedFields(unnamed_fields) => {
                        Fields::UnnamedFields(unnamed_fields.iter().map(|s| get(s)).collect())
                    }
                    Fields::Empty => Fields::Empty,
                };

                new_definitions.insert(new_name, Definition::Struct { fields: new_fields });
            }
        }

        // add inner definitions as well
        for i in old_definition.get_inner_definitions() {
            add_definitions_rec(new_definitions, i, old_definitions, old_to_new_map);
        }
    }
}

pub fn compress_schema(schema: &BorshSchemaContainer) -> BorshSchemaContainer {
    let mut old_to_new_map = HashMap::new();
    let mut next_name_code = 0;

    let mut stack: Vec<&str> = vec![&schema.declaration];

    while let Some(old_name) = stack.pop() {
        if let "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "string" | "bool" = old_name
        {
            continue;
        }
        // four options:
        // - functional tail recursion (bad space efficiency, plus complexity of merging state)
        // - iterative tail recursion (bad space efficiency, plus complexity of managing tail stack)
        // - moving name calculation up one step (code duplication)
        // - second loop (I think best)
        match old_to_new_map.entry(old_name) {
            Entry::Vacant(entry) => {
                let new_name = next_name(&mut next_name_code);
                entry.insert(new_name);
            }
            _ => continue,
        };
        let definition = if let Some(definition) = schema.definitions.get(old_name) {
            definition
        } else {
            continue;
        };
        match definition {
            Definition::Sequence { elements } | Definition::Array { elements, .. } => {
                stack.push(elements)
            }
            Definition::Tuple { elements } => stack.extend(elements.iter().map(|s| s.as_str())),
            // I think that compressing variant names and struct field names is
            // probably a bad idea, since that will show up when deserializing data into JSON, for example.
            Definition::Enum { variants } => {
                stack.extend(variants.iter().map(|(_, d)| d.as_str()));
            }
            Definition::Struct { fields } => {
                stack.extend(match fields {
                    Fields::NamedFields(named_fields) => {
                        named_fields.iter().map(|(_, d)| d.as_str()).collect()
                    }
                    Fields::UnnamedFields(unnamed_fields) => {
                        unnamed_fields.iter().map(|s| s.as_str()).collect()
                    }
                    Fields::Empty => vec![],
                });
            }
        }
    }

    let mut new_definitions = HashMap::new();

    add_definitions_rec(
        &mut new_definitions,
        &schema.declaration,
        &schema.definitions,
        &old_to_new_map,
    );

    BorshSchemaContainer {
        declaration: old_to_new_map
            .get(schema.declaration.as_str())
            .unwrap()
            .to_string(),
        definitions: new_definitions,
    }
}

#[test]
fn test() {
    println!(
        "{:?}",
        (0..1000)
            .filter_map(|i: u32| char::from_u32(i).map(|c| c.to_string()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test2() {
    #[derive(
        BorshSerialize,
        BorshDeserialize,
        BorshSchema,
        Default,
        PartialEq,
        Debug,
        Serialize,
        Deserialize,
    )]
    struct Hello {
        number: i32,
        string: String,
        child: Child,
        child2: Child,
        child3: Child,
        // map: HashMap<u32, Child>,
        // vector: Vec<String>,
    }

    #[derive(
        BorshSerialize,
        BorshDeserialize,
        BorshSchema,
        Default,
        PartialEq,
        Debug,
        Serialize,
        Deserialize,
    )]
    struct Child {
        number: i32,
        string: String,
    }

    let schema_container = Hello::schema_container();
    println!("{schema_container:?}");
    let compressed = compress_schema(&schema_container);
    println!("{compressed:?}");

    let value = Hello {
        number: 6,
        string: "my string".to_string(),
        child: Child {
            string: "boom chakalaka".to_string(),
            number: 108,
            ..Default::default()
        },
        ..Default::default()
    };

    let normal_serialization = schema_container.try_to_vec().unwrap();
    println!(
        "normal serialization length: {}",
        normal_serialization.len()
    );
    let normal_deserialized: BorshSchemaContainer =
        BorshDeserialize::try_from_slice(&normal_serialization).unwrap();
    assert_eq!(normal_deserialized, schema_container);

    let compressed_serialization = compressed.try_to_vec().unwrap();
    println!(
        "normal serialization length: {}",
        compressed_serialization.len()
    );
    let compressed_deserialized: BorshSchemaContainer =
        BorshDeserialize::try_from_slice(&compressed_serialization).unwrap();

    let serialized_value = BorshSerialize::try_to_vec(&value).unwrap();
    let mut buf = &serialized_value as &[u8];
    let deserialized_with_schema =
        dynamic_schema::deserialize_from_schema(&mut buf, &compressed_deserialized).unwrap();

    assert_eq!(
        deserialized_with_schema,
        serde_json::to_value(&value).unwrap()
    );
    // assert_eq!(compressed_deserialized, schema_container);
}
