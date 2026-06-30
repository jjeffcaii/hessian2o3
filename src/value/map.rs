use super::{PrimitiveValue, Value};
use core::iter::FusedIterator;
use std::borrow::Borrow;
use std::collections::hash_map::{self, HashMap};
use std::fmt::{self, Debug, Display, Write};
use std::hash::Hash;
use std::mem;
use std::ops;

pub type Key = PrimitiveValue;

type KeysImpl<'a> = hash_map::Keys<'a, Key, Value>;

#[derive(Clone, Debug)]
pub struct Keys<'a> {
    iter: KeysImpl<'a>,
}

type ValuesImpl<'a> = hash_map::Values<'a, Key, Value>;

#[derive(Clone, Debug)]
pub struct Values<'a> {
    iter: ValuesImpl<'a>,
}

type MapImpl = HashMap<Key, Value>;

#[derive(PartialEq)]
pub struct Map {
    map: MapImpl,
}

impl Map {
    #[inline]
    pub fn new() -> Self {
        Map {
            map: MapImpl::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Map {
            map: MapImpl::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Key: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.get(key)
    }

    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Key: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.contains_key(key)
    }

    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
    where
        Key: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.get_mut(key)
    }

    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&Key, &Value)>
    where
        Key: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.get_key_value(key)
    }

    #[inline]
    pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        self.map.insert(key, value)
    }

    #[inline]
    pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
    where
        Key: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.remove(key)
    }

    #[inline]
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(Key, Value)>
    where
        Key: Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.remove_entry(key)
    }

    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        self.map
            .extend(mem::replace(&mut other.map, MapImpl::default()))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    pub fn keys(&self) -> Keys<'_> {
        Keys {
            iter: self.map.keys(),
        }
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.map.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{")?;

        let mut iter = self.iter();

        if let Some((k, v)) = iter.next() {
            Display::fmt(k, f)?;
            f.write_str(": ")?;
            Display::fmt(v, f)?;

            for (k, v) in iter {
                f.write_str(", ")?;

                Display::fmt(k, f)?;
                f.write_str(": ")?;
                Display::fmt(v, f)?;
            }
        }

        f.write_str("}")?;
        Ok(())
    }
}

type IterImpl<'a> = hash_map::Iter<'a, Key, Value>;

type IterMutImpl<'a> = hash_map::IterMut<'a, Key, Value>;

#[derive(Debug)]
pub struct Iter<'a> {
    iter: IterImpl<'a>,
}

#[derive(Debug)]
pub struct IterMut<'a> {
    iter: IterMutImpl<'a>,
}

impl Default for Map {
    fn default() -> Self {
        Map {
            map: MapImpl::default(),
        }
    }
}

impl<Q> ops::Index<&Q> for Map
where
    Key: Borrow<Q>,
    Q: ?Sized + Eq + Hash,
{
    type Output = Value;

    fn index(&self, index: &Q) -> &Self::Output {
        self.map.index(index)
    }
}

impl<Q> ops::IndexMut<&Q> for Map
where
    Key: Borrow<Q>,
    Q: ?Sized + Eq + Hash,
{
    fn index_mut(&mut self, index: &Q) -> &mut Self::Output {
        self.map.get_mut(index).expect("no entry found for key")
    }
}

impl Debug for Map {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Map")?;
        self.map.fmt(f)
    }
}

macro_rules! delegate_iterator {
    (($name:ident $($generics:tt)*) => $item:ty) => {
        impl $($generics)* Iterator for $name $($generics)* {
            type Item = $item;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        // impl $($generics)* DoubleEndedIterator for $name $($generics)* {
        //     #[inline]
        //     fn next_back(&mut self) -> Option<Self::Item> {
        //         self.iter.next_back()
        //     }
        // }

        impl $($generics)* ExactSizeIterator for $name $($generics)* {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl $($generics)* FusedIterator for $name $($generics)* {}
    }
}

delegate_iterator!((Iter<'a>) => (&'a Key, &'a Value));

delegate_iterator!((IterMut<'a>) => (&'a Key, &'a mut Value));

delegate_iterator!((Keys<'a>) => &'a Key);

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
