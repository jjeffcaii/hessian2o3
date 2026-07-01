use super::map::{Key, Map};
use super::{PrimitiveValue, Value};
use core::fmt;
use std::fmt::Display;
use std::ops;

pub trait Index: private::Sealed {
    #[doc(hidden)]
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;

    #[doc(hidden)]
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value>;

    #[doc(hidden)]
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value;
}

// Prevent users from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<T> Sealed for &T where T: ?Sized + Sealed {}
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::List(vec) => vec.get(*self),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::List(vec) => vec.get_mut(*self),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        match v {
            Value::List(vec) => {
                let len = vec.len();
                vec.get_mut(*self).unwrap_or_else(|| {
                    core::panic!(
                        "cannot access index {} of Hessian list of length {}",
                        self,
                        len
                    )
                })
            }
            _ => core::panic!("cannot access index {} of Hessian {}", self, Type(v)),
        }
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v {
            Value::Map(map) => map.get(&Key::from(self.to_string())),
            _ => None,
        }
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        match v {
            Value::Map(map) => map.get_mut(&Key::from(self.to_string())),
            _ => None,
        }
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        if let Value::Null = v {
            *v = Value::Map(Map::new());
        }
        match v {
            Value::Map(map) => map.entry(self.to_owned()).or_insert(Value::Null),
            _ => core::panic!("cannot access key {:?} in Hessian {}", self, Type(v)),
        }
    }
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self[..].index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        self[..].index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        self[..].index_or_insert(v)
    }
}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }
    fn index_into_mut<'v>(&self, v: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(v)
    }
    fn index_or_insert<'v>(&self, v: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(v)
    }
}

struct Type<'a>(&'a Value);

impl<'a> Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            Value::Null => f.write_str("null"),
            Value::Primitive(ref v) => match v {
                PrimitiveValue::Bool(_) => f.write_str("boolean"),
                PrimitiveValue::Int(_) => f.write_str("int"),
                PrimitiveValue::Long(_) => f.write_str("long"),
                PrimitiveValue::Double(_) => f.write_str("double"),
                PrimitiveValue::Date(_) => f.write_str("date"),
                PrimitiveValue::Binary(_) => f.write_str("binary"),
                PrimitiveValue::String(_) => f.write_str("string"),
            },
            Value::List(_) => f.write_str("list"),
            Value::Map(_) => f.write_str("map"),
        }
    }
}

impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Value;

    fn index(&self, index: I) -> &Value {
        static NULL: Value = Value::Null;
        index.index_into(self).unwrap_or(&NULL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Map;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_index() {
        init();

        let mut m = Map::new();
        m.insert("foo".to_string().into(), 1i32.into());
        m.insert("bar".to_string().into(), 2i32.into());
        m.insert("qux".to_string().into(), 3i32.into());
        let v = Value::from(m);

        info!("foo: {}", v["foo"]);
    }
}
