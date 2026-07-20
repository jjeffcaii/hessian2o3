use super::de::Deserializer;
use super::ser::{DefaultFormatter, Serializer};
use crate::Result;
use crate::codec::Encoder;
use crate::hessian::HSerialize;
use crate::value::Value;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io;

// Rust has no stable trait specialization, so `to_writer`/`to_vec` can't
// pick a `HSerialize` impl over a `Serialize` impl purely based on
// what `T` happens to implement: a blanket `impl<T: Serialize> ToHessian`
// and a blanket `impl<T: HSerialize> ToHessian` would overlap for any
// T implementing both. Dispatching on the concrete type instead (`T` vs
// `HessianRef<T>`) sidesteps that: the two impls below target disjoint
// Self types, so there is no overlap and no need for a second trait bound
// to be "proven" inside a generic function body.
pub trait HessianWriteable {
    fn write_to<W: io::Write>(&self, writer: W) -> Result<()>;
}

impl<T: ?Sized + Serialize> HessianWriteable for T {
    fn write_to<W: io::Write>(&self, writer: W) -> Result<()> {
        let mut ser = Serializer::new(writer, DefaultFormatter);
        self.serialize(&mut ser)
    }
}

/// Wrap a value in `HessianRef` to make [`to_writer`]/[`to_vec`] encode it
/// via its [`HSerialize`] implementation instead of `serde`.
pub struct Hessian<'a, T: ?Sized>(pub &'a T);

impl<'a, T> From<&'a T> for Hessian<'a, T> {
    fn from(value: &'a T) -> Hessian<'a, T> {
        Hessian(value)
    }
}

impl<'a, T> HessianWriteable for Hessian<'a, T>
where
    T: ?Sized + HSerialize,
{
    fn write_to<W: io::Write>(&self, mut writer: W) -> Result<()> {
        let mut enc = Encoder::new(&mut writer);
        self.0.hessian_serialize(&mut enc)
    }
}

#[inline]
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + HessianWriteable,
{
    value.write_to(writer)
}

#[inline]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + HessianWriteable,
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
pub fn get_value<R>(reader: R) -> Result<Value>
where
    R: io::Read,
{
    let mut de = Deserializer::new(reader);
    de.read_value()
}

#[inline]
pub fn get_value_from_slice(v: &[u8]) -> Result<Value> {
    get_value(v)
}

#[inline]
pub fn from_slice<T>(v: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
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
    use crate::hessian::Wrapper;
    use crate::value::Value;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Serialize)]
    struct Point {
        x: i32,
        y: i32,
    }

    impl HSerialize for Point {
        fn hessian_serialize<W: io::Write>(&self, enc: &mut Encoder<W>) -> Result<()> {
            enc.begin_object("com.example.Point", &["x", "y"])?;
            Wrapper(&self.x).hessian_serialize(enc)?;
            Wrapper(&self.y).hessian_serialize(enc)?;
            Ok(())
        }
    }

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

    #[test]
    fn test_to_writer_plain_uses_serde() -> anyhow::Result<()> {
        init();

        // Point implements both `Serialize` and `HSerialize`; passed
        // unwrapped, `to_vec` must go through the `Serialize` path.
        let bytes = to_vec(&Point { x: 1, y: 2 })?;
        assert_ne!(
            "4311636f6d2e6578616d706c652e506f696e749201780179609192",
            hex::encode(&bytes)
        );

        Ok(())
    }

    #[test]
    fn test_to_writer_hessian_ref_uses_hessian_serialize() -> anyhow::Result<()> {
        init();

        let point = Point { x: 1, y: 2 };
        let bytes = to_vec(&Hessian(&point))?;
        assert_eq!(
            "4311636f6d2e6578616d706c652e506f696e749201780179609192",
            hex::encode(&bytes)
        );

        Ok(())
    }

    #[test]
    fn test_serialize_macro_prefers_hessian() -> anyhow::Result<()> {
        init();

        // `Point` implements both `Serialize` and `HSerialize`; `serialize!`
        // must dispatch to the `HSerialize` path.
        let mut buf = Vec::new();
        serialize!(&mut buf, Point { x: 1, y: 2 })?;
        assert_eq!(
            "4311636f6d2e6578616d706c652e506f696e749201780179609192",
            hex::encode(&buf)
        );

        Ok(())
    }

    #[test]
    fn test_serialize_macro_falls_back_to_serde() -> anyhow::Result<()> {
        init();

        // `OnlySerde` implements only `Serialize`, so `serialize!` must match
        // the plain `serde` output.
        #[derive(Serialize)]
        struct OnlySerde {
            n: i32,
        }

        let mut buf = Vec::new();
        serialize!(&mut buf, OnlySerde { n: 1 })?;
        assert_eq!(to_vec(&OnlySerde { n: 1 })?, buf);

        Ok(())
    }
}
