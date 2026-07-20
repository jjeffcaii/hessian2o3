use crate::Result;
use crate::serde::from_reader;
use serde::de::DeserializeOwned;
use std::io;

// --- `deserialize!` dispatch ----------------------------------------------
//
// The target type `T` is only known from the binding, so — unlike
// `serialize!`, which dispatches on an argument value — autoref specialization
// can't help here: at method-resolution time `T` is still an inference
// variable, so a `T: HDeserialize` bound can't select an impl. Instead we use
// a *single* trait, `AutoDeserialize`, and rely on ordinary return-type
// inference (the same mechanism behind `Default::default()` or serde's
// `T::deserialize`): each type has exactly one impl, picked by the binding.
//
// Coverage is split without violating coherence:
//   - a blanket `impl<T: DeserializeOwned>` routes every serde type through
//     `serde::from_reader` (rule 2); `Value` is one such type and is rebuilt
//     through its own `Deserialize` impl (rule 3). For an explicit, direct
//     `Value` read there is also `get_value` / `get_value_from_slice`.
//   - the `HessianSerialize` derive emits a per-type impl routing through
//     `hessian_from_reader` (rule 1).
// The two never overlap because a derived Hessian type does not implement
// `DeserializeOwned` — Rust proves them disjoint. (Deriving both serde
// `Deserialize` and `HessianSerialize` on the same type is therefore a
// conflict; that ambiguity is intentional, since rules 1 and 2 would both
// apply.)

/// Decodes `Self` from a Hessian byte stream, picking the strategy from the
/// target type: types implementing `HDeserialize` (via the `HessianSerialize`
/// derive) go through `hessian_from_reader`; everything else goes through
/// `serde`. Driven by the [`deserialize!`](crate::deserialize) macro; usually
/// not called directly.
pub trait AutoDeserialize: Sized {
    fn auto_deserialize<R: io::Read>(reader: R) -> Result<Self>;
}

impl<T: DeserializeOwned> AutoDeserialize for T {
    #[inline]
    fn auto_deserialize<R: io::Read>(reader: R) -> Result<Self> {
        from_reader(reader)
    }
}

/// Deserialize a value of the inferred target type `T` from `reader`,
/// automatically choosing the decoding per [`AutoDeserialize`]:
/// - if `T` implements [`HDeserialize`](crate::HDeserialize) (the
///   `HessianSerialize` derive), through `hessian_from_reader`;
/// - otherwise (`T: Deserialize`, including [`Value`](crate::value::Value)),
///   through `serde`'s `from_reader`.
///
/// ```ignore
/// let user: User = deserialize!(reader)?;         // User: HDeserialize
/// let simple: SimpleUser = deserialize!(reader)?; // SimpleUser: Deserialize
/// let value: Value = deserialize!(reader)?;       // via serde
/// ```
#[macro_export]
macro_rules! deserialize {
    ($reader:expr $(,)?) => {
        $crate::AutoDeserialize::auto_deserialize($reader)
    };
}

#[cfg(test)]
mod tests {
    use crate::codec::Encoder;
    use crate::de::Deserializer;
    use crate::hessian::{HSerialize, Wrapper, hessian_from_reader, hessian_to_vec};
    use crate::serde::to_vec;
    use crate::value::Value;
    use crate::{AutoDeserialize, HDeserialize, Result};
    use serde::{Deserialize, Serialize};
    use std::io;

    // A Hessian-only type: implements `HSerialize`/`HDeserialize` but NOT serde
    // `Deserialize`. Hand-written (the derive emits `::hessian2::` paths that
    // don't resolve inside the defining crate) — including the `AutoDeserialize`
    // impl the derive would generate.
    #[derive(Debug, PartialEq)]
    struct User {
        id: i64,
        name: String,
        age: i32,
    }

    impl HSerialize for User {
        fn hessian_serialize<W: io::Write>(&self, enc: &mut Encoder<W>) -> Result<()> {
            enc.begin_object("com.example.User", &["id", "name", "age"])?;
            Wrapper(&self.id).hessian_serialize(enc)?;
            Wrapper(&self.name).hessian_serialize(enc)?;
            Wrapper(&self.age).hessian_serialize(enc)?;
            Ok(())
        }
    }

    impl HDeserialize for User {
        fn hessian_deserialize<R: io::Read>(de: &mut Deserializer<R>) -> Result<Self> {
            let mut obj = de.begin_object()?;
            let (mut id, mut name, mut age) = (None, None, None);
            while let Some(field) = obj.next_field() {
                match field.as_ref() {
                    "id" => id = Some(obj.value()?),
                    "name" => name = Some(obj.value()?),
                    "age" => age = Some(obj.value()?),
                    _ => obj.skip_value()?,
                }
            }
            Ok(User {
                id: id.unwrap(),
                name: name.unwrap(),
                age: age.unwrap(),
            })
        }
    }

    impl AutoDeserialize for User {
        fn auto_deserialize<R: io::Read>(mut reader: R) -> Result<Self> {
            hessian_from_reader(&mut reader)
        }
    }

    // A serde-only type: implements `Deserialize`/`Serialize` but NOT `HDeserialize`.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct SimpleUser {
        id: i64,
        name: String,
        age: i32,
    }

    #[test]
    fn deserialize_dispatches_to_hessian() -> anyhow::Result<()> {
        let user = User {
            id: 7,
            name: "alice".to_string(),
            age: 30,
        };
        let bytes = hessian_to_vec(&user)?;

        let got: User = deserialize!(bytes.as_slice())?;
        assert_eq!(user, got);
        Ok(())
    }

    #[test]
    fn deserialize_dispatches_to_serde() -> anyhow::Result<()> {
        let simple = SimpleUser {
            id: 9,
            name: "bob".to_string(),
            age: 42,
        };
        let bytes = to_vec(&simple)?;

        let got: SimpleUser = deserialize!(bytes.as_slice())?;
        assert_eq!(simple, got);
        Ok(())
    }

    #[test]
    fn deserialize_dispatches_value_via_serde() -> anyhow::Result<()> {
        let simple = SimpleUser {
            id: 1,
            name: "carol".to_string(),
            age: 55,
        };
        let bytes = to_vec(&simple)?;

        let got: Value = deserialize!(bytes.as_slice())?;
        let back: SimpleUser = crate::serde::from_value(got)?;
        assert_eq!(simple, back);
        Ok(())
    }
}
