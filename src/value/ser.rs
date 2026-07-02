use super::value::Value;
use crate::Error;
use crate::value::{List, Map, Object, PrimitiveValue};
use serde::{Serialize, ser};
use std::time::SystemTime;

impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Primitive(pv) => pv.serialize(serializer),
            Value::List(l) => l.serialize(serializer),
            Value::Map(m) => m.serialize(serializer),
            Value::Object(o) => o.serialize(serializer),
        }
    }
}

impl serde::Serialize for PrimitiveValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PrimitiveValue::Bool(b) => serializer.serialize_bool(*b),
            PrimitiveValue::Int(i) => serializer.serialize_i32(*i),
            PrimitiveValue::Long(l) => serializer.serialize_i64(*l),
            PrimitiveValue::Double(d) => serializer.serialize_f64(*d),
            PrimitiveValue::Date(d) => {
                let unix_mills = d
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("invalid date")
                    .as_millis();
                serializer.serialize_u128(unix_mills)
            }
            PrimitiveValue::Binary(b) => serializer.serialize_bytes(b.as_ref()),
            PrimitiveValue::String(s) => serializer.serialize_str(s),
        }
    }
}

impl serde::Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for next in self.iter() {
            seq.serialize_element(next)?;
        }
        seq.end()
    }
}

impl serde::Serialize for Map {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use ser::SerializeMap;

        let mut sem = serializer.serialize_map(Some(self.len()))?;

        // write a special key '$class' if exists.
        if let Some(class) = self.class() {
            sem.serialize_entry("$class", class)?;
        }

        for (k, v) in self.iter() {
            sem.serialize_entry(k, v)?;
        }
        sem.end()
    }
}

impl serde::Serialize for Object {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use ser::SerializeMap;

        let mut sem = serializer.serialize_map(Some(self.len()))?;

        // write a special key '$class' which represents `class` from Object.
        sem.serialize_entry("$class", self.class())?;

        for (k, v) in self.iter() {
            sem.serialize_entry(k, v)?;
        }

        sem.end()
    }
}

pub struct SerializeMap {
    map: Map,
    next_key: Option<PrimitiveValue>,
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let v = value.serialize(&mut Serializer::default())?;
        self.map.insert(PrimitiveValue::String(key.to_owned()), v);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let k = key.serialize(&mut Serializer::default())?;
        let pv = match k {
            Value::Primitive(pv) => pv,
            _ => {
                return Err(Error::Other(anyhow::anyhow!(
                    "map key must serialize to a primitive value"
                )));
            }
        };
        self.next_key = Some(pv);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let v = value.serialize(&mut Serializer::default())?;
        let k = self
            .next_key
            .take()
            .expect("serialize_value called before serialize_key");
        self.map.insert(k, v);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

pub struct SerializeVec {
    inner: Vec<Value>,
}

impl ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let v = value.serialize(&mut Serializer::default())?;
        self.inner.push(v);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(self.inner))
    }
}

impl ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub struct SerializeTupleVariant {
    vec: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let v = value.serialize(&mut Serializer::default())?;
        self.vec.push(v);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(List::from(self.vec)))
    }
}

pub struct SerializeStructVariant {
    map: Map,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let v = value.serialize(&mut Serializer::default())?;
        self.map.insert(PrimitiveValue::String(key.to_owned()), v);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

#[derive(Default)]
pub struct Serializer {}

impl Serializer {}

impl serde::Serializer for &mut Serializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as i32))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as i32))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as i32))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as i32))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as i64))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as i64))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut s = String::new();
        s.push(v);
        Ok(Value::from(s))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::from(v.to_owned()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let b = v.to_vec();
        Ok(Value::from(b))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeVec {
            inner: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant {
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap {
            map: Map::with_capacity(len.unwrap_or(0)),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeMap {
            map: Map::with_capacity(len),
            next_key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant {
            map: Map::with_capacity(len),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cachestr::Cachestr;
    use smallvec::smallvec;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    macro_rules! assert_ser {
        ($origin:expr, $expect:expr) => {{
            let mut ser = Serializer::default();
            let actual = $origin.serialize(&mut ser)?;
            assert_eq!($expect, actual);
        }};
    }

    #[test]
    fn test_serialize() -> anyhow::Result<()> {
        init();

        assert_ser!(true, Value::from(true));
        assert_ser!(false, Value::from(false));
        assert_ser!(123i32, Value::from(123i32));
        assert_ser!(123i64, Value::from(123i64));
        assert_ser!(3.14f64, Value::from(3.14f64));
        assert_ser!("foobar", Value::from("foobar".to_owned()));

        Ok(())
    }

    #[test]
    fn test_object2json() -> anyhow::Result<()> {
        init();

        use crate::value::Object;

        let o = Object::new(
            Cachestr::from("com.example.User"),
            smallvec![
                Cachestr::from("id"),
                Cachestr::from("name"),
                Cachestr::from("age"),
            ],
            vec![
                Value::from(123i64),
                Value::from("Foobar".to_owned()),
                Value::from(18i32),
            ],
        );

        let s = serde_json::to_string_pretty(&o)?;

        info!("to_json:\n{}", s);

        Ok(())
    }

    #[test]
    fn test_serialize_seq_and_tuple() -> anyhow::Result<()> {
        init();

        assert_ser!(
            vec![1i32, 2, 3],
            Value::from(vec![
                Value::from(1i32),
                Value::from(2i32),
                Value::from(3i32),
            ])
        );

        assert_ser!(
            (1i32, "two"),
            Value::from(vec![Value::from(1i32), Value::from("two".to_owned())])
        );

        Ok(())
    }

    #[test]
    fn test_serialize_map_and_struct() -> anyhow::Result<()> {
        init();

        use std::collections::BTreeMap;

        let mut m: BTreeMap<String, i32> = BTreeMap::new();
        m.insert("foo".to_owned(), 1);
        m.insert("bar".to_owned(), 2);

        let mut expect = Map::default();
        expect.insert(PrimitiveValue::String("foo".to_owned()), Value::from(1i32));
        expect.insert(PrimitiveValue::String("bar".to_owned()), Value::from(2i32));
        assert_ser!(m, Value::Map(expect));

        #[derive(Serialize)]
        struct Point {
            x: i32,
            y: i32,
        }

        let mut expect = Map::default();
        expect.insert(PrimitiveValue::String("x".to_owned()), Value::from(1i32));
        expect.insert(PrimitiveValue::String("y".to_owned()), Value::from(2i32));
        assert_ser!(Point { x: 1, y: 2 }, Value::Map(expect));

        Ok(())
    }

    #[test]
    fn test_serialize_enum_variants() -> anyhow::Result<()> {
        init();

        #[derive(Serialize)]
        enum E {
            Unit,
            Newtype(i32),
            Tuple(i32, i32),
            Struct { a: i32, b: i32 },
        }

        assert_ser!(E::Unit, Value::from("Unit".to_owned()));
        assert_ser!(E::Newtype(5), Value::from(5i32));
        assert_ser!(
            E::Tuple(1, 2),
            Value::from(vec![Value::from(1i32), Value::from(2i32)])
        );

        let mut expect = Map::default();
        expect.insert(PrimitiveValue::String("a".to_owned()), Value::from(1i32));
        expect.insert(PrimitiveValue::String("b".to_owned()), Value::from(2i32));
        assert_ser!(E::Struct { a: 1, b: 2 }, Value::Map(expect));

        Ok(())
    }
}
