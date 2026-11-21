use std::fmt::Display;
use std::iter;
use std::str::FromStr;

use serde::de::value::{MapDeserializer, SeqDeserializer};
use serde::de::{self, DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, VariantAccess, Visitor};
use serde_value::{DeserializerError, Value};

/// A deserializer over a merged value tree that reads text where a bool, a
/// number, a list or an option is wanted, and hands every child back to itself.
pub(crate) struct Lenient(pub(crate) Value);

fn parse<T: FromStr>(text: &str) -> Result<T, DeserializerError>
where
    T::Err: Display,
{
    text.parse::<T>()
        .map_err(|error| <DeserializerError as de::Error>::custom(format_args!("invalid value {text:?}: {error}")))
}

fn kind_of(value: &Value) -> de::Unexpected<'static> {
    match value {
        Value::Bool(flag) => de::Unexpected::Bool(*flag),
        Value::String(_) => de::Unexpected::Other("string"),
        Value::Seq(_) => de::Unexpected::Seq,
        Value::Map(_) => de::Unexpected::Map,
        Value::Unit => de::Unexpected::Unit,
        Value::Option(_) => de::Unexpected::Option,
        Value::Newtype(_) => de::Unexpected::NewtypeStruct,
        _ => de::Unexpected::Other("scalar"),
    }
}

fn is_empty_map(value: &Value) -> bool {
    matches!(value, Value::Map(entries) if entries.is_empty())
}

macro_rules! parse_scalar {
    ($method:ident, $visit:ident, $target:ty) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            match self.0 {
                Value::String(text) => visitor.$visit(parse::<$target>(&text)?),
                other => Lenient(other).deserialize_any(visitor),
            }
        }
    };
}

impl<'de> IntoDeserializer<'de, DeserializerError> for Lenient {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

impl Lenient {
    fn deserialize_keyed<'de, V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializerError> {
        match self.0 {
            Value::Unit => visitor.visit_map(MapDeserializer::<_, DeserializerError>::new(iter::empty::<(Lenient, Lenient)>())),
            _ => self.deserialize_any(visitor),
        }
    }
}

impl<'de> Deserializer<'de> for Lenient {
    type Error = DeserializerError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Bool(flag) => visitor.visit_bool(flag),
            Value::U8(number) => visitor.visit_u8(number),
            Value::U16(number) => visitor.visit_u16(number),
            Value::U32(number) => visitor.visit_u32(number),
            Value::U64(number) => visitor.visit_u64(number),
            Value::I8(number) => visitor.visit_i8(number),
            Value::I16(number) => visitor.visit_i16(number),
            Value::I32(number) => visitor.visit_i32(number),
            Value::I64(number) => visitor.visit_i64(number),
            Value::F32(number) => visitor.visit_f32(number),
            Value::F64(number) => visitor.visit_f64(number),
            Value::Char(letter) => visitor.visit_char(letter),
            Value::String(text) => visitor.visit_string(text),
            Value::Unit => visitor.visit_unit(),
            Value::Option(None) => visitor.visit_none(),
            Value::Option(Some(inner)) => visitor.visit_some(Lenient(*inner)),
            Value::Newtype(inner) => visitor.visit_newtype_struct(Lenient(*inner)),
            Value::Seq(items) => visitor.visit_seq(SeqDeserializer::<_, DeserializerError>::new(items.into_iter().map(Lenient))),
            Value::Map(entries) => visitor.visit_map(MapDeserializer::<_, DeserializerError>::new(
                entries.into_iter().map(|(key, item)| (Lenient(key), Lenient(item))),
            )),
            Value::Bytes(bytes) => visitor.visit_byte_buf(bytes),
        }
    }

    parse_scalar!(deserialize_i8, visit_i8, i8);
    parse_scalar!(deserialize_i16, visit_i16, i16);
    parse_scalar!(deserialize_i32, visit_i32, i32);
    parse_scalar!(deserialize_i64, visit_i64, i64);
    parse_scalar!(deserialize_i128, visit_i128, i128);
    parse_scalar!(deserialize_u8, visit_u8, u8);
    parse_scalar!(deserialize_u16, visit_u16, u16);
    parse_scalar!(deserialize_u32, visit_u32, u32);
    parse_scalar!(deserialize_u64, visit_u64, u64);
    parse_scalar!(deserialize_u128, visit_u128, u128);
    parse_scalar!(deserialize_f32, visit_f32, f32);
    parse_scalar!(deserialize_f64, visit_f64, f64);
    parse_scalar!(deserialize_bool, visit_bool, bool);

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if is_empty_map(&self.0) || matches!(self.0, Value::Unit) {
            return visitor.visit_string(String::new());
        }
        self.deserialize_any(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if is_empty_map(&self.0) {
            return visitor.visit_none();
        }
        match self.0 {
            Value::Option(_) => self.deserialize_any(visitor),
            Value::Unit => visitor.visit_unit(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Newtype(inner) => visitor.visit_newtype_struct(Lenient(*inner)),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if is_empty_map(&self.0) || matches!(self.0, Value::Unit) {
            return visitor.visit_seq(SeqDeserializer::<_, DeserializerError>::new(iter::empty::<Lenient>()));
        }
        match self.0 {
            Value::Seq(_) | Value::Bytes(_) => self.deserialize_any(visitor),
            single => visitor.visit_seq(SeqDeserializer::<_, DeserializerError>::new(iter::once(Lenient(single)))),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_keyed(visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_keyed(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
        let (variant, payload) = match self.0 {
            Value::Map(entries) => {
                let mut iter = entries.into_iter();
                let Some((variant, payload)) = iter.next() else {
                    return Err(de::Error::invalid_value(de::Unexpected::Map, &"map with a single key"));
                };
                if iter.next().is_some() {
                    return Err(de::Error::invalid_value(de::Unexpected::Map, &"map with a single key"));
                }
                (variant, Some(payload))
            }
            Value::String(variant) => (Value::String(variant), None),
            other => return Err(de::Error::invalid_type(kind_of(&other), &"string or map")),
        };
        visitor.visit_enum(LenientEnum { variant, payload })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }
}

struct LenientEnum {
    variant: Value,
    payload: Option<Value>,
}

impl<'de> EnumAccess<'de> for LenientEnum {
    type Error = DeserializerError;
    type Variant = LenientVariant;

    fn variant_seed<S: DeserializeSeed<'de>>(self, seed: S) -> Result<(S::Value, LenientVariant), Self::Error> {
        let name = seed.deserialize(Lenient(self.variant))?;
        Ok((name, LenientVariant { payload: self.payload }))
    }
}

struct LenientVariant {
    payload: Option<Value>,
}

impl<'de> VariantAccess<'de> for LenientVariant {
    type Error = DeserializerError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.payload {
            Some(value) => de::Deserialize::deserialize(Lenient(value)),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<S: DeserializeSeed<'de>>(self, seed: S) -> Result<S::Value, Self::Error> {
        match self.payload {
            Some(value) => seed.deserialize(Lenient(value)),
            None => Err(de::Error::invalid_type(de::Unexpected::UnitVariant, &"newtype variant")),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        match self.payload {
            Some(Value::Seq(items)) => visitor.visit_seq(SeqDeserializer::<_, DeserializerError>::new(items.into_iter().map(Lenient))),
            Some(other) => Err(de::Error::invalid_type(kind_of(&other), &"tuple variant")),
            None => Err(de::Error::invalid_type(de::Unexpected::UnitVariant, &"tuple variant")),
        }
    }

    fn struct_variant<V: Visitor<'de>>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
        match self.payload {
            Some(Value::Map(entries)) => visitor.visit_map(MapDeserializer::<_, DeserializerError>::new(
                entries.into_iter().map(|(key, item)| (Lenient(key), Lenient(item))),
            )),
            Some(other) => Err(de::Error::invalid_type(kind_of(&other), &"struct variant")),
            None => Err(de::Error::invalid_type(de::Unexpected::UnitVariant, &"struct variant")),
        }
    }
}
