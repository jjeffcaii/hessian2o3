use bytes::BufMut;
use std::collections::{HashMap, HashSet, LinkedList};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io::{self, Read};
use std::time::SystemTime;
// ─── Error ───────────────────────────────────────────────────────────────────

const BC_DATE_MINUTE: u8 = 0x4b;
const BC_DATE: u8 = 0x4a;
const BC_LONG_ZERO: u8 = 0xe0;
const BC_LONG_BYTE_ZERO: u8 = 0xf8;
const BC_LONG_SHORT_ZERO: u8 = 0x3c;
const BC_INT_ZERO: u8 = 0x90;
const BC_INT_BYTE_ZERO: u8 = 0xc8;

const BC_DOUBLE_ZERO: u8 = 0x5b;
const BC_DOUBLE_ONE: u8 = 0x5c;
const BC_DOUBLE_BYTE: u8 = 0x5d;
const BC_DOUBLE_SHORT: u8 = 0x5e;
const BC_DOUBLE_MILL: u8 = 0x5f;

const BC_STRING_DIRECT: u8 = 0x00;
const BC_STRING_SHORT: u8 = 0x30;
const BC_STRING_FINAL: u8 = b'S'; // final string
const BC_STRING_CHUNK: u8 = b'R'; // non-final string

const BC_LIST_DIRECT: u8 = 0x70;
const BC_LIST_DIRECT_UNTYPED: u8 = 0x78;
const BC_LIST_VARIABLE: u8 = 0x55;
const BC_LIST_FIXED: u8 = b'V';
const BC_LIST_VARIABLE_UNTYPED: u8 = 0x57;
const BC_LIST_FIXED_UNTYPED: u8 = 0x58;

const BC_MAP: u8 = b'M';
const BC_MAP_UNTYPED: u8 = b'H';

const BC_END: u8 = b'Z';

const LIST_DIRECT_MAX: usize = 7;

#[derive(Debug)]
pub enum SerdeError {
    Io(io::Error),
    InvalidData(String),
}

impl From<io::Error> for SerdeError {
    fn from(e: io::Error) -> Self {
        SerdeError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, SerdeError>;

#[derive(Debug)]
pub enum Kind {
    Null,
    Binary,
    Boolean,
    Class,
    Date,
    Double,
    Int,
    List,
    Long,
    Map,
    Object,
    Ref,
    String,
}

// ─── Traits ──────────────────────────────────────────────────────────────────

pub trait Encode {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()>;

    fn kind() -> Kind;

    /// 便捷方法：序列化到 Vec<u8>
    fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }
}

pub trait Decode: Sized {
    fn decode<R: Read>(r: &mut R) -> Result<Self>;

    /// 便捷方法：从 &[u8] 反序列化
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cursor = std::io::Cursor::new(bytes);
        Self::decode(&mut cursor)
    }
}

// ─── 基础读写工具 ──────────────────────────────────────────────────────────────

fn read_exact<R: Read>(r: &mut R, n: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

impl Encode for () {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        w.put_u8('N' as u8);
        Ok(())
    }

    fn kind() -> Kind {
        Kind::Null
    }
}

impl Encode for i32 {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        const DIRECT: (i32, i32) = (-16, 47);
        const BYTE: (i32, i32) = (-2048, 2047);
        const SHORT: (i32, i32) = (-0x40000, 0x3ffff);

        const BC_INT_SHORT_ZERO: u8 = 0xd4;

        if DIRECT.0 <= *self && *self <= DIRECT.1 {
            let first = (BC_INT_ZERO as i32) + *self;
            w.put_u8(first as u8);
        } else if BYTE.0 <= *self && *self <= BYTE.1 {
            let first = BC_INT_BYTE_ZERO as i32 + (*self >> 8);
            let second = *self & 0xff;
            w.put_u8(first as u8);
            w.put_u8(second as u8);
        } else if SHORT.0 <= *self && *self <= SHORT.1 {
            let first = (BC_INT_SHORT_ZERO as i32) + (*self >> 16);
            let second = *self >> 8;
            let third = *self & 0xff;
            w.put_u8(first as u8);
            w.put_u8(second as u8);
            w.put_u8(third as u8);
        } else {
            w.put_u8('I' as u8);
            w.put_i32(*self);
        }
        Ok(())
    }

    fn kind() -> Kind {
        Kind::Int
    }
}

impl Encode for i64 {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        const MAX32: i64 = i32::MAX as i64;
        const MIN32: i64 = i32::MIN as i64;

        const LONG_DIRECT_MIN: i64 = -8;
        const LONG_DIRECT_MAX: i64 = 15;
        const LONG_BYTE_MIN: i64 = -2048;
        const LONG_BYTE_MAX: i64 = 2047;
        const LONG_SHORT_MIN: i64 = -0x40000;
        const LONG_SHORT_MAX: i64 = 0x3ffff;

        if LONG_DIRECT_MIN <= *self && *self <= LONG_DIRECT_MAX {
            w.put_u8((BC_LONG_ZERO as i64 + *self) as u8);
        } else if LONG_BYTE_MIN <= *self && *self <= LONG_BYTE_MAX {
            let first = BC_LONG_BYTE_ZERO as i64 + (*self >> 8);
            let second = *self & 0xff;
            w.put_u8(first as u8);
            w.put_u8(second as u8);
        } else if LONG_SHORT_MIN <= *self && *self <= LONG_SHORT_MAX {
            let first = BC_LONG_SHORT_ZERO as i64 + (*self >> 16);
            let second = *self >> 8;
            let third = *self & 0xff;
            w.put_u8(first as u8);
            w.put_u8(second as u8);
            w.put_u8(third as u8);
        } else if MIN32 <= *self && *self <= MAX32 {
            const BC_LONG_INT: u8 = 0x59;
            w.put_u8(BC_LONG_INT);
            w.put_i32(*self as i32);
        } else {
            w.put_u8('L' as u8);
            w.put_i64(*self);
        }

        Ok(())
    }

    fn kind() -> Kind {
        Kind::Long
    }
}

impl Encode for bool {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        const TRUE: u8 = 'T' as u8;
        const FALSE: u8 = 'F' as u8;
        match *self {
            true => w.put_u8(TRUE),
            false => w.put_u8(FALSE),
        }
        Ok(())
    }

    fn kind() -> Kind {
        Kind::Boolean
    }
}

impl Encode for f64 {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        let v = *self;

        if v.is_finite() {
            let fract = v.fract();

            if fract == 0.0 {
                match v.trunc() as i64 {
                    0 => {
                        w.put_u8(BC_DOUBLE_ZERO);
                        return Ok(());
                    }
                    1 => {
                        w.put_u8(BC_DOUBLE_ONE);
                        return Ok(());
                    }
                    v @ -0x80..0x80 => {
                        w.put_u8(BC_DOUBLE_BYTE);
                        w.put_u8((v & 0xff) as u8);
                        return Ok(());
                    }
                    v @ -0x8000..0x8000 => {
                        w.put_u8(BC_DOUBLE_SHORT);
                        w.put_u8((v >> 8) as u8);
                        w.put_u8((v & 0xff) as u8);
                        return Ok(());
                    }
                    _ => (),
                }
            }

            if (1000f64 * fract).fract() == 0.0 {
                let v1000 = (1000f64 * v) as i32;
                w.put_u8(BC_DOUBLE_MILL);
                w.put_i32(v1000);
                return Ok(());
            }
        }

        w.put_u8('D' as u8);
        w.put_f64(*self);
        Ok(())
    }

    fn kind() -> Kind {
        Kind::Double
    }
}

impl Encode for str {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        const STRING_DIRECT_MAX: usize = 32 - 1;
        const STRING_SHORT_MAX: usize = 1024 - 1;

        let total = self.char_indices().count();
        let n = total / 0x8000;
        let tail = total % 0x8000;

        let mut cursor = self.chars();

        for _ in 0..n {
            w.put_u8(BC_STRING_CHUNK);
            w.put_u16(0x8000);

            for _ in 0..0x8000 {
                if let Some(ch) = cursor.next() {
                    put_char(w, ch)
                }
            }
        }

        if tail <= STRING_DIRECT_MAX {
            w.put_u8(BC_STRING_DIRECT + tail as u8);
        } else if tail <= STRING_SHORT_MAX {
            let first = (BC_STRING_SHORT as usize) + tail >> 8;
            w.put_u8(first as u8);
            w.put_u8((0xff & tail) as u8);
        } else {
            w.put_u8(BC_STRING_FINAL);
            w.put_u16(tail as u16);
        }

        while let Some(ch) = cursor.next() {
            put_char(w, ch)
        }

        Ok(())
    }

    fn kind() -> Kind {
        Kind::String
    }
}

impl Encode for String {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        let x = self.as_str();
        x.encode(w)
    }

    fn kind() -> Kind {
        Kind::String
    }
}

impl Encode for SystemTime {
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        let millis = self
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|it| it.as_millis() as i64)
            .unwrap_or(0);

        if millis % 60000i64 == 0 {
            let minutes = millis / 60000i64;
            match minutes >> 31 {
                0 | -1 => {
                    w.put_u8(BC_DATE_MINUTE);
                    w.put_i32(minutes as i32);
                    return Ok(());
                }
                _ => (),
            }
        }
        w.put_u8(BC_DATE);
        w.put_i64(millis);
        Ok(())
    }

    fn kind() -> Kind {
        Kind::Date
    }
}

impl<T> Encode for Option<T>
where
    T: Encode,
{
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        match self {
            None => ().encode(w),
            Some(t) => t.encode(w),
        }
    }

    fn kind() -> Kind {
        T::kind()
    }
}

impl<T> Encode for LinkedList<T>
where
    T: Encode,
{
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        encode_typed_list(w, "java.util.LinkedList", self.iter())
    }

    fn kind() -> Kind {
        Kind::List
    }
}

impl<T> Encode for Vec<T>
where
    T: Encode,
{
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        let length = self.len();

        if length <= LIST_DIRECT_MAX {
            let _kind = T::kind();
            w.put_u8(BC_LIST_DIRECT_UNTYPED + length as u8);
        } else {
            w.put_u8(BC_LIST_FIXED_UNTYPED);
            (length as i32).encode(w)?;
        }

        for next in self {
            next.encode(w)?;
        }

        Ok(())
    }

    fn kind() -> Kind {
        Kind::List
    }
}

impl<T> Encode for HashSet<T>
where
    T: Encode + Eq + Hash,
{
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        encode_typed_list(w, "java.util.HashSet", self.iter())
    }

    fn kind() -> Kind {
        Kind::List
    }
}

impl<K, V> Encode for HashMap<K, V>
where
    K: Encode + Eq + Hash,
    V: Encode,
{
    fn encode<W: BufMut>(&self, w: &mut W) -> Result<()> {
        w.put_u8(BC_MAP_UNTYPED);
        for (k, v) in self {
            k.encode(w)?;
            v.encode(w)?;
        }
        w.put_u8(BC_END);
        Ok(())
    }

    fn kind() -> Kind {
        Kind::Map
    }
}

#[inline]
fn encode_typed_list<'a, W, T, I>(w: &mut W, type_name: &str, iter: I) -> Result<()>
where
    W: BufMut,
    T: Encode + 'a,
    I: ExactSizeIterator<Item = &'a T>,
{
    let length = iter.len();
    if length <= LIST_DIRECT_MAX {
        w.put_u8(BC_LIST_DIRECT + length as u8);
        type_name.encode(w)?;
    } else {
        w.put_u8(BC_LIST_FIXED);
        type_name.encode(w)?;
        (length as i32).encode(w)?;
    }
    for item in iter {
        item.encode(w)?;
    }
    Ok(())
}

#[inline]
fn put_char<W: BufMut>(w: &mut W, c: char) {
    let ch = c as u32;
    if ch < 0x80 {
        w.put_u8(ch as u8);
    } else if ch < 0x800 {
        let first = 0xc0 + ((ch >> 6) & 0x1f) as u8;
        let second = 0x80 + (ch & 0x3f) as u8;
        w.put_u8(first);
        w.put_u8(second);
    } else {
        let first = 0xe0 + ((ch >> 12) & 0xf) as u8;
        let second = 0x80 + ((ch >> 6) & 0x3f) as u8;
        let third = 0x80 + (ch & 0x3f) as u8;
        w.put_u8(first);
        w.put_u8(second);
        w.put_u8(third);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use chrono::DateTime;
    use nom::AsBytes;
    use std::fmt::Debug;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    fn encode<T>(x: &T) -> String
    where
        T: Encode + Debug,
    {
        let mut buf = BytesMut::with_capacity(64);
        let result = x.encode(&mut buf);
        assert!(result.is_ok());
        let hexstr = hex::encode(buf.as_bytes());
        info!("{:?}: {}", x, hexstr);

        hexstr
    }

    #[test]
    fn test_encode_bool() {
        assert_eq!("54", encode(&true));
        assert_eq!("46", encode(&false));
    }

    #[test]
    fn test_encode_i64() {
        init();
        assert_eq!("e0", encode(&0i64));
        assert_eq!("d8", encode(&-8i64));
        assert_eq!("ef", encode(&15i64));
        assert_eq!("f000", encode(&-2048i64));
        assert_eq!("f700", encode(&-256i64));
        assert_eq!("ffff", encode(&2047i64));
        assert_eq!("380000", encode(&-262144i64));
        assert_eq!("3fffff", encode(&262143i64));
        assert_eq!("4c0000000080000000", encode(&2147483648i64));
    }

    #[test]
    fn test_encode_i32() {
        init();

        assert_eq!("90", encode(&0i32));
        assert_eq!("80", encode(&-16i32));
        assert_eq!("bf", encode(&47i32));
        assert_eq!("c000", encode(&-2048i32));
        assert_eq!("c700", encode(&-256i32));
        assert_eq!("cfff", encode(&2047i32));

        assert_eq!("d00000", encode(&-262144i32));
        assert_eq!("d7ffff", encode(&262143i32));
    }

    #[test]
    fn test_encode_f64() {
        init();

        assert_eq!("5b", encode(&0.0f64));
        assert_eq!("5c", encode(&1.0f64));
        assert_eq!("5d80", encode(&-128.0f64));
        assert_eq!("5d7f", encode(&127.0f64));
        assert_eq!("5e8000", encode(&-32768.0f64));
        assert_eq!("5e7fff", encode(&32767.0f64));
        assert_eq!("44400921fb54442d18", encode(&std::f64::consts::PI));
        assert_eq!("447ff0000000000000", encode(&f64::INFINITY));
        assert_eq!("44fff0000000000000", encode(&f64::NEG_INFINITY));
        assert_eq!("447ff8000000000000", encode(&f64::NAN));
    }

    #[test]
    fn test_encode_systemtime() {
        init();

        {
            let rfc3339_str = "2026-06-10T15:16:17+08:00";
            let datetime = DateTime::parse_from_rfc3339(rfc3339_str).unwrap();
            let system_time: SystemTime = SystemTime::from(datetime);

            assert_eq!("4a0000019eb06395e8", encode(&system_time));
        }
        {
            let rfc3339_str = "2026-06-10T15:16:00+08:00";
            let datetime = DateTime::parse_from_rfc3339(rfc3339_str).unwrap();
            let system_time: SystemTime = SystemTime::from(datetime);

            assert_eq!("4b01c4f374", encode(&system_time));
        }
    }

    #[test]
    fn test_encode_str() {
        init();

        assert_eq!("00", encode(&String::from("")));
        assert_eq!("0568656c6c6f", encode(&String::from("hello")));
        assert_eq!("01c383", encode(&String::from("\u{00c3}")));

        assert_eq!(
            "530bfd666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f",
            encode(&String::from("foo".repeat(1023)))
        );
    }

    #[test]
    fn test_encode_hashset() {
        init();

        let mut list: HashSet<i64> = Default::default();

        // assert_eq!("70116a6176612e7574696c2e48617368536574", encode(&list));

        list.insert(111);
        list.insert(222);
        list.insert(333);

        assert_eq!(
            "73116a6176612e7574696c2e48617368536574f94df8def86f",
            encode(&list)
        );
        // list.insert(444);
        // list.insert(555);
        // list.insert(666);
        // list.insert(777);
        // list.insert(888);
        // list.insert(999);
        //
        // assert_eq!(
        //     "56116a6176612e7574696c2e4861736853657499fbe7fb78fb09fa9afa2bf9bcf94df8def86f",
        //     encode(&list)
        // );
    }

    #[test]
    fn test_encode_linkedlist() {
        {
            let mut list: LinkedList<i64> = Default::default();

            assert_eq!(
                "70146a6176612e7574696c2e4c696e6b65644c697374",
                encode(&list)
            );

            list.push_back(111);
            list.push_back(222);
            list.push_back(333);

            assert_eq!(
                "73146a6176612e7574696c2e4c696e6b65644c697374f86ff8def94d",
                encode(&list)
            );
            list.push_back(444);
            list.push_back(555);
            list.push_back(666);
            list.push_back(777);
            list.push_back(888);
            list.push_back(999);

            assert_eq!(
                "56146a6176612e7574696c2e4c696e6b65644c69737499f86ff8def94df9bcfa2bfa9afb09fb78fbe7",
                encode(&list)
            );
        }
    }

    #[test]
    fn test_encode_vec() {
        init();
        {
            let mut list: Vec<i64> = Default::default();
            assert_eq!("78", encode(&list));

            list.push(111);
            list.push(222);
            list.push(333);
            assert_eq!("7bf86ff8def94d", encode(&list));

            list.push(444);
            list.push(555);
            list.push(666);
            list.push(777);
            list.push(888);
            list.push(999);

            assert_eq!("5899f86ff8def94df9bcfa2bfa9afb09fb78fbe7", encode(&list));
        }

        {
            let list = vec!["foo".to_string(), "bar".to_string(), "qux".to_string()];
            assert_eq!("7b03666f6f0362617203717578", encode(&list));
        }
    }

    #[test]
    fn test_encode_hashmap() {
        init();

        {
            let mut m: HashMap<i32, String> = Default::default();
            m.insert(1, "foo".into());
            m.insert(2, "bar".into());
            m.insert(3, "qux".into());

            assert_eq!("489103666f6f920362617293037175785a", encode(&m));
        }
    }
}
