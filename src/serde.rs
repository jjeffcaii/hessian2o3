use super::de::Deserializer;
use super::ser::{DefaultFormatter, Serializer};
use crate::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io;

#[inline]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + Serialize,
{
    let mut ser = Serializer::new(writer, DefaultFormatter);
    value.serialize(&mut ser)
}

#[inline]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut buf = Vec::with_capacity(128);
    to_writer(&mut buf, value)?;
    Ok(buf)
}

#[inline]
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: io::Read,
    T: DeserializeOwned,
{
    let mut de = Deserializer::new(reader);
    T::deserialize(&mut de)
}

#[inline]
pub fn from_slice<T>(v: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    from_reader(v)
}
