use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Hessian2 Date 的包装类型，实现和 SystemTime 的双向转换
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HessianDate(pub SystemTime);

impl HessianDate {
    #[inline]
    pub fn from_millis(unix_millis: u64) -> Self {
        HessianDate(UNIX_EPOCH + Duration::from_millis(unix_millis))
    }

    #[inline]
    pub fn now() -> Self {
        Self(SystemTime::now())
    }
}

impl Serialize for HessianDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis = self
            .0
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?
            .as_millis() as i64;
        // 关键：不能直接调用 serialize_i64，会走成普通 long 编码
        // 必须用自定义方法标记这是"日期"语义
        serializer.serialize_newtype_struct("$hessian::date", &millis)
    }
}

impl<'de> Deserialize<'de> for HessianDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = deserializer.deserialize_newtype_struct("$hessian::date", MillisVisitor)?;
        Ok(HessianDate(
            UNIX_EPOCH + Duration::from_millis(millis as u64),
        ))
    }
}

struct MillisVisitor;

impl<'de> serde::de::Visitor<'de> for MillisVisitor {
    type Value = i64;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an i64 millisecond timestamp")
    }

    fn visit_i64<E>(self, v: i64) -> Result<i64, E> {
        Ok(v)
    }
}

impl From<SystemTime> for HessianDate {
    fn from(value: SystemTime) -> Self {
        HessianDate(value)
    }
}
