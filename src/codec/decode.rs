use super::tags::*;
use crate::cachestr::Cachestr;
use crate::codec::{Context, Fields};
use crate::misc;
use crate::value::{Map, Object, PrimitiveValue, Value};
use std::arch::aarch64::vget_high_u16;
use std::collections::HashMap;
use std::{io, time};

#[inline]
fn read_binary<R>(_ctx: &mut Context, r: &mut R, dst: &mut Vec<u8>, n: usize) -> io::Result<()>
where
    R: io::Read,
{
    let start = dst.len();
    dst.resize(start + n, 0);
    r.read_exact(&mut dst[start..])
}

#[inline]
fn read_binary_chunked<R>(
    ctx: &mut Context,
    r: &mut R,
    dst: &mut Vec<u8>,
    n: usize,
    is_final: bool,
) -> io::Result<()>
where
    R: io::Read,
{
    read_binary(ctx, r, dst, n)?;
    if is_final {
        return Ok(());
    }

    let code = read_u8(r)?;

    match code {
        BC_BINARY_CHUNK => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            read_binary_chunked(ctx, r, dst, length, false)
        }
        BC_BINARY => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            read_binary_chunked(ctx, r, dst, length, true)
        }
        0x20..=0x2f => {
            let length = (code - 0x20) as usize;
            read_binary_chunked(ctx, r, dst, length, true)
        }
        0x34..=0x37 => {
            let length = {
                let high = (code - 0x34) as usize;
                let low = read_u8(r)? as usize;

                (high << 8) + low
            };
            read_binary_chunked(ctx, r, dst, length, true)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid binary tag code",
        )),
    }
}

#[inline]
fn read_utf8<R>(r: &mut R, dst: &mut String, n: usize) -> io::Result<()>
where
    R: io::Read,
{
    let mut buf = [0u8; 4]; // 单个 UTF-8 字符最多 4 字节

    for _ in 0..n {
        // 先读首字节，判断该字符总长度
        r.read_exact(&mut buf[..1])?;
        let first = buf[0];

        let char_len = match first {
            0x00..=0x7F => 1, // 0xxxxxxx
            0xC0..=0xDF => 2, // 110xxxxx
            0xE0..=0xEF => 3, // 1110xxxx
            0xF0..=0xF7 => 4, // 11110xxx
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid utf-8")),
        };

        // 读取剩余的续字节
        if char_len > 1 {
            r.read_exact(&mut buf[1..char_len])?;
        }

        let s = std::str::from_utf8(&buf[..char_len])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        dst.push_str(s);
    }

    Ok(())
}

#[inline]
fn read_utf8_chunked<R>(r: &mut R, dst: &mut String, n: usize, is_final: bool) -> io::Result<()>
where
    R: io::Read,
{
    read_utf8(r, dst, n)?;

    if is_final {
        return Ok(());
    }

    let tag = read_u8(r)?;

    match tag {
        BC_STRING_CHUNK => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            read_utf8_chunked(r, dst, length, false)
        }
        BC_STRING => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            read_utf8_chunked(r, dst, length, true)
        }
        0x00..=0x1f => {
            let length = tag as usize - 0x00;
            read_utf8_chunked(r, dst, length, true)
        }
        0x30..=0x33 => {
            let length = {
                let high = (tag - 0x30) as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            read_utf8_chunked(r, dst, length, true)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid tag code",
        )),
    }
}

#[inline]
fn read_f64<R>(r: &mut R) -> io::Result<f64>
where
    R: io::Read,
{
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

#[inline]
fn read_i16<R>(r: &mut R) -> io::Result<i16>
where
    R: io::Read,
{
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

#[inline]
fn read_i32<R>(r: &mut R) -> io::Result<i32>
where
    R: io::Read,
{
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

#[inline]
fn read_i64<R>(r: &mut R) -> io::Result<i64>
where
    R: io::Read,
{
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

#[inline]
fn read_i8<R>(r: &mut R) -> io::Result<i8>
where
    R: io::Read + Sized,
{
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0] as i8)
}

#[inline]
fn read_u8<R>(r: &mut R) -> io::Result<u8>
where
    R: io::Read + Sized,
{
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

#[inline]
fn read_string<R>(r: &mut R) -> io::Result<Option<Cachestr>>
where
    R: io::Read,
{
    let tag = read_u8(r)?;

    let class = match tag {
        0x00..=0x1f => {
            let length = tag as usize - 0x00;
            let mut s = String::with_capacity(length);
            read_utf8(r, &mut s, length)?;
            Some(Cachestr::from(s))
        }
        0x30..=0x33 => {
            let length = {
                let high = (tag - 0x30) as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            let mut s = String::with_capacity(length);
            read_utf8(r, &mut s, length)?;
            Some(Cachestr::from(s))
        }
        BC_STRING_CHUNK => {
            let mut s = String::with_capacity(0x8000 + 0x8000 / 2);

            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            read_utf8_chunked(r, &mut s, length, false)?;

            Some(Cachestr::from(s))
        }
        BC_STRING => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            let mut s = String::with_capacity(length);

            read_utf8_chunked(r, &mut s, length, true)?;

            Some(Cachestr::from(s))
        }
        _ => None,
    };

    Ok(class)
}

#[inline]
fn read_list<R>(ctx: &mut Context, r: &mut R, dst: &mut Vec<Value>, n: usize) -> io::Result<()>
where
    R: io::Read,
{
    for _ in 0..n {
        let next = get_value(ctx, r)?;
        dst.push(next);
    }
    Ok(())
}

fn read_value<R>(ctx: &mut Context, r: &mut R, tag: u8) -> io::Result<Option<Value>>
where
    R: io::Read,
{
    match tag {
        0x00..=0x1f => {
            let length = tag as usize - 0x00;
            let mut s = String::with_capacity(length);
            read_utf8(r, &mut s, length)?;
            Ok(Some(Value::from(s)))
        }
        0x20..=0x2f => {
            let length = tag as usize - 0x20;
            let mut b = vec![0; length];
            r.read_exact(&mut b[..])?;

            Ok(Some(Value::from(b)))
        }
        0x30..=0x33 => {
            let length = {
                let high = (tag - 0x30) as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            let mut s = String::with_capacity(length);
            read_utf8(r, &mut s, length)?;
            Ok(Some(Value::from(s)))
        }
        0x34..=0x37 => {
            let length = {
                let high = tag as usize - 0x34;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            let mut b = vec![0; length];
            r.read_exact(&mut b[..])?;

            Ok(Some(Value::from(b)))
        }

        BC_BINARY_CHUNK => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            let mut b = Vec::<u8>::with_capacity(length + length / 2);

            read_binary_chunked(ctx, r, &mut b, length, false)?;

            Ok(Some(Value::from(b)))
        }
        BC_BINARY => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            let mut b = vec![0; length];
            r.read_exact(&mut b[..])?;
            Ok(Some(Value::from(b)))
        }
        BC_STRING_CHUNK => {
            let mut s = String::with_capacity(0x8000 + 0x8000 / 2);

            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };

            read_utf8_chunked(r, &mut s, length, false)?;

            Ok(Some(Value::from(s)))
        }
        BC_STRING => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            let mut s = String::with_capacity(length);

            read_utf8_chunked(r, &mut s, length, true)?;

            Ok(Some(Value::from(s)))
        }
        BC_NULL => Ok(Some(Value::Null)),
        BC_BOOL_TRUE => Ok(Some(Value::from(true))),
        BC_BOOL_FALSE => Ok(Some(Value::from(false))),
        // direct integer
        0x80..=0xbf => {
            let direct = (tag as i8) - (BC_INT_ZERO as i8);
            Ok(Some(Value::from(direct as i32)))
        }
        // byte integer
        0xc0..=0xcf => {
            let low = read_u8(r)? as i32;
            let high = (((tag as i8) - (BC_INT_BYTE_ZERO as i8)) as i32) << 8;
            Ok(Some(Value::from(high + low)))
        }
        // short integer
        0xd0..=0xd7 => {
            let num = {
                let high = ((tag as i8) - (BC_INT_SHORT_ZERO as i8)) as i32;
                let middle = read_u8(r)? as i32;
                let low = read_u8(r)? as i32;
                (high << 16) + (middle << 8) + low
            };
            Ok(Some(Value::from(num)))
        }
        // integer
        BC_INT => {
            let v = read_i32(r)?;
            Ok(Some(Value::from(v)))
        }
        // direct long
        0xd8..=0xef => {
            let num = {
                let direct = (tag as i8) - (BC_LONG_ZERO as i8);
                direct as i64
            };
            Ok(Some(Value::from(num)))
        }
        // byte long
        0xf0..=0xff => {
            let num = {
                let low = read_u8(r)? as i64;
                let high = (((tag as i8) - (BC_LONG_BYTE_ZERO as i8)) as i64) << 8;
                high + low
            };

            Ok(Some(Value::from(num)))
        }
        // short long
        0x38..=0x3f => {
            let num = {
                let high = ((tag as i8) - (BC_LONG_SHORT_ZERO as i8)) as i64;
                let middle = read_u8(r)? as i64;
                let low = read_u8(r)? as i64;
                (high << 16) + (middle << 8) + low
            };

            Ok(Some(Value::from(num)))
        }
        // integer long
        BC_LONG_INT => {
            let num = {
                let v = read_i32(r)?;
                v as i64
            };

            Ok(Some(Value::from(num)))
        }
        // long
        BC_LONG => {
            let v = read_i64(r)?;
            Ok(Some(Value::from(v)))
        }
        BC_DOUBLE_ZERO => Ok(Some(Value::from(0f64))),
        BC_DOUBLE_ONE => Ok(Some(Value::from(1f64))),
        BC_DOUBLE_BYTE => {
            let v = read_i8(r)?;
            Ok(Some(Value::from(v as f64)))
        }
        BC_DOUBLE_SHORT => {
            let v = read_i16(r)?;
            Ok(Some(Value::from(v as f64)))
        }
        BC_DOUBLE_MILL => {
            let v = read_i32(r)? as f64;
            Ok(Some(Value::from(0.001f64 * v)))
        }
        BC_DOUBLE => {
            let v = read_f64(r)?;
            Ok(Some(Value::from(v)))
        }
        BC_DATE => {
            let v = read_i64(r)?;
            Ok(Some(Value::from(misc::millis_to_system_time(v))))
        }
        BC_DATE_MINUTE => {
            let unix_mills = (read_i32(r)? as i64) * 60000i64;
            Ok(Some(Value::from(misc::millis_to_system_time(unix_mills))))
        }
        0x70..=0x77 => {
            let length = tag as usize - 0x70;
            let class = read_string(r)?;

            info!("list class {:?}", class);
            let mut v = Vec::<Value>::with_capacity(length);
            read_list(ctx, r, &mut v, length)?;

            Ok(Some(Value::from(v)))
        }
        0x78..=0x7f => {
            let length = tag as usize - 0x78;

            let mut v = Vec::<Value>::with_capacity(length);
            read_list(ctx, r, &mut v, length)?;

            Ok(Some(Value::from(v)))
        }
        BC_MAP_UNTYPED => {
            let mut m = Map::new();
            read_map(ctx, r, &mut m)?;
            Ok(Some(Value::from(m)))
        }
        BC_MAP => {
            let mut m = Map::new();

            if let Some(class) = read_string(r)? {
                m.set_class(class);
            }

            read_map(ctx, r, &mut m)?;

            Ok(Some(Value::from(m)))
        }
        BC_END => Ok(None),
        BC_CLASS => {
            let class = read_string(r)?.expect("class should exist");
            let n = {
                let mut n = -1;
                if let Value::Primitive(pv) = get_value(ctx, r)? {
                    if let PrimitiveValue::Int(i) = pv {
                        n = i
                    }
                }
                n
            };

            let mut fields = Fields::default();

            for _ in 0..n {
                let field = read_string(r)?.expect("field should exist");
                fields.push(field);
            }

            info!("object: class={:?}, fields={:?}", class, fields);

            ctx.insert(class, fields);

            let tag = read_u8(r)?;

            read_value(ctx, r, tag)
        }
        0x60..=0x6f => {
            let reference = tag as usize - 0x60;
            let (class, fields) = ctx.nth(reference).expect("field should exist");

            let mut values = Vec::<Value>::with_capacity(fields.len());
            for _ in 0..fields.len() {
                let value = get_value(ctx, r)?;
                values.push(value);
            }

            let obj = Object::new(class, fields, values);

            info!("read object ok: {}", obj);

            Ok(Some(Value::from(obj)))
        }
        _ => todo!("unsupported tag: {:02x}", tag),
    }
}

pub fn get_value<R>(ctx: &mut Context, r: &mut R) -> io::Result<Value>
where
    R: io::Read + Sized,
{
    let tag: u8 = read_u8(r)?;

    debug!("read tag: {:02x}", tag);

    read_value(ctx, r, tag)?.ok_or(io::Error::from(io::ErrorKind::InvalidData))
}

fn read_map<R>(ctx: &mut Context, r: &mut R, dst: &mut Map) -> io::Result<()>
where
    R: io::Read,
{
    loop {
        let tag = read_u8(r)?;
        match read_value(ctx, r, tag)? {
            Some(item) => match item {
                Value::Primitive(key) => {
                    let val = get_value(ctx, r)?;
                    dst.insert(key, val);
                }
                _ => Err(io::Error::from(io::ErrorKind::InvalidData))?,
            },
            None => {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::encode;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_get_value_null() -> io::Result<()> {
        init();

        let mut ctx = Context::default();

        let b = vec![b'N'];

        let v = {
            let mut r = &b[..];
            get_value(&mut ctx, &mut r)?
        };

        assert_matches!(Value::Null, result);

        Ok(())
    }

    #[test]
    fn test_get_value_bool() -> io::Result<()> {
        init();

        for next in [true, false] {
            let mut ctx = Context::default();

            let b = {
                let mut b = vec![];
                encode::put_bool(&mut b, next)?;
                b
            };

            let o = {
                let mut r = &b[..];
                get_value(&mut ctx, &mut r)?
            };

            assert_matches!(Value::from(next), o);
        }

        Ok(())
    }

    #[test]
    fn test_get_value_i32() -> io::Result<()> {
        init();

        for next in [
            0i32,
            -16,
            47,
            -2048,
            -256,
            2047,
            -262144,
            262143,
            i32::MAX,
            i32::MIN,
        ] {
            let mut ctx = Context::default();
            let b = {
                let mut b = vec![];
                encode::put_i32(&mut b, next)?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut ctx, &mut r)?
            };

            assert_matches!(Value::from(next), v);
        }

        Ok(())
    }

    #[test]
    fn test_get_value_i64() -> io::Result<()> {
        init();

        for next in [
            0i64,
            -8,
            15,
            -16,
            47,
            -2048,
            -256,
            2047,
            -262144,
            262143,
            2147483648,
            i32::MAX as i64,
            i32::MIN as i64,
            i64::MAX,
            i64::MIN,
        ] {
            let mut ctx = Context::default();

            let b = {
                let mut b = vec![];
                encode::put_i64(&mut b, next)?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut ctx, &mut r)?
            };

            assert_matches!(Value::from(next), v);
        }

        Ok(())
    }

    #[test]
    fn test_get_value_string() -> io::Result<()> {
        init();

        for next in [
            "f".repeat(1023),
            "f".repeat(1025),
            format!("{}{}", "f".repeat(0x8000), "a".repeat(8)),
            format!("{}{}", "f".repeat(0x8000), "a".repeat(255)),
            format!("{}{}", "f".repeat(0x8000), "a".repeat(1024)),
        ] {
            let mut ctx = Context::default();
            let b = {
                let mut b = vec![];
                encode::put_str(&mut b, &next)?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut ctx, &mut r)?
            };

            assert_matches!(Value::from(next), v);
        }

        Ok(())
    }

    #[test]
    fn test_get_value_binary() -> io::Result<()> {
        init();

        let g = |n: usize| -> Vec<u8> { "f".repeat(n).into_bytes() };

        for next in [
            "hello world".to_string().into_bytes(),
            g(1023),
            g(1025),
            g(0x8000 + 8),
            g(0x8000 + 255),
            g(0x8000 + 1024),
        ] {
            let mut ctx = Context::default();
            let b = {
                let mut b = vec![];
                encode::put_bytes(&mut b, &next[..])?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut ctx, &mut r)?
            };

            assert_matches!(Value::from(next), v);
        }

        Ok(())
    }

    #[test]
    fn test_get_value_list() -> io::Result<()> {
        init();

        let mut ctx = Context::default();

        let b = {
            let mut b = vec![];

            encode::begin_list(&mut b, Some("java.util.LinkedList"), 3)?;

            encode::put_str(&mut b, "foo")?;
            encode::put_str(&mut b, "bar")?;
            encode::put_str(&mut b, "qux")?;
            b
        };

        info!("encode linked list: {}", hex::encode(&b));

        assert_eq!(
            "73146a6176612e7574696c2e4c696e6b65644c69737403666f6f0362617203717578",
            hex::encode(&b)
        );

        let mut r = &b[..];

        let actual = get_value(&mut ctx, &mut r)?;

        let expect = Value::from(
            vec!["foo", "bar", "qux"]
                .iter()
                .map(|v| Value::from(v.to_string()))
                .collect::<Vec<Value>>(),
        );

        assert_matches!(expect, v);

        Ok(())
    }

    #[test]
    fn test_get_value_map() -> io::Result<()> {
        init();

        for next in [
            "480362617292037175789303666f6f915a",
            "48036261727bfbe8ffd03c0bb8037175784801615b01625c01635f00000c445a03666f6f910362617a4a0000019f18a3a2885a", // untyped
            "4d176a6176612e7574696c2e4c696e6b6564486173684d617003666f6f91036261727bfbe8ffd03c0bb80362617a4a0000019f18a3a288037175784801615b01625c01635f00000c445a5a", // typed
        ] {
            let mut ctx = Context::default();

            let b = hex::decode(next).unwrap();

            let mut r = &b[..];

            let v = get_value(&mut ctx, &mut r)?;

            info!("decode result: {:?}", &v);
        }

        Ok(())
    }

    #[test]
    fn test_object() -> io::Result<()> {
        init();
        for next in
            ["4310636f6d2e6578616d706c652e5573657293026964046e616d650361676560fcd202e69da8e5b982a2"]
        {
            let mut ctx = Context::default();
            let b = hex::decode(next).unwrap();

            let mut r = &b[..];

            let v = get_value(&mut ctx, &mut r)?;

            info!("decode result: {}", &v);
        }

        Ok(())
    }
}
