use super::ser::{DefaultFormatter, Serializer};
use crate::Result;
use serde::Serialize;
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
