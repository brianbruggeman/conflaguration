//! `deserialize_with` helpers for fields serde reads from its own buffer.

use std::fmt::{self, Display};
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::value::{BoolDeserializer, F64Deserializer, I64Deserializer, U64Deserializer};
use serde::de::{Deserialize, Deserializer, Error, Visitor};

struct Text<T>(PhantomData<T>);

fn parse<T: FromStr, E: Error>(text: &str) -> Result<T, E>
where
    T::Err: Display,
{
    text.parse::<T>()
        .map_err(|cause| E::custom(format_args!("invalid value {text:?}: {cause}")))
}

impl<'de, T: Deserialize<'de> + FromStr> Visitor<'de> for Text<T>
where
    T::Err: Display,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a value or its text")
    }

    fn visit_bool<E: Error>(self, value: bool) -> Result<T, E> {
        T::deserialize(BoolDeserializer::<E>::new(value))
    }

    fn visit_i64<E: Error>(self, value: i64) -> Result<T, E> {
        T::deserialize(I64Deserializer::<E>::new(value))
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<T, E> {
        T::deserialize(U64Deserializer::<E>::new(value))
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<T, E> {
        T::deserialize(F64Deserializer::<E>::new(value))
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<T, E> {
        parse(value)
    }
}

struct MaybeText<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de> + FromStr> Visitor<'de> for MaybeText<T>
where
    T::Err: Display,
{
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a value, its text, or nothing")
    }

    fn visit_unit<E: Error>(self) -> Result<Option<T>, E> {
        Ok(None)
    }

    fn visit_none<E: Error>(self) -> Result<Option<T>, E> {
        Ok(None)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<T>, D::Error> {
        deserializer.deserialize_any(Text::<T>(PhantomData)).map(Some)
    }

    fn visit_bool<E: Error>(self, value: bool) -> Result<Option<T>, E> {
        Text::<T>(PhantomData).visit_bool(value).map(Some)
    }

    fn visit_i64<E: Error>(self, value: i64) -> Result<Option<T>, E> {
        Text::<T>(PhantomData).visit_i64(value).map(Some)
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<Option<T>, E> {
        Text::<T>(PhantomData).visit_u64(value).map(Some)
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<Option<T>, E> {
        Text::<T>(PhantomData).visit_f64(value).map(Some)
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Option<T>, E> {
        parse(value).map(Some)
    }
}

/// Read a field from its typed value or from its text.
pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de> + FromStr>(deserializer: D) -> Result<T, D::Error>
where
    T::Err: Display,
{
    deserializer.deserialize_any(Text::<T>(PhantomData))
}

/// Read an optional field from its typed value, its text, or nothing.
pub fn option<'de, D: Deserializer<'de>, T: Deserialize<'de> + FromStr>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T::Err: Display,
{
    deserializer.deserialize_any(MaybeText::<T>(PhantomData))
}
