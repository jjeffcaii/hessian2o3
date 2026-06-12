use super::{Encode, KindSupport};
use crate::{Kind, Result, Typed};
use bytes::BufMut;
use std::collections::LinkedList;

const BC_LIST_DIRECT: u8 = 0x70;
const BC_LIST_DIRECT_UNTYPED: u8 = 0x78;
const BC_LIST_VARIABLE: u8 = 0x55;
const BC_LIST_FIXED: u8 = b'V';
const BC_LIST_VARIABLE_UNTYPED: u8 = 0x57;
const BC_LIST_FIXED_UNTYPED: u8 = 0x58;

const LIST_DIRECT_MAX: usize = 7;

impl<T> KindSupport for Vec<T> {
    fn kind() -> Kind {
        Kind::List
    }
}

impl<'a, T> Encode for &'a Vec<T>
where
    &'a T: Encode,
    T: KindSupport,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        let length = self.len();

        if length <= LIST_DIRECT_MAX {
            let _kind = T::kind();
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

impl<T> Typed for LinkedList<T> {
    fn type_name() -> &'static str {
        "java.util.LinkedList"
    }
}

impl<'a, T, I> Encode for &'a T
where
    T: Typed,
    &'a I: 'a + Encode,
    &'a T: IntoIterator<Item = &'a I>,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        todo!()
    }
}

#[inline]
fn encode_typed_list<'a, W, T, I>(w: &mut W, type_name: &str, iter: I) -> Result<()>
where
    W: BufMut,
    &'a T: Encode,
    T: 'a,
    I: ExactSizeIterator<Item = &'a T>,
{
    let length = iter.len();
    if length <= LIST_DIRECT_MAX {
        w.put_u8(BC_LIST_DIRECT + length as u8);
        type_name.encode(w)?;
    } else {
        w.put_u8(BC_LIST_FIXED);
        type_name.encode(w)?;
        (length as i32).encode(w)?;
    }
    for item in iter {
        item.encode(w)?;
    }
    Ok(())
}

impl<T> KindSupport for LinkedList<T>
where
    T: KindSupport,
{
    fn kind() -> Kind {
        Kind::List
    }
}

// impl<T> Encode for &LinkedList<T>
// where
//     T: Encode,
// {
//     fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
//         encode_typed_list(w, "java.util.LinkedList", self)
//     }
// }
//
// impl<T> KindSupport for HashSet<T> {
//     fn kind() -> Kind {
//         Kind::List
//     }
// }
//
// impl<'a, T> Encode for &'a HashSet<T>
// where
//     T: Eq + Hash,
//     &'a T: Encode,
// {
//     fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
//         encode_typed_list(w, "java.util.HashSet", self.iter())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    // #[test]
    // fn test_encode_hashset() {
    //     init();
    //
    //     let mut list: HashSet<i64> = Default::default();
    //
    //     // assert_eq!("70116a6176612e7574696c2e48617368536574", encode(&list));
    //
    //     list.insert(111);
    //     list.insert(222);
    //     list.insert(333);
    //
    //     assert_eq!(
    //         "73116a6176612e7574696c2e48617368536574f94df8def86f",
    //         encode(&list)
    //     );
    //     // list.insert(444);
    //     // list.insert(555);
    //     // list.insert(666);
    //     // list.insert(777);
    //     // list.insert(888);
    //     // list.insert(999);
    //     //
    //     // assert_eq!(
    //     //     "56116a6176612e7574696c2e4861736853657499fbe7fb78fb09fa9afa2bf9bcf94df8def86f",
    //     //     encode(&list)
    //     // );
    // }

    // #[test]
    // fn test_encode_linkedlist() {
    //     {
    //         let mut list: LinkedList<i64> = Default::default();
    //
    //         assert_eq!(
    //             "70146a6176612e7574696c2e4c696e6b65644c697374",
    //             encode(&list)
    //         );
    //
    //         list.push_back(111);
    //         list.push_back(222);
    //         list.push_back(333);
    //
    //         assert_eq!(
    //             "73146a6176612e7574696c2e4c696e6b65644c697374f86ff8def94d",
    //             encode(&list)
    //         );
    //         list.push_back(444);
    //         list.push_back(555);
    //         list.push_back(666);
    //         list.push_back(777);
    //         list.push_back(888);
    //         list.push_back(999);
    //
    //         assert_eq!(
    //             "56146a6176612e7574696c2e4c696e6b65644c69737499f86ff8def94df9bcfa2bfa9afb09fb78fbe7",
    //             encode(&list)
    //         );
    //     }
    // }

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
