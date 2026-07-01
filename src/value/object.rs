use crate::cachestr::Cachestr;
use crate::codec::Fields;
use crate::value::Value;
use std::fmt::{self, Display};
use std::ops;

#[derive(Debug, PartialEq)]
pub struct Object {
    class: Cachestr,
    fields: Fields,
    values: Vec<Value>,
}

impl Object {
    pub fn new(class: Cachestr, fields: Fields, values: Vec<Value>) -> Self {
        Self {
            class,
            fields,
            values,
        }
    }
}

impl ops::Index<&str> for Object {
    type Output = Value;

    fn index(&self, index: &str) -> &Self::Output {
        let n = self.fields.len();
        for i in 0..n {
            let field = &self.fields[i];

            if field.as_ref() == index {
                return &self.values[i];
            }
        }
        panic!("index '{}' not found", index);
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        f.write_str(self.class.as_ref())?;
        f.write_char('{')?;

        for i in 0..self.fields.len() {
            if i != 0 {
                f.write_str(", ")?;
            }

            let field = &self.fields[i];
            let value = &self.values[i];

            write!(f, "{:?}: ", field.as_ref())?;
            Display::fmt(value, f)?;
        }

        f.write_char('}')?;

        Ok(())
    }
}
