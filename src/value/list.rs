use super::value::Value;
use crate::cachestr::Cachestr;
use std::fmt::{self, Debug, Display, Write};
use std::{ops, slice};

type IterImpl<'a> = slice::Iter<'a, Value>;

pub struct Iter<'a> {
    iter: IterImpl<'a>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(PartialEq, Default)]
pub struct List {
    class: Option<Cachestr>,
    l: Vec<Value>,
}

impl List {
    #[inline]
    pub fn new() -> Self {
        Self {
            class: None,
            l: Vec::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            class: None,
            l: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn set_class<C>(&mut self, class: C)
    where
        C: Into<Cachestr>,
    {
        self.class.replace(class.into());
    }

    #[inline]
    pub fn class(&self) -> Option<&str> {
        self.class.as_deref()
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

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.l.iter(),
        }
    }
}

impl Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.class {
            None => f.write_str("List")?,
            Some(class) => f.write_str(class)?,
        }
        f.write_char('[')?;

        let mut iter = self.l.iter();

        if let Some(first) = iter.next() {
            Display::fmt(&first, f)?;

            for next in iter {
                f.write_str(", ")?;
                Display::fmt(&next, f)?;
            }
        }

        f.write_char(']')?;

        Ok(())
    }
}

impl Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('[')?;

        let mut iter = self.l.iter();

        if let Some(first) = iter.next() {
            Display::fmt(&first, f)?;

            for next in iter {
                f.write_str(", ")?;
                Display::fmt(&next, f)?;
            }
        }

        f.write_char(']')?;

        Ok(())
    }
}

impl ops::Index<usize> for List {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.l[index]
    }
}

impl From<Vec<Value>> for List {
    fn from(value: Vec<Value>) -> Self {
        Self {
            class: None,
            l: value,
        }
    }
}
