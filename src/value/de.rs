use super::{List, Map, Object, PrimitiveValue, Value};
use crate::Error;
use serde::de::{self, Error as _, IntoDeserializer};
use serde::{Deserializer, forward_to_deserialize_any};
use std::fmt;
use std::fmt::Formatter;
use std::time::{Duration, SystemTime};

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("any valid hessian value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::from(value))
            }

            fn visit_i8<E>(self, value: i8) -> Result<Value, E> {
                Ok(Value::from(value as i32))
            }

            fn visit_i16<E>(self, value: i16) -> Result<Value, E> {
                Ok(Value::from(value as i32))
            }

            fn visit_i32<E>(self, value: i32) -> Result<Value, E> {
                Ok(Value::from(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::from(value))
            }

            fn visit_u8<E>(self, value: u8) -> Result<Value, E> {
                Ok(Value::from(value as i32))
            }

            fn visit_u16<E>(self, value: u16) -> Result<Value, E> {
                Ok(Value::from(value as i32))
            }

            fn visit_u32<E>(self, value: u32) -> Result<Value, E> {
                Ok(Value::from(value as i64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::from(value as i64))
            }

            fn visit_u128<E>(self, value: u128) -> Result<Value, E>
            where
                E: de::Error,
            {
                // Mirrors how `PrimitiveValue::Date` is serialized as unix
                // millis (see `impl Serialize for PrimitiveValue`).
                let millis = u64::try_from(value).map_err(de::Error::custom)?;
                Ok(Value::from(
                    SystemTime::UNIX_EPOCH + Duration::from_millis(millis),
                ))
            }

            fn visit_f32<E>(self, value: f32) -> Result<Value, E> {
                Ok(Value::from(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::from(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E> {
                Ok(Value::from(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::from(value))
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Value, E> {
                Ok(Value::from(value.to_vec()))
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Value, E> {
                Ok(Value::from(value))
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, mut visitor: A) -> Result<Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut vec = vec![];
                while let Some(item) = visitor.next_element::<Value>()? {
                    vec.push(item);
                }

                Ok(Value::from(vec))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut m = Map::with_capacity(map.size_hint().unwrap_or(0));
                while let Some(key) = map.next_key::<Value>()? {
                    let key = match key {
                        Value::Primitive(pv) => pv,
                        _ => {
                            return Err(de::Error::custom(
                                "map key must be a hessian primitive value",
                            ));
                        }
                    };
                    let value = map.next_value::<Value>()?;
                    m.insert(key, value);
                }
                Ok(Value::Map(m))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Interprets a [`Value`] as an instance of type `T`, using the `Value` as
/// a [`Deserializer`].
///
/// This is the reverse of [`to_value`](super::to_value).
pub fn from_value<T>(value: Value) -> crate::Result<T>
where
    T: de::DeserializeOwned,
{
    T::deserialize(value)
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Value>,
}

impl SeqDeserializer {
    fn new(iter: std::vec::IntoIter<Value>) -> Self {
        SeqDeserializer { iter }
    }
}

impl<'de> de::SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

struct MapDeserializer {
    iter: std::vec::IntoIter<(PrimitiveValue, Value)>,
    value: Option<Value>,
}

impl MapDeserializer {
    fn new(entries: Vec<(PrimitiveValue, Value)>) -> Self {
        MapDeserializer {
            iter: entries.into_iter(),
            value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(de::Error::custom("hessian map value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

fn visit_array<'de, V>(list: List, visitor: V) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
{
    let len = list.len();
    let mut deserializer = SeqDeserializer::new(list.into_iter());
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(de::Error::invalid_length(len, &"fewer elements in list"))
    }
}

fn visit_map_value<'de, V>(map: Map, visitor: V) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
{
    let len = map.len();
    let mut deserializer = MapDeserializer::new(map.into_iter().collect());
    let value = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(value)
    } else {
        Err(de::Error::invalid_length(len, &"fewer elements in map"))
    }
}

fn visit_object<'de, V>(object: Object, visitor: V) -> Result<V::Value, Error>
where
    V: de::Visitor<'de>,
{
    let len = object.len();
    let entries: Vec<(PrimitiveValue, Value)> = object
        .into_fields()
        .map(|(k, v)| (PrimitiveValue::String(k.to_string()), v))
        .collect();
    let mut deserializer = MapDeserializer::new(entries);
    let value = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(value)
    } else {
        Err(de::Error::invalid_length(len, &"fewer elements in object"))
    }
}

// hessian2o3's `Serializer` drops enum variant tags entirely for
// newtype/tuple/struct variants (see `value::ser`): only unit variants
// survive the round trip, encoded as a bare string. `deserialize_enum`
// below can therefore only ever recover unit variants.
const ENUM_TAG_LOST_MSG: &str = "hessian2o3 can only deserialize a unit enum variant (encoded as a string) from a Value; \
     tuple/struct/newtype variants lose their tag when serialized and cannot be recovered";

impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Primitive(PrimitiveValue::Bool(b)) => visitor.visit_bool(b),
            Value::Primitive(PrimitiveValue::Int(i)) => visitor.visit_i32(i),
            Value::Primitive(PrimitiveValue::Long(l)) => visitor.visit_i64(l),
            Value::Primitive(PrimitiveValue::Double(d)) => visitor.visit_f64(d),
            Value::Primitive(PrimitiveValue::Date(d)) => {
                let millis = d
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(Error::custom)?
                    .as_millis();
                visitor.visit_u128(millis)
            }
            Value::Primitive(PrimitiveValue::Binary(b)) => visitor.visit_byte_buf(b),
            Value::Primitive(PrimitiveValue::String(s)) => visitor.visit_string(s),
            Value::List(l) => visit_array(l, visitor),
            Value::Map(m) => visit_map_value(m, visitor),
            Value::Object(o) => visit_object(o, visitor),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
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
        match self {
            Value::Primitive(PrimitiveValue::String(variant)) => {
                visitor.visit_enum(variant.into_deserializer())
            }
            _ => Err(de::Error::custom(ENUM_TAG_LOST_MSG)),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

impl<'de> Deserializer<'de> for PrimitiveValue {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            PrimitiveValue::Bool(b) => visitor.visit_bool(b),
            PrimitiveValue::Int(i) => visitor.visit_i32(i),
            PrimitiveValue::Long(l) => visitor.visit_i64(l),
            PrimitiveValue::Double(d) => visitor.visit_f64(d),
            PrimitiveValue::Date(d) => {
                let millis = d
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(Error::custom)?
                    .as_millis();
                visitor.visit_u128(millis)
            }
            PrimitiveValue::Binary(b) => visitor.visit_byte_buf(b),
            PrimitiveValue::String(s) => visitor.visit_string(s),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
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
    ) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            PrimitiveValue::String(variant) => visitor.visit_enum(variant.into_deserializer()),
            _ => Err(de::Error::custom(ENUM_TAG_LOST_MSG)),
        }
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
    use crate::cachestr::Cachestr;
    use crate::value::to_value;
    use serde::{Deserialize, Serialize};
    use smallvec::smallvec;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_value_deserialize_from_json() -> anyhow::Result<()> {
        init();

        let json = r#"{"id":123,"name":"Jerry","age":18,"tags":["a","b"],"nick":null}"#;
        let v: Value = serde_json::from_str(json)?;

        let mut expect = Map::default();
        expect.insert(PrimitiveValue::String("id".to_owned()), Value::from(123i64));
        expect.insert(
            PrimitiveValue::String("name".to_owned()),
            Value::from("Jerry".to_owned()),
        );
        expect.insert(PrimitiveValue::String("age".to_owned()), Value::from(18i64));
        expect.insert(
            PrimitiveValue::String("tags".to_owned()),
            Value::from(vec![
                Value::from("a".to_owned()),
                Value::from("b".to_owned()),
            ]),
        );
        expect.insert(PrimitiveValue::String("nick".to_owned()), Value::Null);

        assert_eq!(Value::Map(expect), v);

        Ok(())
    }

    #[test]
    fn test_from_value_roundtrip_struct() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct User {
            id: i64,
            name: String,
            age: i32,
        }

        let user = User {
            id: 123,
            name: "Jerry".to_owned(),
            age: 18,
        };

        let value = to_value(&user)?;
        info!("value: {:?}", &value);
        let back: User = from_value(value)?;
        info!("back: {:?}", &back);

        assert_eq!(user, back);

        Ok(())
    }

    #[test]
    fn test_from_value_seq_and_scalars() -> anyhow::Result<()> {
        init();

        let value = Value::from(vec![
            Value::from(1i32),
            Value::from(2i32),
            Value::from(3i32),
        ]);
        let back: Vec<i32> = from_value(value)?;
        assert_eq!(vec![1, 2, 3], back);

        assert!(from_value::<bool>(Value::from(true))?);
        assert_eq!(42i64, from_value::<i64>(Value::from(42i32))?);
        assert_eq!(
            "hello".to_owned(),
            from_value::<String>(Value::from("hello".to_owned()))?
        );
        assert_eq!(None::<i32>, from_value::<Option<i32>>(Value::Null)?);
        assert_eq!(Some(7i32), from_value::<Option<i32>>(Value::from(7i32))?);

        Ok(())
    }

    #[test]
    fn test_from_value_object_into_struct() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Deserialize)]
        struct User {
            id: i64,
            name: String,
            age: i32,
        }

        let object = Object::new(
            Cachestr::from("com.example.User"),
            smallvec![
                Cachestr::from("id"),
                Cachestr::from("name"),
                Cachestr::from("age"),
            ],
            vec![
                Value::from(123i64),
                Value::from("Jerry".to_owned()),
                Value::from(18i32),
            ],
        );

        let user: User = from_value(Value::Object(object))?;
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
    fn test_from_value_unit_enum_variant() -> anyhow::Result<()> {
        init();

        #[derive(Debug, PartialEq, Deserialize)]
        enum Direction {
            North,
            South,
        }

        assert_eq!(
            Direction::North,
            from_value(Value::from("North".to_owned()))?
        );
        assert_eq!(
            Direction::South,
            from_value(Value::from("South".to_owned()))?
        );

        Ok(())
    }

    #[test]
    fn test_from_value_wrong_shape_errors() {
        init();

        let value = Value::from(vec![Value::from(1i32), Value::from(2i32)]);
        let result: crate::Result<(i32,)> = from_value(value);
        assert!(result.is_err());
    }
}
