use crate::encode::{self, Context};
use std::io;

pub trait HessianSerialize {
    fn hessian_serialize<W: io::Write>(
        &self,
        w: &mut W,
        ctx: &mut Context,
    ) -> io::Result<()>;
}

impl HessianSerialize for bool {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_bool(w, *self)
    }
}

impl HessianSerialize for i8 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for i16 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for i32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self)
    }
}

impl HessianSerialize for i64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i64(w, *self)
    }
}

impl HessianSerialize for u8 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for u16 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for u32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i64(w, *self as i64)
    }
}

impl HessianSerialize for u64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i64(w, *self as i64)
    }
}

impl HessianSerialize for f32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_f64(w, *self as f64)
    }
}

impl HessianSerialize for f64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_f64(w, *self)
    }
}

impl HessianSerialize for str {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_str(w, self)
    }
}

impl<T: HessianSerialize + ?Sized> HessianSerialize for &T {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        (**self).hessian_serialize(w, ctx)
    }
}

impl HessianSerialize for String {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_str(w, self.as_str())
    }
}

impl<T: HessianSerialize> HessianSerialize for Option<T> {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        match self {
            None => encode::put_null(w),
            Some(v) => v.hessian_serialize(w, ctx),
        }
    }
}

impl<T: HessianSerialize> HessianSerialize for Vec<T> {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        encode::begin_list(w, None, self.len())?;
        for item in self {
            item.hessian_serialize(w, ctx)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Context;

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
}
