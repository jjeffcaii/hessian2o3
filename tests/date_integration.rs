#[macro_use]
extern crate log;

use anyhow::Result;
use hessian2::value::{Map, PrimitiveValue, Value};
use hessian2::{from_slice, get_value_from_slice, to_vec};
use serde::{Deserialize, Serialize};

fn init() {
    pretty_env_logger::try_init_timed().ok();
}

// matches the API design in TOC_hessian_date.md verbatim.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Employee {
    name: String,
    #[serde(with = "hessian2::date")]
    created_at: i64,
}

#[test]
fn test_date_field_roundtrips_and_uses_hessian_date_tag() -> Result<()> {
    init();

    let employee = Employee {
        name: "Alice".to_owned(),
        created_at: 1_749_540_617_123,
    };

    let expect = {
        let mut m = Map::new();
        m.insert(
            PrimitiveValue::from("name".to_owned()),
            Value::from(Clone::clone(&employee.name)),
        );
        m.insert(
            PrimitiveValue::from("created_at".to_owned()),
            Value::Primitive(PrimitiveValue::Date(employee.created_at)),
        );

        let v = Value::from(m);

        to_vec(&v)?
    };

    info!("expect: {}", hex::encode(&expect));

    let actual = to_vec(&employee)?;

    info!("actual: {}", hex::encode(&actual));

    // Compare structurally rather than byte-for-byte: `expect` is built from
    // a `Map`, which is `HashMap`-backed and has no field-ordering
    // guarantee, so its wire bytes need not match the struct's
    // declaration-order encoding even when both represent the same data.
    // `Value` reads go through the dedicated reader, which preserves the
    // `Date` primitive (the serde path decodes the date tag as a plain long).
    assert_eq!(get_value_from_slice(&actual)?, get_value_from_slice(&expect)?);

    // 0x4a is the hessian date wire tag; a plain `i64` field would never
    // produce it (it'd use the long tag 0x4c or one of its short forms).
    assert!(hex::encode(&actual).contains("4a"));

    let back: Employee = from_slice(&actual)?;
    assert_eq!(employee, back);

    let actual_value: Value = get_value_from_slice(&actual)?;
    let created_at = &actual_value["created_at"];

    let mut is_date = false;

    if let Value::Primitive(pv) = created_at {
        if let PrimitiveValue::Date(d) = pv {
            assert_eq!(employee.created_at, *d);
            is_date = true;
        }
    }

    assert!(is_date, "value.created_at should be a Date!");

    Ok(())
}
