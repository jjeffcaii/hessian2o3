use super::Encode;
use crate::{Classifier, Result, Sizeable};
use bytes::BufMut;
use std::collections::{HashSet, LinkedList};

const BC_LIST_DIRECT: u8 = 0x70;
const BC_LIST_DIRECT_UNTYPED: u8 = 0x78;
#[allow(dead_code)]
const BC_LIST_VARIABLE: u8 = 0x55;
const BC_LIST_FIXED: u8 = b'V';
#[allow(dead_code)]
const BC_LIST_VARIABLE_UNTYPED: u8 = 0x57;
const BC_LIST_FIXED_UNTYPED: u8 = 0x58;

const LIST_DIRECT_MAX: usize = 7;

impl<'a, T> Encode for &'a Vec<T>
where
    &'a T: Encode,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        let length = self.len();
        if length <= LIST_DIRECT_MAX {
            w.put_u8(BC_LIST_DIRECT_UNTYPED + length as u8);
        } else {
            w.put_u8(BC_LIST_FIXED_UNTYPED);
            (length as i32).encode(w)?;
        }
        for next in self {
            next.encode(w)?;
        }
        Ok(())
    }
}

impl<T> Classifier for LinkedList<T> {
    fn class_name() -> &'static str {
        "java.util.LinkedList"
    }
}

impl<T> Sizeable for LinkedList<T> {
    fn size(&self) -> usize {
        self.len()
    }
}

impl<T> Classifier for HashSet<T> {
    fn class_name() -> &'static str {
        "java.util.HashSet"
    }
}

impl<T> Sizeable for HashSet<T> {
    fn size(&self) -> usize {
        self.len()
    }
}

impl<'a, T, I> Encode for &'a T
where
    T: Classifier + Sizeable,
    &'a I: 'a + Encode,
    &'a T: IntoIterator<Item = &'a I>,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        let size = self.size();
        let class_name = T::class_name();
        if size <= LIST_DIRECT_MAX {
            w.put_u8(BC_LIST_DIRECT + size as u8);
            class_name.encode(w)?;
        } else {
            w.put_u8(BC_LIST_FIXED);
            class_name.encode(w)?;
            (size as i32).encode(w)?;
        }
        for item in self {
            item.encode(w)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_encode_hashset() -> Result<()> {
        init();

        let mut list: HashSet<i64> = Default::default();

        assert_eq!(
            "70116a6176612e7574696c2e48617368536574",
            list.to_hex_string()?
        );

        list.insert(111);
        list.insert(222);
        list.insert(333);

        assert!(!list.to_hex_string()?.is_empty());

        Ok(())
    }

    #[test]
    fn test_encode_linkedlist() -> Result<()> {
        init();
        {
            let mut list: LinkedList<i64> = Default::default();

            assert_eq!(
                "70146a6176612e7574696c2e4c696e6b65644c697374",
                list.to_hex_string()?
            );

            list.push_back(111);
            list.push_back(222);
            list.push_back(333);

            assert_eq!(
                "73146a6176612e7574696c2e4c696e6b65644c697374f86ff8def94d",
                list.to_hex_string()?
            );

            list.push_back(444);
            list.push_back(555);
            list.push_back(666);
            list.push_back(777);
            list.push_back(888);
            list.push_back(999);

            assert_eq!(
                "56146a6176612e7574696c2e4c696e6b65644c69737499f86ff8def94df9bcfa2bfa9afb09fb78fbe7",
                list.to_hex_string()?
            );
        }
        Ok(())
    }

    #[test]
    fn test_encode_vec() -> Result<()> {
        init();
        {
            let mut list: Vec<i64> = Default::default();
            assert_eq!("78", list.to_hex_string()?);

            list.push(111);
            list.push(222);
            list.push(333);
            assert_eq!("7bf86ff8def94d", list.to_hex_string()?);

            list.push(444);
            list.push(555);
            list.push(666);
            list.push(777);
            list.push(888);
            list.push(999);

            assert_eq!(
                "5899f86ff8def94df9bcfa2bfa9afb09fb78fbe7",
                list.to_hex_string()?
            );
        }

        {
            let list: Vec<String> = vec!["foo".to_string(), "bar".to_string(), "qux".to_string()];
            assert_eq!("7b03666f6f0362617203717578", list.to_hex_string()?);
        }

        Ok(())
    }
}
