use super::tags::*;
use crate::cachestr::Cachestr;
use crate::codec::{Context, Fields};
use crate::value::{Map, Object, PrimitiveValue, Value};
use crate::{misc, Error, Result};
use serde::de::Error as _;
use std::fmt;
use std::io::{self, BufRead as _, BufReader, Read as _};

#[derive(Debug)]
pub(crate) enum HeaderFamily {
    Null,
    Boolean,
    Int,
    Long,
    Binary,
    String,

    Double,
    Date,
    List,
    Map,
    Class,
    ClassRef,
}

impl fmt::Display for HeaderFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeaderFamily::Null => f.write_str("null"),
            HeaderFamily::Boolean => f.write_str("boolean"),
            HeaderFamily::Int => f.write_str("int"),
            HeaderFamily::Long => f.write_str("long"),
            HeaderFamily::Binary => f.write_str("binary"),
            HeaderFamily::String => f.write_str("string"),
            HeaderFamily::Double => f.write_str("double"),
            HeaderFamily::Date => f.write_str("date"),
            HeaderFamily::List => f.write_str("list"),
            HeaderFamily::Map => f.write_str("map"),
            HeaderFamily::Class => f.write_str("class"),
            HeaderFamily::ClassRef => f.write_str("class_ref"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Header {
    Null,
    Boolean(u8),
    StringDirect(u8),
    StringShort(u8),
    StringChunk,
    StringFinal,

    BinaryDirect(u8),
    BinaryShort(u8),
    BinaryChunk,
    BinaryFinal,

    Int0(u8),
    Int8(u8),
    Int16(u8),
    Int32,

    Long0(u8),
    Long8(u8),
    Long16(u8),
    Long32,
    Long64,

    DoubleZero,
    DoubleOne,

    Double8,
    Double16,
    Double32,
    Double64,

    Date32,
    Date64,

    BeginTypedList0(u8),
    BeginUntypedList0(u8),

    BeginUntypedMap,
    BeginTypedMap,
    End,

    BeginClass,
    BeginClassReference(u8),
}

impl Header {
    #[inline]
    pub(crate) fn family(&self) -> HeaderFamily {
        match self {
            // null
            Header::Null => HeaderFamily::Null,

            // boolean
            Header::Boolean(_) => HeaderFamily::Boolean,

            // string
            Header::StringDirect(_)
            | Header::StringShort(_)
            | Header::StringChunk
            | Header::StringFinal => HeaderFamily::String,

            // binary
            Header::BinaryDirect(_)
            | Header::BinaryShort(_)
            | Header::BinaryChunk
            | Header::BinaryFinal => HeaderFamily::Binary,

            // int
            Header::Int0(_) | Header::Int8(_) | Header::Int16(_) | Header::Int32 => {
                HeaderFamily::Int
            }

            // long
            Header::Long0(_)
            | Header::Long8(_)
            | Header::Long16(_)
            | Header::Long32
            | Header::Long64 => HeaderFamily::Long,

            // double
            Header::DoubleZero
            | Header::DoubleOne
            | Header::Double8
            | Header::Double16
            | Header::Double32
            | Header::Double64 => HeaderFamily::Double,

            // date
            Header::Date32 | Header::Date64 => HeaderFamily::Date,

            // list
            Header::BeginTypedList0(_) | Header::BeginUntypedList0(_) => HeaderFamily::List,
            // map
            Header::BeginUntypedMap | Header::BeginTypedMap | Header::End => HeaderFamily::Map,
            // class
            Header::BeginClass => HeaderFamily::Class,
            // object
            Header::BeginClassReference(_) => HeaderFamily::ClassRef,
        }
    }
}

#[inline(always)]
fn unexpect_type<T>(expect: HeaderFamily, actual: HeaderFamily) -> Result<T> {
    Err(Error::custom(format!(
        "expect hessian {}, but got {}",
        expect, actual
    )))
}

pub(crate) struct Reader<R> {
    r: BufReader<R>,
    ctx: Context,
}

impl<R> Reader<R> {
    pub fn new(reader: R) -> Self
    where
        R: io::Read,
    {
        Self {
            r: BufReader::new(reader),
            ctx: Context::default(),
        }
    }
}

impl<R> Reader<R>
where
    R: io::Read + Sized,
{
    #[inline]
    pub(crate) fn consume(&mut self, n: usize) {
        self.r.consume(n)
    }

    #[inline]
    pub(crate) fn peek(&mut self) -> Result<Header> {
        let buf = {
            let mut buf = self.r.buffer();
            if buf.is_empty() {
                buf = self.r.fill_buf()?;
            }
            buf
        };

        let first = buf
            .first()
            .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;

        let header = match *first {
            0x00..=0x1f => Header::StringDirect(*first),
            0x30..=0x33 => Header::StringShort(*first),
            BC_STRING_CHUNK => Header::StringChunk,
            BC_STRING => Header::StringFinal,

            BC_BINARY_CHUNK => Header::BinaryChunk,
            BC_BINARY => Header::BinaryFinal,
            0x20..=0x2f => Header::BinaryDirect(*first),
            0x34..=0x37 => Header::BinaryShort(*first),

            BC_NULL => Header::Null,

            BC_BOOL_TRUE => Header::Boolean(*first),
            BC_BOOL_FALSE => Header::Boolean(*first),

            0x80..=0xbf => Header::Int0(*first),
            0xc0..=0xcf => Header::Int8(*first),
            0xd0..=0xd7 => Header::Int16(*first),
            BC_INT => Header::Int32,

            0xd8..=0xef => Header::Long0(*first),
            0xf0..=0xff => Header::Long8(*first),
            0x38..=0x3f => Header::Long16(*first),
            BC_LONG_INT => Header::Long32,
            BC_LONG => Header::Long64,

            BC_DOUBLE_ZERO => Header::DoubleZero,
            BC_DOUBLE_ONE => Header::DoubleOne,
            BC_DOUBLE_BYTE => Header::Double8,
            BC_DOUBLE_SHORT => Header::Double16,
            BC_DOUBLE_MILL => Header::Double32,
            BC_DOUBLE => Header::Double64,

            BC_DATE_MINUTE => Header::Date32,
            BC_DATE => Header::Date64,

            0x70..=0x77 => Header::BeginTypedList0(*first),
            0x78..=0x7f => Header::BeginUntypedList0(*first),

            BC_MAP_UNTYPED => Header::BeginUntypedMap,
            BC_MAP => Header::BeginTypedMap,
            BC_END => Header::End,

            BC_CLASS => Header::BeginClass,
            0x60..=0x6f => Header::BeginClassReference(*first),

            other => {
                return Err(Error::custom(format!("invalid hessian header {}", other)));
            }
        };

        Ok(header)
    }

    pub(crate) fn read_class_ref(&mut self) -> Result<(Cachestr, Fields)> {
        match self.peek()? {
            Header::BeginClassReference(i) => {
                self.consume(1);
                let class_ref = i - 0x60;
                let (class, fields) = self.ctx.nth(class_ref as usize).ok_or_else(|| {
                    Error::custom(format!("class-ref#{} is not found", class_ref))
                })?;
                Ok((class, fields))
            }
            other => unexpect_type(HeaderFamily::ClassRef, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_class(&mut self) -> Result<usize> {
        match self.peek()? {
            Header::BeginClass => {
                self.consume(1);

                let class = Cachestr::from(self.read_string()?);
                let n = self.read_i32()?;

                let mut fields = Fields::default();
                for _ in 0..n {
                    let field = Cachestr::from(self.read_string()?);
                    fields.push(field);
                }

                debug!("read class '{}': fields={:?}", class.as_ref(), fields);

                let idx = self.ctx.insert(class, fields);
                Ok(idx)
            }
            other => unexpect_type(HeaderFamily::Class, other.family()),
        }
    }

    #[inline]
    pub(crate) fn begin_map(&mut self) -> Result<Option<Cachestr>> {
        match self.peek()? {
            Header::BeginTypedMap => {
                self.consume(1);

                let class = Cachestr::from(self.read_string()?);

                Ok(Some(class))
            }
            Header::BeginUntypedMap => {
                self.consume(1);
                Ok(None)
            }
            other => unexpect_type(HeaderFamily::Map, other.family()),
        }
    }

    #[inline]
    pub(crate) fn begin_list(&mut self) -> Result<(Option<Cachestr>, usize)> {
        match self.peek()? {
            Header::BeginTypedList0(n) => {
                self.consume(1);
                let length = (n - BC_LIST_DIRECT) as usize;
                let class = Cachestr::from(self.read_string()?);
                Ok((Some(class), length))
            }
            Header::BeginUntypedList0(n) => {
                self.consume(1);
                let length = (n - BC_LIST_DIRECT_UNTYPED) as usize;
                Ok((None, length))
            }
            other => unexpect_type(HeaderFamily::List, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_null(&mut self) -> Result<()> {
        match self.peek()? {
            Header::Null => {
                self.consume(1);
                Ok(())
            }
            other => unexpect_type(HeaderFamily::Null, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_bool(&mut self) -> Result<bool> {
        match self.peek()? {
            Header::Boolean(n) => {
                self.consume(1);
                match n {
                    BC_BOOL_TRUE => Ok(true),
                    BC_BOOL_FALSE => Ok(false),
                    _ => unreachable!(),
                }
            }
            other => unexpect_type(HeaderFamily::Boolean, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_string(&mut self) -> Result<String> {
        match self.peek()? {
            Header::StringDirect(n) => {
                self.consume(1);
                let length = n as usize;
                let mut s = String::with_capacity(length);
                read_utf8(&mut self.r, &mut s, length)?;
                Ok(s)
            }
            Header::StringShort(n) => {
                self.consume(1);

                let length = {
                    let high = (n - BC_STRING_SHORT) as usize;
                    let low = read_u8(&mut self.r)? as usize;
                    (high << 8) + low
                };
                let mut s = String::with_capacity(length);
                read_utf8(&mut self.r, &mut s, length)?;
                Ok(s)
            }
            Header::StringChunk => {
                self.consume(1);

                let mut s = String::with_capacity(0x8000 + 0x8000 / 2);

                let length = {
                    let high = read_u8(&mut self.r)? as usize;
                    let low = read_u8(&mut self.r)? as usize;
                    (high << 8) + low
                };

                read_utf8_chunked(&mut self.r, &mut s, length, false)?;

                Ok(s)
            }
            Header::StringFinal => {
                self.consume(1);

                let length = {
                    let high = read_u8(&mut self.r)? as usize;
                    let low = read_u8(&mut self.r)? as usize;
                    (high << 8) + low
                };
                let mut s = String::with_capacity(length);

                read_utf8_chunked(&mut self.r, &mut s, length, true)?;

                Ok(s)
            }
            other => unexpect_type(HeaderFamily::String, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_i32(&mut self) -> Result<i32> {
        match self.peek()? {
            Header::Int0(n) => {
                self.consume(1);
                let direct = (n as i8) - (BC_INT_ZERO as i8);
                Ok(direct as i32)
            }
            Header::Int8(n) => {
                self.consume(1);
                let low = read_u8(&mut self.r)? as i32;
                let high = (((n as i8) - (BC_INT_BYTE_ZERO as i8)) as i32) << 8;
                Ok(high + low)
            }
            Header::Int16(n) => {
                self.consume(1);
                let num = {
                    let high = ((n as i8) - (BC_INT_SHORT_ZERO as i8)) as i32;
                    let middle = read_u8(&mut self.r)? as i32;
                    let low = read_u8(&mut self.r)? as i32;
                    (high << 16) + (middle << 8) + low
                };
                Ok(num)
            }
            Header::Int32 => {
                self.consume(1);
                let v = read_i32(&mut self.r)?;
                Ok(v)
            }
            other => unexpect_type(HeaderFamily::Int, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_i64(&mut self) -> Result<i64> {
        match self.peek()? {
            Header::Long0(n) => {
                self.consume(1);
                let num = {
                    let direct = (n as i8) - (BC_LONG_ZERO as i8);
                    direct as i64
                };
                Ok(num)
            }
            Header::Long8(n) => {
                self.consume(1);

                let num = {
                    let low = read_u8(&mut self.r)? as i64;
                    let high = (((n as i8) - (BC_LONG_BYTE_ZERO as i8)) as i64) << 8;
                    high + low
                };

                Ok(num)
            }
            Header::Long16(n) => {
                self.consume(1);

                let num = {
                    let high = ((n as i8) - (BC_LONG_SHORT_ZERO as i8)) as i64;
                    let middle = read_u8(&mut self.r)? as i64;
                    let low = read_u8(&mut self.r)? as i64;
                    (high << 16) + (middle << 8) + low
                };

                Ok(num)
            }
            Header::Long32 => {
                self.consume(1);
                let num = {
                    let v = read_i32(&mut self.r)?;
                    v as i64
                };
                Ok(num)
            }
            Header::Long64 => {
                self.consume(1);
                Ok(read_i64(&mut self.r)?)
            }
            other => unexpect_type(HeaderFamily::Long, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_binary(&mut self) -> Result<Vec<u8>> {
        match self.peek()? {
            Header::BinaryDirect(n) => {
                self.consume(1);

                let length = n as usize - 0x20;
                let mut b = vec![0; length];
                self.r.read_exact(&mut b[..])?;

                Ok(b)
            }
            Header::BinaryShort(n) => {
                self.consume(1);

                let length = {
                    let high = (n as usize) - (BC_BINARY_SHORT as usize);
                    let low = read_u8(&mut self.r)? as usize;
                    (high << 8) + low
                };

                let mut b = vec![0; length];
                self.r.read_exact(&mut b[..])?;

                Ok(b)
            }
            Header::BinaryChunk => {
                self.consume(1);
                let length = {
                    let high = read_u8(&mut self.r)? as usize;
                    let low = read_u8(&mut self.r)? as usize;
                    (high << 8) + low
                };

                let mut b = Vec::<u8>::with_capacity(length + length / 2);

                read_binary_chunked(&mut self.r, &mut b, length, false)?;

                Ok(b)
            }
            Header::BinaryFinal => {
                self.consume(1);

                let length = {
                    let high = read_u8(&mut self.r)? as usize;
                    let low = read_u8(&mut self.r)? as usize;
                    (high << 8) + low
                };

                let mut b = vec![0; length];
                self.r.read_exact(&mut b[..])?;

                Ok(b)
            }
            other => unexpect_type(HeaderFamily::Binary, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_f64(&mut self) -> Result<f64> {
        match self.peek()? {
            Header::DoubleZero => {
                self.consume(1);
                Ok(0f64)
            }
            Header::DoubleOne => {
                self.consume(1);
                Ok(1f64)
            }
            Header::Double8 => {
                self.consume(1);
                let v = read_i8(&mut self.r)?;
                Ok(v as f64)
            }
            Header::Double16 => {
                self.consume(1);
                let v = read_i16(&mut self.r)?;
                Ok(v as f64)
            }
            Header::Double32 => {
                self.consume(1);
                let v = read_i32(&mut self.r)? as f64;
                Ok(0.001f64 * v)
            }
            Header::Double64 => {
                self.consume(1);
                Ok(read_f64(&mut self.r)?)
            }
            other => unexpect_type(HeaderFamily::Double, other.family()),
        }
    }

    #[inline]
    pub(crate) fn read_date(&mut self) -> Result<i64> {
        match self.peek()? {
            Header::Date32 => {
                self.consume(1);
                let unix_mills = (read_i32(&mut self.r)? as i64) * 60000i64;
                Ok(unix_mills)
            }
            Header::Date64 => {
                self.consume(1);
                let v = read_i64(&mut self.r)?;
                Ok(v)
            }
            other => unexpect_type(HeaderFamily::Date, other.family()),
        }
    }
}

#[inline]
fn read_binary<R>(r: &mut R, dst: &mut Vec<u8>, n: usize) -> io::Result<()>
where
    R: io::Read,
{
    let start = dst.len();
    dst.resize(start + n, 0);
    r.read_exact(&mut dst[start..])
}

#[inline]
fn read_binary_chunked<R>(r: &mut R, dst: &mut Vec<u8>, n: usize, is_final: bool) -> io::Result<()>
where
    R: io::Read,
{
    read_binary(r, dst, n)?;
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
            read_binary_chunked(r, dst, length, false)
        }
        BC_BINARY => {
            let length = {
                let high = read_u8(r)? as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            read_binary_chunked(r, dst, length, true)
        }
        0x20..=0x2f => {
            let length = (code - 0x20) as usize;
            read_binary_chunked(r, dst, length, true)
        }
        0x34..=0x37 => {
            let length = {
                let high = (code - 0x34) as usize;
                let low = read_u8(r)? as usize;

                (high << 8) + low
            };
            read_binary_chunked(r, dst, length, true)
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
            let length = tag as usize;
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
            let length = tag as usize;
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
            // string direct
            let length = tag as usize;
            let mut s = String::with_capacity(length);
            read_utf8(r, &mut s, length)?;
            Ok(Some(Value::from(s)))
        }
        0x20..=0x2f => {
            // binary direct
            let length = tag as usize - 0x20;
            let mut b = vec![0; length];
            r.read_exact(&mut b[..])?;

            Ok(Some(Value::from(b)))
        }
        0x30..=0x33 => {
            // string short
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
            // binary short
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

            read_binary_chunked(r, &mut b, length, false)?;

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
            // typed list direct
            let length = tag as usize - 0x70;
            let class = read_string(r)?;

            info!("list class {:?}", class);
            let mut v = Vec::<Value>::with_capacity(length);
            read_list(ctx, r, &mut v, length)?;

            Ok(Some(Value::from(v)))
        }
        0x78..=0x7f => {
            // untyped list direct
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
                if let Value::Primitive(PrimitiveValue::Int(i)) = get_value(ctx, r)? {
                    n = i
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

        let b = [b'N'];

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
            ["foo", "bar", "qux"]
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
        {
            let next = "4310636f6d2e6578616d706c652e5573657293026964046e616d650361676560fcd202e69da8e5b982a2";
            let mut ctx = Context::default();
            let b = hex::decode(next).unwrap();

            let mut r = &b[..];

            let v = get_value(&mut ctx, &mut r)?;

            info!("decode result: {}", &v);
        }

        Ok(())
    }
}
