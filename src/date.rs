//! Support for `#[serde(with = "hessian2::date")]` on an `i64` field holding
//! Unix milliseconds, so it round-trips through the Hessian *date* wire type
//! (tag `0x4a`/`0x4b`) instead of a plain long.
//!
//! Deserializing doesn't need any special handling: the decoder already
//! accepts both a Hessian long and a Hessian date wherever an `i64` is
//! expected. Serializing does need help, since `serde::Serializer` has no
//! `serialize_date` hook — a private newtype-struct marker name signals the
//! date encoding to `hessian2`'s own [`Serializer`](crate::serde), and is a
//! no-op passthrough for every other `serde::Serializer`.

pub(crate) const MARKER: &str = "$hessian::date";

pub fn serialize<S>(millis: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_newtype_struct(MARKER, millis)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::{from_slice, to_vec};
    use serde::{Deserialize, Serialize};

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Employee {
        name: String,
        #[serde(with = "crate::date")]
        created_at: i64,
    }

    fn to_hex(millis: i64) -> String {
        let mut buf = vec![];
        let mut ser = crate::ser::Serializer::new(&mut buf, crate::ser::DefaultFormatter);
        serialize(&millis, &mut ser).unwrap();
        hex::encode(buf)
    }

    #[test]
    fn test_serialize_uses_hessian_date_tag() -> anyhow::Result<()> {
        init();

        use chrono::DateTime;
        use std::time::SystemTime;

        let millis_since_epoch = |rfc3339: &str| -> anyhow::Result<i64> {
            let datetime = DateTime::parse_from_rfc3339(rfc3339)?;
            let system_time: SystemTime = SystemTime::from(datetime);
            Ok(system_time
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis() as i64)
        };

        // millis not aligned to a whole minute -> full-precision date tag
        // (0x4a), matching `Encoder::put_date`'s own encoding exactly.
        let millis = millis_since_epoch("2026-06-10T15:16:17+08:00")?;
        assert_eq!("4a0000019eb06395e8", to_hex(millis));

        // minute-aligned millis -> compact date-minute form (0x4b), not the
        // long tag (0x4c/short forms) a plain i64 would use.
        let millis = millis_since_epoch("2026-06-10T15:16:00+08:00")?;
        assert_eq!("4b01c4f374", to_hex(millis));

        Ok(())
    }

    #[test]
    fn test_roundtrip_through_struct_field() -> anyhow::Result<()> {
        init();

        let employee = Employee {
            name: "Alice".to_owned(),
            created_at: 1_749_540_617_123,
        };

        let bytes = to_vec(&employee)?;
        // 0x4a marks the hessian date tag; a plain i64 would never emit it.
        assert!(hex::encode(&bytes).contains("4a"));

        let back: Employee = from_slice(&bytes)?;
        assert_eq!(employee, back);

        Ok(())
    }

    #[test]
    fn test_deserialize_accepts_plain_long_too() -> anyhow::Result<()> {
        init();

        // a value written as a plain hessian long must still deserialize
        // fine through the `with = "hessian2::date"` field, since decoding
        // accepts both wire flavors.
        use crate::codec::Encoder;

        let bytes = {
            let mut buf = vec![];
            let mut enc = Encoder::new(&mut buf);
            enc.begin_object("com.example.Employee", &["name", "created_at"])?;
            enc.put_str("Carol")?;
            enc.put_i64(1_700_000_000_000)?;
            buf
        };

        let employee: Employee = from_slice(&bytes)?;
        assert_eq!(
            Employee {
                name: "Carol".to_owned(),
                created_at: 1_700_000_000_000,
            },
            employee
        );

        Ok(())
    }
}
