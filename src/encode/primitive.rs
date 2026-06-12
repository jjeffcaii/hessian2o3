use super::{Encode, Kind, KindSupport};
use crate::Result;
use bytes::BufMut;
use std::fmt::Write;
use std::time::SystemTime;

const TRUE: u8 = b'T';
const FALSE: u8 = b'F';

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

fn write_null<W>(w: &mut W) -> Result<()>
where
    W: BufMut,
{
    w.put_u8('N' as u8);
    Ok(())
}

impl KindSupport for i32 {
    fn kind() -> Kind {
        Kind::Int
    }
}

impl Encode for i32 {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        (&self).encode(w)
    }
}

impl Encode for &i32 {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
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
}

impl KindSupport for i64 {
    fn kind() -> Kind {
        Kind::Long
    }
}

impl Encode for i64 {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        (&self).encode(w)
    }
}

impl Encode for &i64 {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
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
}

impl KindSupport for bool {
    fn kind() -> Kind {
        Kind::Boolean
    }
}

impl Encode for &bool {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        match *self {
            true => w.put_u8(TRUE),
            false => w.put_u8(FALSE),
        }
        Ok(())
    }
}

impl KindSupport for f64 {
    fn kind() -> Kind {
        Kind::Double
    }
}

impl Encode for &f64 {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
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
}

#[inline(always)]
fn write_str<W: BufMut>(w: &mut W, s: &str) -> Result<()> {
    const STRING_DIRECT_MAX: usize = 32 - 1;
    const STRING_SHORT_MAX: usize = 1024 - 1;

    let total = s.char_indices().count();
    let n = total / 0x8000;
    let tail = total % 0x8000;

    let mut cursor = s.chars();

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

impl Encode for &str {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        write_str(w, self)
    }
}

impl Encode for String {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        write_str(w, self.as_str())
    }
}

impl Encode for &String {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        write_str(w, self.as_str())
    }
}

impl KindSupport for SystemTime {
    fn kind() -> Kind {
        Kind::Date
    }
}

impl Encode for &SystemTime {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
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
}

impl<T> KindSupport for Option<T>
where
    T: KindSupport,
{
    fn kind() -> Kind {
        T::kind()
    }
}

impl<'a, T> Encode for &'a Option<T>
where
    &'a T: Encode,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        match self {
            None => write_null(w),
            Some(t) => t.encode(w),
        }
    }
}

impl<'a, T> Encode for Option<&'a T>
where
    &'a T: Encode,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        match self {
            None => write_null(w),
            Some(t) => t.encode(w),
        }
    }
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
    use chrono::DateTime;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_encode_bool() -> Result<()> {
        assert_eq!("54", true.to_hex_string()?);
        assert_eq!("46", false.to_hex_string()?);

        Ok(())
    }

    #[test]
    fn test_encode_i64() -> Result<()> {
        init();
        assert_eq!("e0", 0i64.to_hex_string()?);
        assert_eq!("d8", (-8i64).to_hex_string()?);
        assert_eq!("ef", 15i64.to_hex_string()?);
        assert_eq!("f000", (-2048i64).to_hex_string()?);
        assert_eq!("f700", (-256i64).to_hex_string()?);
        assert_eq!("ffff", 2047i64.to_hex_string()?);
        assert_eq!("380000", (-262144i64).to_hex_string()?);
        assert_eq!("3fffff", 262143i64.to_hex_string()?);
        assert_eq!("4c0000000080000000", 2147483648i64.to_hex_string()?);

        Ok(())
    }

    #[test]
    fn test_encode_i32() -> Result<()> {
        init();

        assert_eq!("90", 0i32.to_hex_string()?);
        assert_eq!("80", (-16i32).to_hex_string()?);
        assert_eq!("bf", 47i32.to_hex_string()?);
        assert_eq!("c000", (-2048i32).to_hex_string()?);
        assert_eq!("c700", (-256i32).to_hex_string()?);
        assert_eq!("cfff", 2047i32.to_hex_string()?);

        assert_eq!("d00000", (-262144i32).to_hex_string()?);
        assert_eq!("d7ffff", 262143i32.to_hex_string()?);

        Ok(())
    }

    #[test]
    fn test_encode_f64() -> Result<()> {
        init();

        assert_eq!("5b", 0.0f64.to_hex_string()?);
        assert_eq!("5c", 1.0f64.to_hex_string()?);
        assert_eq!("5d80", (-128.0f64).to_hex_string()?);
        assert_eq!("5d7f", 127.0f64.to_hex_string()?);
        assert_eq!("5e8000", (-32768.0f64).to_hex_string()?);
        assert_eq!("5e7fff", 32767.0f64.to_hex_string()?);
        assert_eq!("44400921fb54442d18", std::f64::consts::PI.to_hex_string()?);
        assert_eq!("447ff0000000000000", f64::INFINITY.to_hex_string()?);
        assert_eq!("44fff0000000000000", f64::NEG_INFINITY.to_hex_string()?);
        assert_eq!("447ff8000000000000", f64::NAN.to_hex_string()?);

        Ok(())
    }

    #[test]
    fn test_encode_systemtime() -> Result<()> {
        init();

        {
            let rfc3339_str = "2026-06-10T15:16:17+08:00";
            let datetime = DateTime::parse_from_rfc3339(rfc3339_str)?;
            let system_time: SystemTime = SystemTime::from(datetime);

            assert_eq!("4a0000019eb06395e8", system_time.to_hex_string()?);
        }
        {
            let rfc3339_str = "2026-06-10T15:16:00+08:00";
            let datetime = DateTime::parse_from_rfc3339(rfc3339_str)?;
            let system_time: SystemTime = SystemTime::from(datetime);

            assert_eq!("4b01c4f374", system_time.to_hex_string()?);
        }

        Ok(())
    }

    #[test]
    fn test_encode_str() -> Result<()> {
        init();

        assert_eq!("00", "".to_hex_string()?);
        assert_eq!("0568656c6c6f", "hello".to_hex_string()?);
        assert_eq!("01c383", "\u{00c3}".to_hex_string()?);

        assert_eq!(
            "530bfd666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f666f6f",
            &String::from("foo".repeat(1023)).to_hex_string()?
        );

        Ok(())
    }

    #[test]
    fn test_encode_option() -> Result<()> {
        init();

        let o = Some("hello".to_string());

        assert_eq!("0568656c6c6f", o.to_hex_string()?);

        let o: Option<i32> = None;
        assert_eq!("4e", o.to_hex_string()?);

        Ok(())
    }
}
