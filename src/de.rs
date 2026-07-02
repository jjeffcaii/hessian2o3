use crate::codec::{self, Context};
use crate::error::Error;
use crate::value::Value;
use serde::de;
use serde::forward_to_deserialize_any;
use std::io;

/// A hessian deserializer that reads values from an [`io::Read`] stream.
///
/// Decoding is done in two steps: the byte-level core in [`crate::codec`]
/// parses the stream into a [`Value`] tree, which then acts as the
/// [`serde::Deserializer`] (see `value::de`). The [`Context`] carries
/// class-definition references, so several objects of the same class can be
/// read from one stream.
pub struct Deserializer<R> {
    reader: R,
    context: Context,
}

impl<R> Deserializer<R>
where
    R: io::Read,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            context: Context::default(),
        }
    }

    /// Decodes the next hessian value from the stream.
    pub fn read_value(&mut self) -> crate::Result<Value> {
        codec::get_value(&mut self.context, &mut self.reader).map_err(Error::io)
    }
}

impl<'de, R> de::Deserializer<'de> for &mut Deserializer<R>
where
    R: io::Read,
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        self.read_value()?.deserialize_any(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        self.read_value()?.deserialize_option(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        self.read_value()?.deserialize_enum(name, variants, visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec as encode;
    use crate::serde::{from_slice, to_vec};
    use serde::{Deserialize, Serialize};

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_from_slice_scalars() -> anyhow::Result<()> {
        init();

        assert!(from_slice::<bool>(&to_vec(&true)?)?);
        assert!(!from_slice::<bool>(&to_vec(&false)?)?);
        assert_eq!(123i32, from_slice::<i32>(&to_vec(&123i32)?)?);
        assert_eq!(-262144i32, from_slice::<i32>(&to_vec(&-262144i32)?)?);
        assert_eq!(i64::MAX, from_slice::<i64>(&to_vec(&i64::MAX)?)?);
        assert_eq!(2.5f64, from_slice::<f64>(&to_vec(&2.5f64)?)?);
        assert_eq!(
            "foobar".to_owned(),
            from_slice::<String>(&to_vec("foobar")?)?
        );
        assert_eq!(None::<i32>, from_slice::<Option<i32>>(&to_vec(&())?)?);
        assert_eq!(Some(7i32), from_slice::<Option<i32>>(&to_vec(&7i32)?)?);

        Ok(())
    }

    #[test]
    fn test_from_slice_containers() -> anyhow::Result<()> {
        init();

        let v = vec![1i32, 2, 3];
        assert_eq!(v, from_slice::<Vec<i32>>(&to_vec(&v)?)?);

        let mut m = std::collections::BTreeMap::new();
        m.insert("foo".to_owned(), 1i32);
        m.insert("bar".to_owned(), 2i32);
        assert_eq!(
            m,
            from_slice::<std::collections::BTreeMap<String, i32>>(&to_vec(&m)?)?
        );

        Ok(())
    }

    #[test]
    fn test_from_slice_struct_roundtrip() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct User {
            id: i64,
            name: String,
            age: i32,
            nick: Option<String>,
        }

        let user = User {
            id: 123,
            name: "Jerry".to_owned(),
            age: 18,
            nick: None,
        };

        let b = to_vec(&user)?;
        let back: User = from_slice(&b)?;

        assert_eq!(user, back);

        Ok(())
    }

    #[test]
    fn test_from_slice_object() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Deserialize)]
        struct User {
            id: i64,
            name: String,
            age: i32,
        }

        // encode a class-based hessian object ('C' definition + instance)
        let b = {
            let mut b = vec![];
            let mut ctx = Context::default();
            encode::begin_object(&mut b, &mut ctx, "com.example.User", &["id", "name", "age"])?;
            encode::put_i64(&mut b, 123)?;
            encode::put_str(&mut b, "Jerry")?;
            encode::put_i32(&mut b, 18)?;
            b
        };

        let user: User = from_slice(&b)?;

        assert_eq!(
            User {
                id: 123,
                name: "Jerry".to_owned(),
                age: 18,
            },
            user
        );

        Ok(())
    }

    #[test]
    fn test_deserializer_multiple_values_share_class_refs() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Deserialize)]
        struct Point {
            x: i32,
            y: i32,
        }

        // two objects of the same class in one stream: the second instance
        // only carries a class *reference*, so decoding must reuse the
        // Context of the first.
        let b = {
            let mut b = vec![];
            let mut ctx = Context::default();
            encode::begin_object(&mut b, &mut ctx, "com.example.Point", &["x", "y"])?;
            encode::put_i32(&mut b, 1)?;
            encode::put_i32(&mut b, 2)?;
            encode::begin_object(&mut b, &mut ctx, "com.example.Point", &["x", "y"])?;
            encode::put_i32(&mut b, 3)?;
            encode::put_i32(&mut b, 4)?;
            b
        };

        let mut de = Deserializer::new(&b[..]);
        let p1 = Point::deserialize(&mut de)?;
        let p2 = Point::deserialize(&mut de)?;

        assert_eq!(Point { x: 1, y: 2 }, p1);
        assert_eq!(Point { x: 3, y: 4 }, p2);

        Ok(())
    }

    #[test]
    fn test_from_slice_unit_enum_variant() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Direction {
            North,
            South,
        }

        let b = to_vec(&Direction::North)?;
        assert_eq!(Direction::North, from_slice::<Direction>(&b)?);

        Ok(())
    }
}
