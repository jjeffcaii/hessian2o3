use super::{Classifier, Encode};
use crate::Result;
use bytes::BufMut;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

const BC_MAP: u8 = b'M';
const BC_MAP_UNTYPED: u8 = b'H';

const BC_END: u8 = b'Z';

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

impl<K, V> Classifier for BTreeMap<K, V> {
    fn class_name() -> &'static str {
        "java.util.TreeMap"
    }
}

impl<'a, K, V> Encode for &'a BTreeMap<K, V>
where
    &'a K: 'a + Encode + Ord,
    &'a V: 'a + Encode,
{
    fn encode<W: BufMut>(self, w: &mut W) -> Result<()> {
        w.put_u8(BC_MAP);
        BTreeMap::<K, V>::class_name().encode(w)?;
        for (k, v) in self {
            k.encode(w)?;
            v.encode(w)?;
        }
        w.put_u8(BC_END);
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
    fn test_encode_hashmap() -> Result<()> {
        init();

        let mut m: HashMap<i32, String> = Default::default();
        m.insert(1, "foo".into());
        m.insert(2, "bar".into());
        m.insert(3, "qux".into());

        assert!(!m.to_hex_string()?.is_empty());

        Ok(())
    }

    #[test]
    fn test_encode_btreemap() -> Result<()> {
        init();

        let mut m: BTreeMap<i32, String> = Default::default();
        m.insert(1, "foo".into());
        m.insert(2, "bar".into());
        m.insert(3, "qux".into());

        assert_eq!(
            "4d116a6176612e7574696c2e547265654d61709103666f6f920362617293037175785a",
            m.to_hex_string()?
        );

        Ok(())
    }
}
