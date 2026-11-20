use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_value::Value;

use crate::lenient_de::Lenient;

#[cfg(feature = "json")]
use serde_json::{Number, Value as Json};

const TEXT_KEY: &str = "$text";
const DATETIME_KEY: &str = "$__toml_private_datetime";

/// A free-form document that reads alike from every format.
///
/// Repeated map keys are grouped into a list in document order, and a map whose
/// only key is `$text` collapses to that text.
///
/// ```rust
/// # #[cfg(all(feature = "xml", feature = "json"))]
/// # fn main() -> Result<(), conflaguration::Error> {
/// use conflaguration::Tree;
///
/// let tree: Tree = conflaguration::from_xml_str("<args><id>A</id><id>B</id></args>")?;
/// let json = serde_json::Value::from(tree);
/// assert_eq!(json.to_string(), r#"{"id":["A","B"]}"#);
/// # Ok(())
/// # }
/// # #[cfg(not(all(feature = "xml", feature = "json")))]
/// # fn main() {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tree(serde_value::Value);

impl<'de> Deserialize<'de> for Tree {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TreeVisitor)
    }
}

#[cfg(feature = "xml")]
impl Tree {
    pub(crate) fn into_value(self) -> Value {
        self.0
    }
}

struct TreeVisitor;

impl<'de> Visitor<'de> for TreeVisitor {
    type Value = Tree;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Tree, E> {
        Ok(Tree(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Tree, E> {
        Ok(Tree(Value::I64(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Tree, E> {
        Ok(Tree(Value::U64(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Tree, E> {
        Ok(Tree(Value::F64(value)))
    }

    fn visit_char<E>(self, value: char) -> Result<Tree, E> {
        Ok(Tree(Value::Char(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Tree, E> {
        Ok(Tree(Value::String(value.to_string())))
    }

    fn visit_string<E>(self, value: String) -> Result<Tree, E> {
        Ok(Tree(Value::String(value)))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Tree, E> {
        Ok(Tree(Value::Bytes(value.to_vec())))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Tree, E> {
        Ok(Tree(Value::Bytes(value)))
    }

    fn visit_unit<E>(self) -> Result<Tree, E> {
        Ok(Tree(Value::Unit))
    }

    fn visit_none<E>(self) -> Result<Tree, E> {
        Ok(Tree(Value::Option(None)))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Tree, D::Error> {
        let inner = Tree::deserialize(deserializer)?;
        Ok(Tree(Value::Option(Some(Box::new(inner.0)))))
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> Result<Tree, D::Error> {
        let inner = Tree::deserialize(deserializer)?;
        Ok(Tree(Value::Newtype(Box::new(inner.0))))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Tree, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = access.next_element::<Tree>()? {
            items.push(item.0);
        }
        Ok(Tree(Value::Seq(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Tree, A::Error> {
        let mut entries: BTreeMap<Value, Value> = BTreeMap::new();
        let mut grouped: BTreeSet<Value> = BTreeSet::new();
        while let Some((key, value)) = access.next_entry::<Tree, Tree>()? {
            match entries.remove(&key.0) {
                None => {
                    entries.insert(key.0, value.0);
                }
                Some(Value::Seq(mut items)) if grouped.contains(&key.0) => {
                    items.push(value.0);
                    entries.insert(key.0, Value::Seq(items));
                }
                Some(first) => {
                    grouped.insert(key.0.clone());
                    entries.insert(key.0, Value::Seq(vec![first, value.0]));
                }
            }
        }
        if entries.len() == 1 {
            let only = entries.keys().next().cloned();
            let collapses = matches!(&only, Some(Value::String(name)) if name == TEXT_KEY || name == DATETIME_KEY);
            if collapses && let Some(inner) = only.and_then(|key| entries.remove(&key)) {
                return Ok(Tree(inner));
            }
        }
        Ok(Tree(Value::Map(entries)))
    }
}

#[cfg(feature = "json")]
impl From<Tree> for serde_json::Value {
    fn from(tree: Tree) -> Self {
        to_json(tree.0)
    }
}

#[cfg(feature = "json")]
fn to_json(value: Value) -> serde_json::Value {
    match value {
        Value::Bool(flag) => Json::Bool(flag),
        Value::U8(number) => Json::from(number),
        Value::U16(number) => Json::from(number),
        Value::U32(number) => Json::from(number),
        Value::U64(number) => Json::from(number),
        Value::I8(number) => Json::from(number),
        Value::I16(number) => Json::from(number),
        Value::I32(number) => Json::from(number),
        Value::I64(number) => Json::from(number),
        Value::F32(number) => Number::from_f64(f64::from(number)).map_or(Json::Null, Json::Number),
        Value::F64(number) => Number::from_f64(number).map_or(Json::Null, Json::Number),
        Value::Char(letter) => Json::String(letter.to_string()),
        Value::String(text) => Json::String(text),
        Value::Unit | Value::Option(None) => Json::Null,
        Value::Option(Some(inner)) | Value::Newtype(inner) => to_json(*inner),
        Value::Seq(items) => Json::Array(items.into_iter().map(to_json).collect()),
        Value::Map(entries) => Json::Object(
            entries
                .into_iter()
                .map(|(key, item)| {
                    let name = match key {
                        Value::String(text) => text,
                        other => to_json(other).to_string(),
                    };
                    (name, to_json(item))
                })
                .collect(),
        ),
        Value::Bytes(bytes) => Json::Array(bytes.into_iter().map(Json::from).collect()),
    }
}

impl Tree {
    pub(crate) fn into_lenient(self) -> Lenient {
        Lenient(self.0)
    }
}
