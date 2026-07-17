use super::de::Deserializer;
use super::ser::{DefaultFormatter, Serializer};
use crate::Result;
use crate::value::Value;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::{Any, TypeId};
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
    T: DeserializeOwned + 'static,
{
    let mut de = Deserializer::new(reader);
    if TypeId::of::<T>() == TypeId::of::<Value>() {
        let value = de.read_value()?;
        let any: Box<dyn Any> = Box::new(value);
        return Ok(*any.downcast::<T>().unwrap());
    }
    T::deserialize(&mut de)
}

#[inline]
pub fn from_slice<T>(v: &[u8]) -> Result<T>
where
    T: DeserializeOwned + 'static,
{
    from_reader(v)
}

#[inline]
pub fn from_value<T>(v: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    T::deserialize(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use serde::Deserialize;
    use serde_json::json;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test() -> anyhow::Result<()> {
        init();

        let b = hex::decode(
            "431d6a6176612e6c616e672e41726974686d65746963457863657074696f6e940d64657461696c4d6573736167650563617573650a737461636b54726163651473757070726573736564457863657074696f6e7360092f206279207a65726f5190561c5b6a6176612e6c616e672e537461636b5472616365456c656d656e749c431b6a6176612e6c616e672e537461636b5472616365456c656d656e74940e6465636c6172696e67436c6173730a6d6574686f644e616d650866696c654e616d650a6c696e654e756d626572613038636f6d2e616c69626162612e6873662e6c696768742e70726f746f636f6c2e6873662e48656c6c6f576f726c6453657276696365496d706c036164641a48656c6c6f576f726c6453657276696365496d706c2e6a617661a161302473756e2e7265666c6563742e4e61746976654d6574686f644163636573736f72496d706c07696e766f6b65301d4e61746976654d6574686f644163636573736f72496d706c2e6a6176618e61302473756e2e7265666c6563742e4e61746976654d6574686f644163636573736f72496d706c06696e766f6b651d4e61746976654d6574686f644163636573736f72496d706c2e6a617661c83e61302873756e2e7265666c6563742e44656c65676174696e674d6574686f644163636573736f72496d706c06696e766f6b65302144656c65676174696e674d6574686f644163636573736f72496d706c2e6a617661bb61186a6176612e6c616e672e7265666c6563742e4d6574686f6406696e766f6b650b4d6574686f642e6a617661c9f261303f636f6d2e616c69626162612e6873662e6c696768742e6170692e50726f78794d6574686f6448616e646c65727324556e61727950726f7879466163746f72790f6c616d6264612463726561746524301850726f78794d6574686f6448616e646c6572732e6a617661c830613053636f6d2e616c69626162612e6873662e6c696768742e6170692e4d6574686f6443616c6c48616e646c65727324556e6172794d6574686f6424556e617279536572766572496e626f756e644c697374656e65720a6f6e436f6d706c657465174d6574686f6443616c6c48616e646c6572732e6a617661c85d613030636f6d2e616c69626162612e6873662e6c696768742e70726f746f636f6c2e6873662e4853464d6574686f6443616c6c06616363657074124853464d6574686f6443616c6c2e6a617661c846613033636f6d2e616c69626162612e6873662e6c696768742e70726f746f636f6c2e6873662e48534653657276657248616e646c6572176c616d6264612470726f636573735265717565737424321548534653657276657248616e646c65722e6a617661c85f6130276a6176612e7574696c2e636f6e63757272656e742e546872656164506f6f6c4578656375746f720972756e576f726b657217546872656164506f6f6c4578656375746f722e6a617661cc7d61302e6a6176612e7574696c2e636f6e63757272656e742e546872656164506f6f6c4578656375746f7224576f726b65720372756e17546872656164506f6f6c4578656375746f722e6a617661ca7061106a6176612e6c616e672e5468726561640372756e0b5468726561642e6a617661caee7030266a6176612e7574696c2e436f6c6c656374696f6e7324556e6d6f6469666961626c654c697374",
        )?;

        let v: Value = from_slice(&b)?;

        info!("value: {}", v);

        Ok(())
    }

    #[test]
    fn test_from_value() -> anyhow::Result<()> {
        init();

        let j = json!({
            "foo": "bar",
            "hello": "world",
            "levels": [1,2,3],
        });

        let b = to_vec(&j)?;

        let v = from_slice::<Value>(&b)?;

        #[derive(Debug, Serialize, Deserialize)]
        struct Pojo {
            foo: String,
            hello: String,
            levels: Vec<i32>,
        }

        let pojo = from_value::<Pojo>(v)?;

        info!("from_value: {:?}", pojo);

        Ok(())
    }
}
