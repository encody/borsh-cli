use std::io::{self, Write};

use borsh::BorshSerialize;

/// Newtype wraps serde_json::Value because it does not implement BorshSerialize
#[derive(Debug)]
pub(crate) struct JsonSerializableAsBorsh<'a>(pub &'a serde_json::Value);

impl<'a> BorshSerialize for JsonSerializableAsBorsh<'a> {
    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match &self.0 {
            // Since we don't have a schema, we're kind of flying blind. We
            // don't know when a value is nullable. If a value is nullable, it
            // has a different serialization from a non-nullable value.
            serde_json::Value::Null => BorshSerialize::serialize(&None::<()>, writer),
            serde_json::Value::Bool(b) => BorshSerialize::serialize(&b, writer),
            serde_json::Value::Number(ref n) => {
                if let Some(f) = n.as_f64() {
                    BorshSerialize::serialize(&f, writer)
                } else if let Some(u) = n.as_u64() {
                    BorshSerialize::serialize(&u, writer)
                } else if let Some(i) = n.as_i64() {
                    BorshSerialize::serialize(&i, writer)
                } else {
                    // This is essentially an exhaustive match expression, but serde_json::number::N is private
                    unreachable!()
                }
            }
            serde_json::Value::String(s) => BorshSerialize::serialize(&s, writer),
            serde_json::Value::Array(v) => BorshSerialize::serialize(
                &v.iter().map(JsonSerializableAsBorsh).collect::<Vec<_>>(),
                writer,
            ),
            serde_json::Value::Object(o) => o.iter().try_fold((), |_, (_, v)| {
                BorshSerialize::serialize(&JsonSerializableAsBorsh(v), writer)
            }),
        }
    }
}
