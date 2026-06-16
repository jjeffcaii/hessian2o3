use crate::encode;
use crate::error::Error;
use serde::ser::{self, Serialize};
use std::io;

pub struct DefaultFormatter;

impl Formatter for DefaultFormatter {}

pub trait Formatter {
    fn put_unit<W>(&mut self, w: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_null(w)
    }

    fn put_null<W>(&mut self, w: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_null(w)
    }

    fn put_bool<W>(&mut self, w: &mut W, b: bool) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_bool(w, b)
    }

    fn put_i8<W>(&mut self, w: &mut W, i: i8) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i32(w, i as i32)
    }

    fn put_i16<W>(&mut self, w: &mut W, i: i16) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i32(w, i as i32)
    }

    fn put_i32<W>(&mut self, w: &mut W, i: i32) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i32(w, i)
    }

    fn put_i64<W>(&mut self, w: &mut W, i: i64) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i64(w, i)
    }

    fn put_u8<W>(&mut self, w: &mut W, i: u8) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i32(w, i as i32)
    }

    fn put_u16<W>(&mut self, w: &mut W, i: u16) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i32(w, i as i32)
    }

    fn put_u32<W>(&mut self, w: &mut W, i: u32) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i64(w, i as i64)
    }

    fn put_u64<W>(&mut self, w: &mut W, i: u64) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_i64(w, i as i64)
    }

    fn put_f32<W>(&mut self, w: &mut W, f: f32) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_f64(w, f as f64)
    }

    fn put_f64<W>(&mut self, w: &mut W, v: f64) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_f64(w, v)
    }

    fn put_str<W>(&mut self, w: &mut W, s: &str) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_str(w, s)
    }

    fn put_bytes<W>(&mut self, w: &mut W, b: &[u8]) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::put_bytes(w, b)
    }

    fn begin_list<W>(&mut self, w: &mut W, n: Option<usize>) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::begin_list(w, None, n.unwrap_or(0))
    }

    fn begin_typed_list<W>(
        &mut self,
        w: &mut W,
        class: &str,
        length: Option<usize>,
    ) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::begin_list(w, Some(class), length.unwrap_or(0))
    }

    fn begin_map<W>(&mut self, w: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::begin_map(w, None)
    }

    fn begin_typed_map<W>(&mut self, w: &mut W, class: &str) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::begin_map(w, Some(class))
    }

    fn end_compound<W>(&mut self, w: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        encode::end_map(w)
    }
}

pub struct TypedList<L> {
    class: &'static str,
    inner: L,
}

impl<L> TypedList<L> {
    pub fn new(class: &'static str, inner: L) -> Self {
        Self { class, inner }
    }
}

impl<L: Serialize> Serialize for TypedList<L> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(self.class, &self.inner)
    }
}

pub struct TypedMap<M> {
    class: &'static str,
    inner: M,
}

impl<M> TypedMap<M> {
    pub fn new(class: &'static str, inner: M) -> Self {
        Self { class, inner }
    }
}

impl<M: Serialize> Serialize for TypedMap<M> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(self.class, &self.inner)
    }
}

pub struct Serializer<W, F> {
    writer: W,
    formatter: F,
    pending_class: Option<&'static str>,
}

impl<W: io::Write, F: Formatter> Serializer<W, F> {
    pub fn new(writer: W, formatter: F) -> Self {
        Self {
            writer,
            formatter,
            pending_class: None,
        }
    }
}

pub enum Compound<'a, W: 'a, F: 'a> {
    Seq { ser: &'a mut Serializer<W, F> },
    Map { ser: &'a mut Serializer<W, F> },
}

impl<'a, W, F> ser::SerializeSeq for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        match self {
            Compound::Seq { ser } => value.serialize(&mut **ser),
            Compound::Map { ser } => value.serialize(&mut **ser),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if let Compound::Map { ser } = self {
            ser.formatter
                .end_compound(&mut ser.writer)
                .map_err(Error::io)?;
        }

        Ok(())
    }
}

impl<'a, W, F> ser::SerializeTuple for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeTupleStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeTupleVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeMap for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        let Compound::Map { ser } = self else {
            unreachable!()
        };
        key.serialize(&mut **ser)
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let Compound::Map { ser } = self else {
            unreachable!()
        };
        value.serialize(&mut **ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Compound::Map { ser } = self else {
            unreachable!()
        };
        ser.formatter
            .end_compound(&mut ser.writer)
            .map_err(Error::io)
    }
}

impl<'a, W, F> ser::SerializeStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let Compound::Map { ser } = self else {
            unreachable!()
        };
        ser.formatter
            .put_str(&mut ser.writer, key)
            .map_err(Error::io)?;
        value.serialize(&mut **ser)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

impl<'a, W, F> ser::SerializeStructVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

impl<'a, W, F> ser::Serializer for &'a mut Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a, W, F>;
    type SerializeTuple = Compound<'a, W, F>;
    type SerializeTupleStruct = Compound<'a, W, F>;
    type SerializeTupleVariant = Compound<'a, W, F>;
    type SerializeMap = Compound<'a, W, F>;
    type SerializeStruct = Compound<'a, W, F>;
    type SerializeStructVariant = Compound<'a, W, F>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_bool(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_i8(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_i16(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_i32(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_i64(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_u8(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_u16(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_u32(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_u64(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_f32(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_f64(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0u8; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_str(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.formatter
            .put_bytes(&mut self.writer, v)
            .map_err(Error::io)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.formatter.put_null(&mut self.writer).map_err(Error::io)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.formatter.put_unit(&mut self.writer).map_err(Error::io)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.pending_class = Some(name);
        let result = value.serialize(&mut *self);
        self.pending_class = None;
        result
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_seq(self, size: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        match self.pending_class.take() {
            None => self.formatter.begin_list(&mut self.writer, size),
            Some(class) => self
                .formatter
                .begin_typed_list(&mut self.writer, class, size),
        }
        .map_err(Error::io)?;

        Ok(Compound::Seq { ser: self })
    }

    fn serialize_tuple(self, size: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.formatter
            .begin_list(&mut self.writer, Some(size))
            .map_err(Error::io)?;
        Ok(Compound::Seq { ser: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        size: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.formatter
            .begin_list(&mut self.writer, Some(size))
            .map_err(Error::io)?;
        Ok(Compound::Seq { ser: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        size: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.formatter
            .begin_list(&mut self.writer, Some(size))
            .map_err(Error::io)?;
        Ok(Compound::Seq { ser: self })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        if let Some(class) = self.pending_class.take() {
            self.formatter
                .begin_typed_map(&mut self.writer, class)
                .map_err(Error::io)?;
        } else {
            self.formatter
                .begin_map(&mut self.writer)
                .map_err(Error::io)?;
        }
        Ok(Compound::Map { ser: self })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.formatter
            .begin_map(&mut self.writer)
            .map_err(Error::io)?;
        Ok(Compound::Map { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.formatter
            .begin_map(&mut self.writer)
            .map_err(Error::io)?;
        Ok(Compound::Map { ser: self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::collections::BTreeMap;

    fn to_hex<T>(value: T) -> String
    where
        T: Serialize,
    {
        let mut b = vec![];
        let mut ser = Serializer::new(&mut b, DefaultFormatter);
        value.serialize(&mut ser).unwrap();
        hex::encode(b)
    }

    #[test]
    fn test_bool() {
        assert_eq!("54", to_hex(true));
        assert_eq!("46", to_hex(false));
    }

    #[test]
    fn test_integers() {
        // i32 compact encoding (matches encode::primitive tests)
        assert_eq!("90", to_hex(0i32));
        assert_eq!("bf", to_hex(47i32));
        assert_eq!("80", to_hex(-16i32));
        assert_eq!("d586a0", to_hex(100_000i32)); // SHORT range: -262144..=262143

        // i8 / i16 are widened to i32
        assert_eq!("90", to_hex(0i8));
        assert_eq!("91", to_hex(1i16));

        // u8 / u16 widened to i32, u32 / u64 to i64
        assert_eq!("9f", to_hex(15u8));
        assert_eq!("e1", to_hex(1u64));
    }

    #[test]
    fn test_i64() {
        assert_eq!("e0", to_hex(0i64));
        assert_eq!("ef", to_hex(15i64));
        assert_eq!("d8", to_hex(-8i64));
        assert_eq!("4c0000000080000000", to_hex(2_147_483_648i64));
    }

    #[test]
    fn test_floats() {
        assert_eq!("5b", to_hex(0.0f64));
        assert_eq!("5c", to_hex(1.0f64));
        assert_eq!("5d80", to_hex(-128.0f64));
        // f32 is widened to f64
        assert_eq!("5b", to_hex(0.0f32));
        assert_eq!("5c", to_hex(1.0f32));
    }

    #[test]
    fn test_str_and_char() {
        assert_eq!("00", to_hex(""));
        assert_eq!("0568656c6c6f", to_hex("hello"));
        // char serialized as a one-character string
        assert_eq!("0141", to_hex('A'));
    }

    #[test]
    fn test_option() {
        assert_eq!("4e", to_hex(None::<i32>));
        assert_eq!("0568656c6c6f", to_hex(Some("hello")));
        assert_eq!("91", to_hex(Some(1i32)));
    }

    #[test]
    fn test_sequence() {
        // empty
        assert_eq!("78", to_hex(Vec::<i32>::new()));

        // simple
        assert_eq!("7b919293", to_hex(vec![1i32, 2, 3]));

        // nested
        assert_eq!(
            "7b799179927993",
            to_hex(vec![vec![1i32], vec![2i32], vec![3i32]])
        );
    }

    #[test]
    fn test_untyped_map() {
        let mut m: BTreeMap<i32, &str> = Default::default();
        m.insert(1, "foo");
        m.insert(2, "bar");
        m.insert(3, "qux");

        assert_eq!("489103666f6f920362617293037175785a", to_hex(&m));
    }

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct Point {
            x: i32,
            y: i32,
        }
        // H + "x"(0178) + 1(91) + "y"(0179) + 2(92) + Z
        assert_eq!("480178910179925a", to_hex(Point { x: 1, y: 2 }));
    }

    #[test]
    fn test_unit_variant() {
        #[derive(Serialize)]
        enum Direction {
            North,
            South,
        }
        // unit variant → string of variant name
        assert_eq!("054e6f727468", to_hex(Direction::North)); // "North"
        assert_eq!("05536f757468", to_hex(Direction::South)); // "South"
    }

    #[test]
    fn test_newtype_struct() {
        #[derive(Serialize)]
        struct Meters(i32);
        // transparent: just encodes the inner value
        assert_eq!("91", to_hex(Meters(1)));
    }

    #[test]
    fn test_tuple() {
        // `() -> null` by default
        assert_eq!("4e", to_hex(()));

        // tuple → variable untyped list
        assert_eq!("7c91e25d030134", to_hex((1i32, 2i64, 3f64, "4")));
    }

    #[test]
    fn test_typed_list() {
        let class = "java.util.LinkedList";

        assert_eq!(
            "70146a6176612e7574696c2e4c696e6b65644c697374",
            to_hex(TypedList::new(class, Vec::<i32>::new()))
        );
        assert_eq!(
            "56146a6176612e7574696c2e4c696e6b65644c69737499919293949596979899",
            to_hex(TypedList::new(class, vec![1i32, 2, 3, 4, 5, 6, 7, 8, 9]))
        );
    }

    #[test]
    fn test_typed_map() {
        let mut m: BTreeMap<i32, &str> = BTreeMap::new();
        m.insert(1, "foo");
        m.insert(2, "bar");
        m.insert(3, "qux");

        assert_eq!(
            "4d116a6176612e7574696c2e547265654d61709103666f6f920362617293037175785a",
            to_hex(TypedMap::new("java.util.TreeMap", &m))
        );
    }
}
