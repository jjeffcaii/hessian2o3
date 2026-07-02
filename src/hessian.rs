use crate::codec::{self, Context};
use crate::value::{PrimitiveValue, Value};
use std::io;

pub trait HessianSerialize {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()>;
}

impl HessianSerialize for bool {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_bool(w, *self)
    }
}

impl HessianSerialize for i8 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for i16 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for i32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i32(w, *self)
    }
}

impl HessianSerialize for i64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i64(w, *self)
    }
}

impl HessianSerialize for u8 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for u16 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for u32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i64(w, *self as i64)
    }
}

impl HessianSerialize for u64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_i64(w, *self as i64)
    }
}

impl HessianSerialize for f32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_f64(w, *self as f64)
    }
}

impl HessianSerialize for f64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_f64(w, *self)
    }
}

impl HessianSerialize for str {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_str(w, self)
    }
}

impl<T: HessianSerialize + ?Sized> HessianSerialize for &T {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        (**self).hessian_serialize(w, ctx)
    }
}

impl HessianSerialize for String {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        codec::put_str(w, self.as_str())
    }
}

impl<T: HessianSerialize> HessianSerialize for Option<T> {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        match self {
            None => codec::put_null(w),
            Some(v) => v.hessian_serialize(w, ctx),
        }
    }
}

impl<T: HessianSerialize> HessianSerialize for Vec<T> {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        codec::begin_list(w, None, self.len())?;
        for item in self {
            item.hessian_serialize(w, ctx)?;
        }
        Ok(())
    }
}

/// The counterpart of [`HessianSerialize`]: builds `Self` from a decoded
/// [`Value`] tree. The byte-level wire format (class definitions, object
/// references, chunking, ...) is handled by [`codec::get_value`]; this trait
/// only maps the resulting `Value` onto a Rust type.
pub trait HessianDeserialize: Sized {
    fn hessian_deserialize(value: Value) -> io::Result<Self>;
}

/// Builds an error for a `Value` whose shape doesn't match the target type.
/// Exposed (hidden) for use by the `#[derive(Hessian)]` expansion.
#[doc(hidden)]
pub fn unexpected_value(expect: &str, actual: &Value) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("expect hessian {}, but got: {:?}", expect, actual),
    )
}

impl HessianDeserialize for bool {
    fn hessian_deserialize(value: Value) -> io::Result<Self> {
        match value {
            Value::Primitive(PrimitiveValue::Bool(b)) => Ok(b),
            other => Err(unexpected_value("bool", &other)),
        }
    }
}

macro_rules! impl_hessian_deserialize_int {
    ($($t:ty),*) => {$(
        impl HessianDeserialize for $t {
            fn hessian_deserialize(value: Value) -> io::Result<Self> {
                let n = match value {
                    Value::Primitive(PrimitiveValue::Int(i)) => i as i64,
                    Value::Primitive(PrimitiveValue::Long(l)) => l,
                    other => return Err(unexpected_value("integer", &other)),
                };
                <$t>::try_from(n).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("integer {} out of range of {}", n, stringify!($t)),
                    )
                })
            }
        }
    )*};
}

impl_hessian_deserialize_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl HessianDeserialize for f32 {
    fn hessian_deserialize(value: Value) -> io::Result<Self> {
        f64::hessian_deserialize(value).map(|d| d as f32)
    }
}

impl HessianDeserialize for f64 {
    fn hessian_deserialize(value: Value) -> io::Result<Self> {
        match value {
            Value::Primitive(PrimitiveValue::Double(d)) => Ok(d),
            other => Err(unexpected_value("double", &other)),
        }
    }
}

impl HessianDeserialize for String {
    fn hessian_deserialize(value: Value) -> io::Result<Self> {
        match value {
            Value::Primitive(PrimitiveValue::String(s)) => Ok(s),
            other => Err(unexpected_value("string", &other)),
        }
    }
}

impl<T: HessianDeserialize> HessianDeserialize for Option<T> {
    fn hessian_deserialize(value: Value) -> io::Result<Self> {
        match value {
            Value::Null => Ok(None),
            other => T::hessian_deserialize(other).map(Some),
        }
    }
}

impl<T: HessianDeserialize> HessianDeserialize for Vec<T> {
    fn hessian_deserialize(value: Value) -> io::Result<Self> {
        match value {
            Value::List(l) => l.into_iter().map(T::hessian_deserialize).collect(),
            other => Err(unexpected_value("list", &other)),
        }
    }
}

pub fn hessian_to_writer<W: io::Write, T: HessianSerialize>(
    writer: &mut W,
    value: &T,
) -> crate::Result<()> {
    let mut ctx = Context::default();
    value
        .hessian_serialize(writer, &mut ctx)
        .map_err(crate::Error::IO)
}

pub fn hessian_to_vec<T: HessianSerialize>(value: &T) -> crate::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(128);
    hessian_to_writer(&mut buf, value)?;
    Ok(buf)
}

pub fn hessian_from_reader<R: io::Read, T: HessianDeserialize>(reader: &mut R) -> crate::Result<T> {
    let mut ctx = Context::default();
    let value = codec::get_value(&mut ctx, reader).map_err(crate::Error::IO)?;
    T::hessian_deserialize(value).map_err(crate::Error::IO)
}

pub fn hessian_from_slice<T: HessianDeserialize>(mut b: &[u8]) -> crate::Result<T> {
    hessian_from_reader(&mut b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Context;

    fn hex<T: HessianSerialize>(v: &T) -> String {
        let mut buf = vec![];
        let mut ctx = Context::default();
        v.hessian_serialize(&mut buf, &mut ctx).unwrap();
        hex::encode(buf)
    }

    #[test]
    fn test_primitives() {
        // bool
        assert_eq!("54", hex(&true));
        assert_eq!("46", hex(&false));
        // i8 / i16 / i32 → put_i32
        assert_eq!("90", hex(&0i32));
        assert_eq!("91", hex(&1i32));
        assert_eq!("90", hex(&0i8));
        assert_eq!("90", hex(&0i16));
        // i64 → put_i64
        assert_eq!("e0", hex(&0i64));
        assert_eq!("e1", hex(&1i64));
        // u8 / u16 → put_i32
        assert_eq!("90", hex(&0u8));
        assert_eq!("90", hex(&0u16));
        // u32 / u64 → put_i64
        assert_eq!("e0", hex(&0u32));
        assert_eq!("e0", hex(&0u64));
        // f32 / f64
        assert_eq!("5b", hex(&0.0f64));
        assert_eq!("5c", hex(&1.0f64));
        assert_eq!("5b", hex(&0.0f32));
        // String / &str
        assert_eq!("00", hex(&String::from("")));
        assert_eq!("0568656c6c6f", hex(&String::from("hello")));
        assert_eq!("00", hex(&""));
        assert_eq!("0568656c6c6f", hex(&"hello"));
        // Option
        assert_eq!("4e", hex(&None::<i32>));
        assert_eq!("91", hex(&Some(1i32)));
        // Vec<T: HessianSerialize>
        assert_eq!("78", hex(&Vec::<i32>::new()));
        assert_eq!("7b919293", hex(&vec![1i32, 2, 3]));
    }

    #[test]
    fn test_manual_object() {
        // Manually implement HessianSerialize for a Point struct to verify
        // hessian_to_vec produces the correct object encoding.
        struct Point {
            x: i32,
            y: i32,
        }

        impl HessianSerialize for Point {
            fn hessian_serialize<W: io::Write>(
                &self,
                w: &mut W,
                ctx: &mut Context,
            ) -> io::Result<()> {
                codec::begin_object(w, ctx, "com.example.Point", &["x", "y"])?;
                self.x.hessian_serialize(w, ctx)?;
                self.y.hessian_serialize(w, ctx)?;
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
