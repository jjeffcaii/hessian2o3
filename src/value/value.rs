use super::map::Map;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::time;
use std::time::SystemTime;

#[derive(Debug, PartialEq, Default)]
pub struct List {
    l: Vec<Value>,
}

impl List {
    #[inline]
    pub fn new() -> Self {
        Self { l: Vec::new() }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            l: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn get(&self, i: usize) -> Option<&Value> {
        self.l.get(i)
    }

    #[inline]
    pub fn get_mut(&mut self, i: usize) -> Option<&mut Value> {
        self.l.get_mut(i)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.l.len()
    }
}

impl Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;

        let mut iter = self.l.iter();

        if let Some(first) = iter.next() {
            Display::fmt(&first, f)?;
        }

        for next in iter {
            f.write_str(", ")?;
            Display::fmt(&next, f)?;
        }

        f.write_str("]")?;

        Ok(())
    }
}

pub enum PrimitiveValue {
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    Date(SystemTime),
    Binary(Vec<u8>),
    String(String),
}

impl Display for PrimitiveValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveValue::Bool(b) => write!(f, "{}", b),
            PrimitiveValue::Int(i) => write!(f, "{}", i),
            PrimitiveValue::Long(l) => write!(f, "{}", l),
            PrimitiveValue::Double(d) => write!(f, "{}", d),
            PrimitiveValue::Date(d) => {
                let unix_mills = d
                    .duration_since(time::UNIX_EPOCH)
                    .expect("time went backwards");

                write!(f, "{}", unix_mills.as_millis())
            }
            PrimitiveValue::Binary(b) => {
                let b64 = encode_base64(b);
                write!(f, "{:?}", &b64)
            }
            PrimitiveValue::String(s) => write!(f, "{:?}", s),
        }
    }
}

impl Debug for PrimitiveValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveValue::Bool(b) => write!(f, "Bool({})", b),
            PrimitiveValue::Int(i) => write!(f, "Int({})", i),
            PrimitiveValue::Long(l) => write!(f, "Long({})", l),
            PrimitiveValue::Double(d) => write!(f, "Double({})", d),
            PrimitiveValue::Date(d) => write!(f, "Date({:?})", d),
            PrimitiveValue::Binary(b) => {
                let b64 = encode_base64(b);
                write!(f, "Binary({:?})", b64)
            }
            PrimitiveValue::String(s) => write!(f, "String({:?})", s),
        }
    }
}

impl PartialEq for PrimitiveValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Long(a), Self::Long(b)) => a == b,
            (Self::Double(a), Self::Double(b)) => a.to_ne_bytes() == b.to_ne_bytes(),
            (Self::Date(a), Self::Date(b)) => a == b,
            (Self::Binary(a), Self::Binary(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for PrimitiveValue {}

impl Hash for PrimitiveValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PrimitiveValue::Bool(v) => v.hash(state),
            PrimitiveValue::Int(v) => v.hash(state),
            PrimitiveValue::Long(v) => v.hash(state),
            PrimitiveValue::Double(v) => {
                let v64 = {
                    let b = v.to_ne_bytes();
                    i64::from_be_bytes(b)
                };
                v64.hash(state)
            }
            PrimitiveValue::Date(v) => v.hash(state),
            PrimitiveValue::Binary(v) => v.hash(state),
            PrimitiveValue::String(v) => v.hash(state),
        }
    }
}

impl From<bool> for PrimitiveValue {
    fn from(b: bool) -> Self {
        PrimitiveValue::Bool(b)
    }
}

impl From<i32> for PrimitiveValue {
    fn from(i: i32) -> Self {
        PrimitiveValue::Int(i)
    }
}

impl From<i64> for PrimitiveValue {
    fn from(i: i64) -> Self {
        PrimitiveValue::Long(i)
    }
}

impl From<f64> for PrimitiveValue {
    fn from(f: f64) -> Self {
        PrimitiveValue::Double(f)
    }
}

impl From<SystemTime> for PrimitiveValue {
    fn from(t: SystemTime) -> Self {
        PrimitiveValue::Date(t)
    }
}

impl From<Vec<u8>> for PrimitiveValue {
    fn from(v: Vec<u8>) -> Self {
        PrimitiveValue::Binary(v)
    }
}

impl From<String> for PrimitiveValue {
    fn from(s: String) -> Self {
        PrimitiveValue::String(s)
    }
}

#[derive(PartialEq)]
pub enum Value {
    Null,
    Primitive(PrimitiveValue),
    List(List),
    Map(Map),
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Primitive(v) => Display::fmt(v, f),
            Value::List(v) => Display::fmt(v, f),
            Value::Map(v) => Display::fmt(v, f),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("Null"),
            Value::Primitive(p) => Debug::fmt(p, f),
            Value::List(l) => Debug::fmt(l, f),
            Value::Map(m) => Debug::fmt(m, f),
        }
    }
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl From<()> for Value {
    fn from(value: ()) -> Self {
        Self::Null
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::Primitive(PrimitiveValue::Binary(value))
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::List(List { l: value })
    }
}

impl From<PrimitiveValue> for Value {
    fn from(value: PrimitiveValue) -> Self {
        Self::Primitive(value)
    }
}

impl From<Map> for Value {
    fn from(value: Map) -> Self {
        Self::Map(value)
    }
}

impl From<List> for Value {
    fn from(value: List) -> Self {
        Self::List(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Primitive(PrimitiveValue::Int(value))
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Primitive(PrimitiveValue::Long(value))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Primitive(PrimitiveValue::Double(value))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Primitive(PrimitiveValue::Bool(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Primitive(PrimitiveValue::String(value))
    }
}

impl From<SystemTime> for Value {
    fn from(value: SystemTime) -> Self {
        Self::Primitive(PrimitiveValue::Date(value))
    }
}

fn encode_base64(b: &[u8]) -> String {
    use base64::{Engine as _, alphabet, engine};

    const G: engine::GeneralPurpose =
        engine::GeneralPurpose::new(&alphabet::STANDARD, engine::general_purpose::PAD);

    let mut buf = String::with_capacity(b.len() * 3 / 4);
    G.encode_string(b, &mut buf);
    buf
}
