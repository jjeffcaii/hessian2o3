use crate::cachestr::Cachestr;
use indexmap::IndexMap;
use indexmap::map::Entry;
use smallvec::{SmallVec, smallvec};

mod decode;
mod encode;
mod tags;

pub use decode::*;
pub use encode::*;

pub type Fields = SmallVec<[Cachestr; 16]>;

#[derive(Debug, Default)]
pub struct Context {
    pub(crate) class_refs: IndexMap<Cachestr, Fields>, // (class,fields)
}

impl Context {
    // class: &str, fields: &[S]

    pub(crate) fn insert(&mut self, class: Cachestr, fields: Fields) {
        self.class_refs.insert(class, fields);
    }

    pub(crate) fn nth(&self, i: usize) -> Option<(Cachestr, Fields)> {
        self.class_refs
            .get_index(i)
            .map(|(class, fields)| (Clone::clone(class), fields.clone()))
    }

    pub(crate) fn put_class_define<C, F>(&mut self, class: C, fields: &[F]) -> Result<usize, usize>
    where
        C: AsRef<str>,
        F: AsRef<str>,
    {
        let class = Cachestr::from(class.as_ref());

        let ent = self.class_refs.entry(Clone::clone(&class));
        let index = ent.index();
        match ent {
            Entry::Occupied(o) => Err(index),
            Entry::Vacant(v) => {
                let mut newborn = smallvec![];
                for field in fields {
                    newborn.push(Cachestr::from(field.as_ref()));
                }

                v.insert(newborn);
                Ok(index)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context() {
        let mut ctx = Context::default();

        let i = ctx.put_class_define("com.example.Example", &["id", "name"]);

        assert_matches!(Ok::<usize, usize>(1), i);
    }
}
