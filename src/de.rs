use crate::cachestr::Cachestr;
use crate::codec::{self, Fields, Header, HeaderFamily};
use crate::error::Error;
use crate::value::{Map, Object, Value};
use serde::de::{self, Error as _, IntoDeserializer as _};
use serde::forward_to_deserialize_any;
use smallvec::SmallVec;
use std::io;

/// A hessian deserializer that reads values from an [`io::Read`] stream.
///
/// Decoding is done in two steps: the byte-level core in [`codec`]
/// parses the stream into a [`Value`] tree, which then acts as the
/// [`serde::Deserializer`] (see `value::de`). The [`Context`] carries
/// class-definition references, so several objects of the same class can be
/// read from one stream.
pub struct Deserializer<R> {
    r: codec::Decoder<R>,
}

impl<R> Deserializer<R>
where
    R: io::Read,
{
    pub fn new(reader: R) -> Self {
        Self {
            r: codec::Decoder::new(reader),
        }
    }

    /// Decodes a single [`Value`] tree from the stream, preserving the
    /// distinction between plain maps (`Value::Map`) and class-tagged
    /// objects (`Value::Object`) that a generic `serde::Deserialize` for
    /// `Value` cannot express, since `serde::de::Visitor` has no callback
    /// that carries a class name alongside `visit_map`.
    pub(crate) fn read_value(&mut self) -> Result<Value, Error> {
        let header = self.r.peek()?;

        let value = match header.family() {
            HeaderFamily::Null => {
                self.r.read_null()?;
                Value::Null
            }
            HeaderFamily::Boolean => Value::from(self.r.read_bool()?),
            HeaderFamily::Int => Value::from(self.r.read_i32()?),
            HeaderFamily::Long => Value::from(self.r.read_i64()?),
            HeaderFamily::Binary => Value::from(self.r.read_binary()?),
            HeaderFamily::String => Value::from(self.r.read_string()?),
            HeaderFamily::Double => Value::from(self.r.read_f64()?),
            HeaderFamily::Date => Value::from(self.r.read_date()?),
            HeaderFamily::List => {
                let (_class, length) = self.r.begin_list()?;
                let items = match length {
                    Some(length) => {
                        let mut items = Vec::with_capacity(length);
                        for _ in 0..length {
                            items.push(self.read_value()?);
                        }
                        items
                    }
                    None => {
                        let mut items = Vec::new();
                        loop {
                            if let Header::End = self.r.peek()? {
                                self.r.consume(1);
                                break;
                            }
                            items.push(self.read_value()?);
                        }
                        items
                    }
                };
                Value::from(items)
            }
            HeaderFamily::Map => {
                let class = self.r.begin_map()?;
                let mut m = Map::new();
                if let Some(class) = class {
                    m.set_class(class);
                }
                loop {
                    if let Header::End = self.r.peek()? {
                        self.r.consume(1);
                        break;
                    }
                    let key = match self.read_value()? {
                        Value::Primitive(pv) => pv,
                        other => {
                            return Err(Error::custom(format!(
                                "map key must be a hessian primitive value, but got: {:?}",
                                other
                            )));
                        }
                    };
                    let val = self.read_value()?;
                    m.insert(key, val);
                }
                Value::from(m)
            }
            HeaderFamily::Class => {
                self.r.read_class()?;
                self.read_value()?
            }
            HeaderFamily::ClassRef => {
                let (class, fields) = self.r.read_class_ref()?;
                let mut values = Vec::with_capacity(fields.len());
                for _ in 0..fields.len() {
                    values.push(self.read_value()?);
                }
                Value::from(Object::new(class, fields, values))
            }
        };

        Ok(value)
    }

    /// Reads a boolean value.
    pub fn read_bool(&mut self) -> Result<bool, Error> {
        self.r.read_bool()
    }

    /// Reads an integer, accepting both int and long wire flavors.
    pub fn read_i64(&mut self) -> Result<i64, Error> {
        match self.r.peek()?.family() {
            HeaderFamily::Int => {
                let i = self.r.read_i32()?;
                Ok(i as i64)
            }
            HeaderFamily::Long => self.r.read_i64(),
            HeaderFamily::Double => {
                let f = self.r.read_f64()?;
                Ok(f as i64)
            }
            HeaderFamily::Date => {
                let i = self.r.read_date()?;
                Ok(i)
            }
            _ => self.r.read_i64(),
        }
    }

    /// Reads an integer, accepting both int and long wire flavors;
    /// errors if the value doesn't fit an i32.
    pub fn read_i32(&mut self) -> Result<i32, Error> {
        match self.r.peek()?.family() {
            HeaderFamily::Int => self.r.read_i32(),
            HeaderFamily::Long => match self.r.read_i64()?.try_into() {
                Ok(i) => Ok(i),
                Err(e) => Err(Error::custom(e)),
            },
            HeaderFamily::Double => {
                // It's safe by saturating casting.
                Ok(self.r.read_f64()? as i32)
            }
            _ => self.r.read_i32(),
        }
    }

    /// Reads a double value.
    pub fn read_f64(&mut self) -> Result<f64, Error> {
        match self.r.peek()?.family() {
            HeaderFamily::Int => Ok(self.r.read_i32()? as f64),
            HeaderFamily::Long => Ok(self.r.read_i64()? as f64),
            HeaderFamily::Double => self.r.read_f64(),
            HeaderFamily::Date => Ok(self.r.read_date()? as f64),
            _ => self.r.read_f64(),
        }
    }

    /// Reads a string value.
    pub fn read_string(&mut self) -> Result<String, Error> {
        self.r.read_string()
    }

    /// Reads a binary value.
    pub fn read_binary(&mut self) -> Result<Vec<u8>, Error> {
        self.r.read_binary()
    }

    /// Consumes a null (`N`) if it is the next value, returning whether one
    /// was consumed.
    pub fn try_read_null(&mut self) -> Result<bool, Error> {
        match self.r.peek()? {
            Header::Null => {
                self.r.read_null()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Begins reading a list, returning its element count. A fixed-length
    /// list returns `Some(n)` and the caller must then read exactly `n`
    /// values; a variable-length list returns `None` and the caller must
    /// read values until [`try_end_list`](Deserializer::try_end_list)
    /// returns `true`.
    pub fn begin_list(&mut self) -> Result<Option<usize>, Error> {
        let (_class, length) = self.r.begin_list()?;
        Ok(length)
    }

    /// Consumes the list end tag (`'Z'`) if it is the next value, returning
    /// whether the list ended.
    pub fn try_end_list(&mut self) -> Result<bool, Error> {
        match self.r.peek()? {
            Header::End => {
                self.r.consume(1);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Begins reading a hessian object: consumes any pending class
    /// definitions (`C`) plus the object's class reference, and returns an
    /// [`ObjectReader`] over its fields.
    pub fn begin_object(&mut self) -> Result<ObjectReader<'_, R>, Error> {
        match self.r.peek()?.family() {
            HeaderFamily::Class => {
                let class_ref = self.r.read_class()?;
                debug!("read class ok: ref={:?}", class_ref);
                self.begin_object()
            }
            HeaderFamily::ClassRef => {
                let (class, fields) = self.r.read_class_ref()?;

                if log_enabled!(log::Level::Debug) {
                    let mut b: SmallVec<[u8; 128]> = smallvec![];
                    let mut it = fields.iter();
                    if let Some(f) = it.next() {
                        use io::Write as _;
                        write!(&mut b, "{}", f.as_ref())?;
                        for f in it {
                            write!(&mut b, ",{}", f.as_ref())?;
                        }
                    }
                    let fields_str = unsafe { std::str::from_utf8_unchecked(&b[..]) };

                    debug!(
                        "read class_ref ok: class={}, fields=[{}]",
                        class, fields_str
                    );
                }

                Ok(ObjectReader::new(self, fields))
            }
            other => Err(Error::custom(format!("unexpect hessian type {}", other))),
        }
    }
}

/// Streaming access to the fields of a hessian object, created by
/// [`Deserializer::begin_object`]. Field names come from the class
/// definition in declaration order; after each `next_field`, exactly one of
/// [`value`](ObjectReader::value) or [`skip_value`](ObjectReader::skip_value)
/// must be called to consume the field's value.
pub struct ObjectReader<'a, R> {
    de: &'a mut Deserializer<R>,
    fields: Fields,
    index: usize,
}

impl<'a, R> ObjectReader<'a, R> {
    #[inline]
    fn new(de: &'a mut Deserializer<R>, fields: Fields) -> Self {
        Self {
            de,
            fields,
            index: 0,
        }
    }
}

impl<'a, R: io::Read> ObjectReader<'a, R> {
    /// Returns the next field name, or `None` when all fields are consumed.
    pub fn next_field(&mut self) -> Option<Cachestr> {
        let field = self.fields.get(self.index)?;
        debug!("**** next_field: {}", field.as_ref());
        self.index += 1;
        Some(Clone::clone(field))
    }

    /// Deserializes the current field's value.
    pub fn value<T: crate::HessianDeserialize>(&mut self) -> Result<T, Error> {
        T::hessian_deserialize(self.de)
    }

    /// Decodes and discards the current field's value.
    pub fn skip_value(&mut self) -> Result<(), Error> {
        self.de.read_value().map(drop)
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
        let header = self.r.peek()?;
        debug!("begin deserialize_any: header={:?}", header);

        match header.family() {
            HeaderFamily::Null => {
                self.r.read_null()?;
                visitor.visit_unit()
            }
            HeaderFamily::Boolean => {
                let b = self.r.read_bool()?;
                visitor.visit_bool(b)
            }
            HeaderFamily::Int => {
                let i = self.r.read_i32()?;
                visitor.visit_i32(i)
            }
            HeaderFamily::Long => {
                let i = self.r.read_i64()?;
                visitor.visit_i64(i)
            }
            HeaderFamily::Binary => {
                let b = self.r.read_binary()?;
                visitor.visit_byte_buf(b)
            }
            HeaderFamily::String => {
                let s = self.r.read_string()?;
                visitor.visit_string(s)
            }
            HeaderFamily::Double => {
                let f = self.r.read_f64()?;
                visitor.visit_f64(f)
            }
            HeaderFamily::Date => {
                let unix_millis = self.r.read_date()?;
                visitor.visit_i64(unix_millis)
            }
            HeaderFamily::List => {
                let (_class, length) = self.r.begin_list()?;
                visitor.visit_seq(SeqAccess::new(self, length))
            }
            HeaderFamily::Map => {
                let _class = self.r.begin_map()?;
                visitor.visit_map(MapAccess::new(self))
            }
            HeaderFamily::Class => {
                let class_ref = self.r.read_class()?;
                debug!("read class ok: class_ref={}", class_ref);
                self.deserialize_any(visitor)
            }
            HeaderFamily::ClassRef => {
                let (_class, fields) = self.r.read_class_ref()?;
                visitor.visit_map(ObjectAccess::new(self, fields))
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self.r.peek()? {
            Header::Null => {
                self.r.read_null()?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(VariantAccess::new(self))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct VariantAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R> VariantAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>) -> Self {
        VariantAccess { de }
    }
}

impl<'de, 'a, R: io::Read + 'a> de::EnumAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let v = seed.deserialize(&mut *self.de)?;
        Ok((v, self))
    }
}

impl<'de, 'a, R: io::Read + 'a> de::VariantAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

struct ObjectAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
    fields: Fields,
    index: usize,
}

impl<'a, R: 'a> ObjectAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, fields: Fields) -> Self {
        Self {
            de,
            fields,
            index: 0,
        }
    }
}

impl<'de, 'a, R: io::Read + 'a> de::MapAccess<'de> for ObjectAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.index >= self.fields.len() {
            return Ok(None);
        }

        let field = FieldName(Clone::clone(&self.fields[self.index]));
        self.index += 1;

        seed.deserialize(field).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

struct FieldName(Cachestr);

impl<'de> de::Deserializer<'de> for FieldName {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_str(self.0.as_ref())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self.0.into_deserializer())
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct MapAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R: 'a> MapAccess<'a, R> {
    #[inline]
    fn new(de: &'a mut Deserializer<R>) -> MapAccess<'a, R> {
        MapAccess { de }
    }
}

impl<'de, 'a, R: io::Read + 'a> de::MapAccess<'de> for MapAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        Ok(match self.de.r.peek()? {
            Header::End => {
                self.de.r.consume(1);
                None
            }
            _ => {
                let key = seed.deserialize(&mut *self.de)?;
                Some(key)
            }
        })
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

struct SeqAccess<'a, R: 'a> {
    de: &'a mut Deserializer<R>,
    // `None` for variable-length lists, which end with a 'Z' tag.
    remaining: Option<usize>,
}

impl<'a, R: 'a> SeqAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, size: Option<usize>) -> Self {
        SeqAccess {
            de,
            remaining: size,
        }
    }
}

impl<'de, 'a, R: io::Read + 'a> de::SeqAccess<'de> for SeqAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.remaining {
            Some(0) => Ok(None),
            Some(ref mut n) => {
                *n -= 1;
                seed.deserialize(&mut *self.de).map(Some)
            }
            None => {
                if let Header::End = self.de.r.peek()? {
                    self.de.r.consume(1);
                    return Ok(None);
                }
                seed.deserialize(&mut *self.de).map(Some)
            }
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use crate::codec::Encoder;
    use crate::serde::{from_slice, to_vec};
    use serde::{Deserialize, Serialize};

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_from_slice_scalars() -> Result<()> {
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
            let mut enc = Encoder::new(&mut b);
            enc.begin_object("com.example.User", &["id", "name", "age"])?;
            enc.put_i64(123)?;
            enc.put_str("Jerry")?;
            enc.put_i32(18)?;
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
            let mut enc = Encoder::new(&mut b);
            enc.begin_object("com.example.Point", &["x", "y"])?;
            enc.put_i32(1)?;
            enc.put_i32(2)?;
            enc.begin_object("com.example.Point", &["x", "y"])?;
            enc.put_i32(3)?;
            enc.put_i32(4)?;
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
    fn test_from_slice_variable_list() -> anyhow::Result<()> {
        init();

        // x57 value* 'Z': untyped variable-length list
        let b = hex::decode("579192935a")?;
        assert_eq!(vec![1i32, 2, 3], from_slice::<Vec<i32>>(&b)?);

        // x55 type value* 'Z': typed variable-length list ("[int")
        let b = hex::decode("55045b696e749192935a")?;
        assert_eq!(vec![1i32, 2, 3], from_slice::<Vec<i32>>(&b)?);

        Ok(())
    }

    #[test]
    fn test_read_value_variable_list() -> anyhow::Result<()> {
        init();

        use crate::value::Value;

        let b = hex::decode("579192935a")?;
        let mut de = Deserializer::new(&b[..]);
        let v = de.read_value()?;

        assert_eq!(
            Value::from(vec![
                Value::from(1i32),
                Value::from(2i32),
                Value::from(3i32)
            ]),
            v
        );

        Ok(())
    }

    #[test]
    fn test_from_slice_unit_enum_variant() -> Result<()> {
        init();

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        enum Direction {
            East,
            South,
            West,
            North,
        }

        let b = to_vec(&Direction::West)?;
        let actual = from_slice::<Direction>(&b)?;
        assert_matches!(actual, Direction::West);

        Ok(())
    }
}
