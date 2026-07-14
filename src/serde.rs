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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test() -> anyhow::Result<()> {
        init();

        let b = hex::decode(
            "48086561676c656579654807747261636549640005727063496400087573657244617461005a0768656164657273480c436f6e74656e742d547970655891106170706c69636174696f6e2f6a736f6e5a06706172616d73480568656c6c6f589303666f6f03626172037175780174589101315a5a",
        )?;

        let v: Value = from_slice(&b)?;

        info!("value: {}", v);

        Ok(())
    }
}
