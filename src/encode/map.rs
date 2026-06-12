use super::{Encode, Kind, Typed};
use crate::{KindSupport, Result};
use bytes::BufMut;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

const BC_MAP: u8 = b'M';
const BC_MAP_UNTYPED: u8 = b'H';

const BC_END: u8 = b'Z';

impl<K, V> KindSupport for HashMap<K, V> {
    fn kind() -> Kind {
        Kind::Map
    }
}

impl<'a, K, V> Encode for &'a HashMap<K, V>
where
    &'a K: Encode + Eq + Hash,
    &'a V: Encode,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        w.put_u8(BC_MAP_UNTYPED);
        for (k, v) in self {
            k.encode(w)?;
            v.encode(w)?;
        }
        w.put_u8(BC_END);
        Ok(())
    }
}

impl<K, V> Typed for BTreeMap<K, V>
where
    K: Encode + Ord,
    V: Encode,
{
    fn type_name() -> &'static str {
        "java.util.TreeMap"
    }
}

impl<'a, K, V, T> Encode for &'a T
where
    K: 'a + Encode,
    V: 'a + Encode,
    T: Typed,
    &'a T: IntoIterator<Item = (K, V)>,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        w.put_u8(BC_MAP);
        T::type_name().encode(w)?;
        for (k, v) in self {
            k.encode(w)?;
            v.encode(w)?;
        }
        w.put_u8(BC_END);
        Ok(())
    }
}

// fn encode_typed_map<'a, W, K, V, I>(w: &mut W, type_name: &str, iter: I) -> Result<()>
// where
//     W: BufMut,
//     K: Encode + 'a,
//     V: Encode + 'a,
//     I: Iterator<Item = (&'a K, &'a V)>,
// {
//     w.put_u8(BC_MAP);
//     type_name.encode(w)?;
//     for (k, v) in iter {
//         k.encode(w)?;
//         v.encode(w)?;
//     }
//     w.put_u8(BC_END);
//     Ok(())
// }


#[cfg(test)]
mod tests{
    
    use super::*;
    
    fn init(){
        
    }

    #[test]
    fn test_encode_hashmap() {
        init();

        {
            let mut m: HashMap<i32, String> = Default::default();
            m.insert(1, "foo".into());
            m.insert(2, "bar".into());
            m.insert(3, "qux".into());

            assert_eq!("489103666f6f920362617293037175785a", encode(&m));
        }
    }

    #[test]
    fn test_encode_btreemap() {
        init();

        {
            let mut m: BTreeMap<i32, String> = Default::default();
            m.insert(1, "foo".into());
            m.insert(2, "bar".into());
            m.insert(3, "qux".into());

            assert_eq!(
                "4d116a6176612e7574696c2e547265654d61709103666f6f920362617293037175785a",
                encode(&m)
            );
        }
    }
}