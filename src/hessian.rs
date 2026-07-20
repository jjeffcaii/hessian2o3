use crate::Result;
use crate::codec::Encoder;
use crate::de::Deserializer;
use std::io;

pub trait HSerialize {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write;
}

pub struct Wrapper<'a, T>(pub &'a T);

impl<'a> HSerialize for Wrapper<'a, bool> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_bool(*self.0)
    }
}

impl<'a> HSerialize for Wrapper<'a, i8> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i32(*self.0 as i32)
    }
}

impl<'a> HSerialize for Wrapper<'a, i16> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i32(*self.0 as i32)
    }
}

impl<'a> HSerialize for Wrapper<'a, i32> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i32(*self.0)
    }
}

impl<'a> HSerialize for Wrapper<'a, i64> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i64(*self.0)
    }
}

impl<'a> HSerialize for Wrapper<'a, u8> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i32(*self.0 as i32)
    }
}

impl<'a> HSerialize for Wrapper<'a, u16> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i32(*self.0 as i32)
    }
}

impl<'a> HSerialize for Wrapper<'a, u32> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i64(*self.0 as i64)
    }
}

impl<'a> HSerialize for Wrapper<'a, u64> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_i64(*self.0 as i64)
    }
}

impl<'a> HSerialize for Wrapper<'a, f32> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_f64(*self.0 as f64)
    }
}

impl<'a> HSerialize for Wrapper<'a, f64> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_f64(*self.0)
    }
}

impl<'a> HSerialize for Wrapper<'a, String> {
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        enc.put_str(self.0)
    }
}

impl<'a, T> HSerialize for Wrapper<'a, T>
where
    T: HSerialize,
{
    fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()>
    where
        W: io::Write,
    {
        self.0.hessian_serialize(enc)
    }
}

impl<T: HSerialize + ?Sized> HSerialize for &T {
    fn hessian_serialize<W: io::Write>(&self, enc: &mut Encoder<W>) -> Result<()> {
        (**self).hessian_serialize(enc)
    }
}

impl<T: HSerialize> HSerialize for Option<T> {
    fn hessian_serialize<W: io::Write>(&self, enc: &mut Encoder<W>) -> Result<()> {
        match self {
            None => enc.put_null(),
            Some(v) => v.hessian_serialize(enc),
        }
    }
}

macro_rules! impl_hessian_serialize_vec {
    ($($t:ty),*) => {$(
        impl<'a> HSerialize for Wrapper<'a, Vec<$t>> {
            fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()> where W: io::Write {
                enc.begin_list(None, self.0.len())?;
                for item in self.0 {
                    Wrapper(item).hessian_serialize(enc)?;
                }
                Ok(())
            }
        }
    )*};
}

macro_rules! impl_hessian_serialize_option {
    ($($t:ty),*) => {$(
        impl HSerialize for Option<$t> {
            fn hessian_serialize<W>(&self, enc: &mut Encoder<W>) -> Result<()> where W: io::Write {
                match self {
                    None => enc.put_null(),
                    Some(v) => (&Wrapper(v)).hessian_serialize(enc),
                }
            }
        }
    )*};
}

impl_hessian_serialize_vec!(bool, i8, i16, i32, i64, u8, u16, u64, f32, f64, String);
impl_hessian_serialize_option!(bool, i8, i16, i32, i64, u8, u16, u64, f32, f64, String);

impl<T: HSerialize> HSerialize for Vec<T> {
    fn hessian_serialize<W: io::Write>(&self, enc: &mut Encoder<W>) -> Result<()> {
        enc.begin_list(None, self.len())?;
        for item in self {
            item.hessian_serialize(enc)?;
        }
        Ok(())
    }
}

/// The counterpart of [`HSerialize`]: builds `Self` by reading
/// directly from the streaming [`Deserializer`] in `crate::de`, which
/// handles the byte-level wire format (class definitions, object
/// references, chunking, ...). No intermediate [`crate::value::Value`]
/// tree is built.
pub trait HDeserialize: Sized {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self>;
}

impl HDeserialize for bool {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
        de.read_bool()
    }
}

macro_rules! impl_hessian_deserialize_int {
    ($($t:ty),*) => {$(
        impl HDeserialize for $t {
            fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> crate::Result<Self> {
                let n = de.read_i64()?;
                <$t>::try_from(n).map_err(|_| {
                    crate::Error::IO(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("integer {} out of range of {}", n, stringify!($t)),
                    ))
                })
            }
        }
    )*};
}

impl_hessian_deserialize_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl HDeserialize for f32 {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
        f64::hessian_deserialize(de).map(|d| d as f32)
    }
}

impl HDeserialize for f64 {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
        de.read_f64()
    }
}

impl HDeserialize for String {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
        de.read_string()
    }
}

impl<T: HDeserialize> HDeserialize for Option<T> {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
        if de.try_read_null()? {
            Ok(None)
        } else {
            T::hessian_deserialize(de).map(Some)
        }
    }
}

impl<T: HDeserialize> HDeserialize for Vec<T> {
    fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
        match de.begin_list()? {
            Some(len) => {
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    items.push(T::hessian_deserialize(de)?);
                }
                Ok(items)
            }
            None => {
                let mut items = Vec::new();
                while !de.try_end_list()? {
                    items.push(T::hessian_deserialize(de)?);
                }
                Ok(items)
            }
        }
    }
}

#[inline]
pub fn hessian_to_writer<W, T>(w: &mut W, value: &T) -> Result<()>
where
    W: io::Write,
    T: HSerialize,
{
    let mut enc = Encoder::new(w);
    value.hessian_serialize(&mut enc)
}

#[inline]
pub fn hessian_to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: HSerialize,
{
    let mut buf = Vec::with_capacity(128);
    hessian_to_writer(&mut buf, value)?;
    Ok(buf)
}

#[inline]
pub fn hessian_from_reader<R, T>(reader: &mut R) -> Result<T>
where
    R: io::Read,
    T: HDeserialize,
{
    let mut de = Deserializer::new(reader);
    T::hessian_deserialize(&mut de)
}

#[inline]
pub fn hessian_from_slice<T>(mut b: &[u8]) -> Result<T>
where
    T: HDeserialize,
{
    hessian_from_reader(&mut b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Encoder;
    use anyhow::Result;

    fn hex<T: HSerialize>(v: &T) -> Result<String> {
        let mut buf = vec![];
        let mut enc = Encoder::new(&mut buf);
        v.hessian_serialize(&mut enc)?;
        Ok(hex::encode(buf))
    }

    #[test]
    fn test_primitives() -> Result<()> {
        // bool
        assert_eq!("54", hex(&Wrapper(&true))?);
        assert_eq!("46", hex(&Wrapper(&false))?);
        // i8 / i16 / i32 → put_i32
        assert_eq!("90", hex(&Wrapper(&0i32))?);
        assert_eq!("91", hex(&Wrapper(&1i32))?);
        assert_eq!("90", hex(&Wrapper(&0i8))?);
        assert_eq!("90", hex(&Wrapper(&0i16))?);
        // i64 → put_i64
        assert_eq!("e0", hex(&Wrapper(&0i64))?);
        assert_eq!("e1", hex(&Wrapper(&1i64))?);
        // u8 / u16 → put_i32
        assert_eq!("90", hex(&Wrapper(&0u8))?);
        assert_eq!("90", hex(&Wrapper(&0u16))?);
        // u32 / u64 → put_i64
        assert_eq!("e0", hex(&Wrapper(&0u32))?);
        assert_eq!("e0", hex(&Wrapper(&0u64))?);
        // f32 / f64
        assert_eq!("5b", hex(&Wrapper(&0.0f64))?);
        assert_eq!("5c", hex(&Wrapper(&1.0f64))?);
        assert_eq!("5b", hex(&Wrapper(&0.0f32))?);
        // String / &str
        assert_eq!("00", hex(&Wrapper(&String::from("")))?);
        assert_eq!("0568656c6c6f", hex(&Wrapper(&String::from("hello")))?);
        assert_eq!("00", hex(&Wrapper(&String::from("")))?);
        assert_eq!("0568656c6c6f", hex(&Wrapper(&"hello".to_string()))?);
        // Option
        assert_eq!("4e", hex(&None::<Wrapper<'_, i32>>)?);
        assert_eq!("91", hex(&Some(Wrapper(&1i32)))?);
        // Vec<T: HSerialize>
        assert_eq!("78", hex(&Wrapper(&Vec::<i32>::new()))?);
        assert_eq!("7b919293", hex(&Wrapper(&vec![1i32, 2, 3]))?);

        Ok(())
    }

    #[test]
    fn test_deserialize_variable_list() -> Result<()> {
        // x57 value* 'Z': untyped variable-length list
        let b = hex::decode("579192935a")?;
        let v: Vec<i32> = hessian_from_slice(&b)?;
        assert_eq!(vec![1, 2, 3], v);

        // x55 type value* 'Z': typed variable-length list ("[int")
        let b = hex::decode("55045b696e749192935a")?;
        let v: Vec<i32> = hessian_from_slice(&b)?;
        assert_eq!(vec![1, 2, 3], v);

        Ok(())
    }

    #[test]
    fn test_roundtrip_scalars() -> Result<()> {
        assert!(hessian_from_slice::<bool>(&hessian_to_vec(&Wrapper(
            &true
        ))?)?);
        assert_eq!(
            -8i8,
            hessian_from_slice::<i8>(&hessian_to_vec(&Wrapper(&-8i8))?)?
        );
        assert_eq!(
            123i32,
            hessian_from_slice::<i32>(&hessian_to_vec(&Wrapper(&123i32))?)?
        );
        assert_eq!(
            i64::MAX,
            hessian_from_slice::<i64>(&hessian_to_vec(&Wrapper(&i64::MAX))?)?
        );
        assert_eq!(
            2.5f64,
            hessian_from_slice::<f64>(&hessian_to_vec(&Wrapper(&2.5f64))?)?
        );
        assert_eq!(
            "杨幂".to_owned(),
            hessian_from_slice::<String>(&hessian_to_vec(&Wrapper(&"杨幂".to_string()))?)?
        );
        assert_eq!(
            None::<i32>,
            hessian_from_slice::<Option<i32>>(&hessian_to_vec(&None::<i32>)?)?
        );
        assert_eq!(
            Some(7i32),
            hessian_from_slice::<Option<i32>>(&hessian_to_vec(&Some(7i32))?)?
        );
        assert_eq!(
            vec![1i32, 2, 3],
            hessian_from_slice::<Vec<i32>>(&hessian_to_vec(&Wrapper(&vec![1i32, 2, 3]))?)?
        );
        // an i64-encoded value that doesn't fit the requested type
        assert!(hessian_from_slice::<i8>(&hessian_to_vec(&Wrapper(&1234i32))?).is_err());

        Ok(())
    }

    #[test]
    fn test_manual_object() {
        // Manually implement HSerialize for a Point struct to verify
        // hessian_to_vec produces the correct object encoding.
        struct Point {
            x: i32,
            y: i32,
        }

        impl HSerialize for Point {
            fn hessian_serialize<W: io::Write>(&self, enc: &mut Encoder<W>) -> crate::Result<()> {
                enc.begin_object("com.example.Point", &["x", "y"])?;
                Wrapper(&self.x).hessian_serialize(enc)?;
                Wrapper(&self.y).hessian_serialize(enc)?;
                Ok(())
            }
        }

        // Expected byte-by-byte:
        //  43               C (class definition)
        //  11               17 chars (direct string)
        //  636f6d2e6578616d706c652e506f696e74  "com.example.Point"
        //  92               put_i32(2) = 0x90+2 (field count)
        //  01 78            "x" (1 char)
        //  01 79            "y" (1 char)
        //  60               BC_OBJECT_DIRECT + 0 (ref 0)
        //  91               put_i32(1)
        //  92               put_i32(2)
        let bytes = hessian_to_vec(&Point { x: 1, y: 2 }).unwrap();
        assert_eq!(
            "4311636f6d2e6578616d706c652e506f696e749201780179609192",
            hex::encode(&bytes)
        );
    }
}
