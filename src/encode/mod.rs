mod list;
mod primitive;
// mod map;

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

pub trait KindSupport {
    fn kind() -> Kind;
}

pub trait Typed {
    fn type_name() -> &'static str;
}

use crate::Result;
use bytes::{Buf, BufMut};

pub trait Encode: Sized {
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()>;

    /// 便捷方法：序列化到 Vec<u8>
    fn to_bytes(self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(buf)
    }

    fn to_hex_string(self) -> Result<String> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        Ok(hex::encode(&buf))
    }
}

pub trait Decode: Sized {
    fn decode<R: Buf>(r: &mut R) -> Result<Self>;

    /// 便捷方法：从 &[u8] 反序列化
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cursor = std::io::Cursor::new(bytes);
        Self::decode(&mut cursor)
    }
}
