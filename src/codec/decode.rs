use super::misc::*;
use crate::value::Value;
use std::io;
use std::time;

#[inline(always)]
fn millis_to_system_time(millis: i64) -> time::SystemTime {
    if millis >= 0 {
        time::SystemTime::UNIX_EPOCH + time::Duration::from_millis(millis as u64)
    } else {
        // process timestamp before 1970
        time::UNIX_EPOCH - time::Duration::from_millis(millis.unsigned_abs())
    }
}

#[inline(always)]
fn read_utf8<R>(r: &mut R, n: usize) -> io::Result<String>
where
    R: io::Read,
{
    let mut result = String::with_capacity(n);
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

        result.push_str(s);
    }

    Ok(result)
}

#[inline(always)]
fn read_f64<R>(r: &mut R) -> io::Result<f64>
where
    R: io::Read,
{
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

#[inline(always)]
fn read_i16<R>(r: &mut R) -> io::Result<i16>
where
    R: io::Read,
{
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

#[inline(always)]
fn read_i32<R>(r: &mut R) -> io::Result<i32>
where
    R: io::Read,
{
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

#[inline(always)]
fn read_i64<R>(r: &mut R) -> io::Result<i64>
where
    R: io::Read,
{
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

#[inline(always)]
fn read_i8<R>(r: &mut R) -> io::Result<i8>
where
    R: io::Read + Sized,
{
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0] as i8)
}

#[inline(always)]
fn read_u8<R>(r: &mut R) -> io::Result<u8>
where
    R: io::Read + Sized,
{
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub fn get_value<R>(r: &mut R) -> io::Result<Value>
where
    R: io::Read + Sized,
{
    let tag: u8 = read_u8(r)?;

    debug!("read tag: {:02x}", tag);

    match tag {
        0x00..=0x1f => {
            let length = tag as usize - 0x00;
            Ok(Value::String(read_utf8(r, length)?))
        }
        0x30..=0x33 => {
            let length = {
                let high = (tag - 0x30) as usize;
                let low = read_u8(r)? as usize;
                (high << 8) + low
            };
            Ok(Value::String(read_utf8(r, length)?))
        }
        BC_NULL => Ok(Value::Null),
        BC_BOOL_TRUE => Ok(Value::Bool(true)),
        BC_BOOL_FALSE => Ok(Value::Bool(false)),
        // direct integer
        0x80..=0xbf => {
            let direct = (tag as i8) - (BC_INT_ZERO as i8);
            Ok(Value::Int(direct as i32))
        }
        // byte integer
        0xc0..=0xcf => {
            let low = read_u8(r)? as i32;
            let high = (((tag as i8) - (BC_INT_BYTE_ZERO as i8)) as i32) << 8;
            Ok(Value::Int(high + low))
        }
        // short integer
        0xd0..=0xd7 => {
            let high = ((tag as i8) - (BC_INT_SHORT_ZERO as i8)) as i32;
            let middle = read_u8(r)? as i32;
            let low = read_u8(r)? as i32;

            Ok(Value::Int((high << 16) + (middle << 8) + low))
        }
        // integer
        BC_INT => {
            let v = read_i32(r)?;
            Ok(Value::Int(v))
        }
        // direct long
        0xd8..=0xef => {
            let direct = (tag as i8) - (BC_LONG_ZERO as i8);
            Ok(Value::Long(direct as i64))
        }
        // byte long
        0xf0..=0xff => {
            let low = read_u8(r)? as i64;
            let high = (((tag as i8) - (BC_LONG_BYTE_ZERO as i8)) as i64) << 8;
            Ok(Value::Long(high + low))
        }
        // short long
        0x38..=0x3f => {
            let high = ((tag as i8) - (BC_LONG_SHORT_ZERO as i8)) as i64;
            let middle = read_u8(r)? as i64;
            let low = read_u8(r)? as i64;

            Ok(Value::Long((high << 16) + (middle << 8) + low))
        }
        // integer long
        BC_LONG_INT => {
            let v = read_i32(r)?;
            Ok(Value::Long(v as i64))
        }
        // long
        BC_LONG => {
            let v = read_i64(r)?;
            Ok(Value::Long(v))
        }
        BC_DOUBLE_ZERO => Ok(Value::Double(0f64)),
        BC_DOUBLE_ONE => Ok(Value::Double(1f64)),
        BC_DOUBLE_BYTE => {
            let v = read_i8(r)?;
            Ok(Value::Double(v as f64))
        }
        BC_DOUBLE_SHORT => {
            let v = read_i16(r)?;
            Ok(Value::Double(v as f64))
        }
        BC_DOUBLE_MILL => {
            let v = read_i32(r)? as f64;
            Ok(Value::Double(0.001f64 * v))
        }
        BC_DOUBLE => {
            let v = read_f64(r)?;
            Ok(Value::Double(v))
        }
        BC_DATE => {
            let v = read_i64(r)?;
            Ok(Value::Date(millis_to_system_time(v)))
        }
        BC_DATE_MINUTE => {
            let unix_mills = (read_i32(r)? as i64) * 60000i64;
            Ok(Value::Date(millis_to_system_time(unix_mills)))
        }
        _ => todo!("unsupported tag: {:02x}", tag),
    }
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

        let b = vec![b'N'];

        let v = {
            let mut r = &b[..];
            get_value(&mut r)?
        };

        assert_matches!(Value::Null, result);

        Ok(())
    }

    #[test]
    fn test_get_value_bool() -> io::Result<()> {
        init();

        for next in [true, false] {
            let b = {
                let mut b = vec![];
                encode::put_bool(&mut b, next)?;
                b
            };

            let o = {
                let mut r = &b[..];
                get_value(&mut r)?
            };

            assert_matches!(Value::Bool(next), o);
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
            let b = {
                let mut b = vec![];
                encode::put_i32(&mut b, next)?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut r)?
            };

            assert_matches!(Value::Int(next), v);
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
            let b = {
                let mut b = vec![];
                encode::put_i64(&mut b, next)?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut r)?
            };

            assert_matches!(Value::Long(next), v);
        }

        Ok(())
    }

    #[test]
    fn test_get_value_string() -> io::Result<()> {
        init();

        for next in ["a".repeat(1023)] {
            let b = {
                let mut b = vec![];
                encode::put_str(&mut b, &next)?;
                b
            };

            let v = {
                let mut r = &b[..];
                get_value(&mut r)?
            };

            assert_matches!(Value::String(next.into()), v);
        }

        Ok(())
    }
}
