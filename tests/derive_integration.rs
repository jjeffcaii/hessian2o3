#[macro_use]
extern crate log;
use anyhow::Result;

use hessian2::{Hessian, hessian_from_slice, hessian_to_vec};

fn init() {
    pretty_env_logger::try_init_timed().ok();
}

#[derive(Hessian, Debug, PartialEq)]
#[hessian(class = "com.example.Point")]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn test_derive_simple_struct() -> Result<()> {
    init();
    // Same expected output as the manual test in Task 3:
    //  43 11 "com.example.Point" 92 01 78 01 79 60 91 92
    let bytes = hessian_to_vec(&Point { x: 1, y: 2 })?;
    assert_eq!(
        "4311636f6d2e6578616d706c652e506f696e749201780179609192",
        hex::encode(&bytes)
    );
    Ok(())
}

#[derive(Hessian, Debug, PartialEq)]
#[hessian(class = "com.example.User")]
struct User {
    #[hessian(rename = "id")]
    id: i64,
    #[hessian(rename = "name")]
    name: String,
    #[hessian(rename = "age")]
    age: i32,
}

#[test]
fn test_derive_with_rename() -> Result<()> {
    init();

    // Expected for User{id:1234, name:"杨幂", age:18}:
    //  43 13 "com.example.User"   C + class name (19 chars)
    //  93                            field count 3
    //  02 6964                       "id"
    //  04 6e616d65                   "name"
    //  03 616765                     "age"
    //  60                            object ref 0
    //  fc d2                         put_i64(1234)
    //  02 e69da8e5b982               "杨幂" (2 chars, each 3 UTF-8 bytes)
    //  a2                            put_i32(18)
    let bytes = hessian_to_vec(&User {
        id: 1234,
        name: String::from("杨幂"),
        age: 18,
    })?;

    assert_eq!(
        "4310636f6d2e6578616d706c652e5573657293026964046e616d650361676560fcd202e69da8e5b982a2",
        hex::encode(&bytes)
    );

    Ok(())
}

#[derive(Hessian, Debug, PartialEq)]
#[hessian(class = "com.example.Address")]
struct Address {
    #[hessian(rename = "city")]
    city: String,
    #[hessian(rename = "zipcode")]
    zipcode: String,
}

#[derive(Hessian, Debug, PartialEq)]
#[hessian(class = "com.example.UserFull")]
struct UserFull {
    #[hessian(rename = "id")]
    id: i64,
    #[hessian(rename = "name")]
    name: String,
    #[hessian(rename = "age")]
    age: i32,
    #[hessian(rename = "home")]
    home: Address,
    #[hessian(rename = "company")]
    company: Address,
}

#[test]
fn test_nested_objects_match_encode_test() -> Result<()> {
    init();
    // Expected output matches encode::tests::test_object exactly,
    // except the outer class is "com.example.UserFull" not "com.example.User"
    // (different name to avoid collision with the User struct above).
    //
    // Byte structure:
    //  C "com.example.UserFull" (24 chars) 5-fields [id,name,age,home,company]
    //  0x60  id=1234  name="杨幂"  age=18
    //  C "com.example.Address" (22 chars) 2-fields [city,zipcode]
    //  0x61  "Shanghai" "200000"
    //  0x61  "Beijing"  "100000"   ← class def NOT repeated
    let user = UserFull {
        id: 1234,
        name: String::from("杨幂"),
        age: 18,
        home: Address {
            city: String::from("Shanghai"),
            zipcode: String::from("200000"),
        },
        company: Address {
            city: String::from("Beijing"),
            zipcode: String::from("100000"),
        },
    };
    let bytes = hessian_to_vec(&user).unwrap();
    let s = hex::encode(&bytes);

    assert_eq!(
        "4314636f6d2e6578616d706c652e5573657246756c6c95026964046e616d650361676504686f6d6507636f6d70616e7960fcd202e69da8e5b982a24313636f6d2e6578616d706c652e41646472657373920463697479077a6970636f646561085368616e676861690632303030303061074265696a696e6706313030303030",
        &s,
    );

    Ok(())
}

#[test]
fn test_option_and_vec_fields() -> Result<()> {
    init();

    #[derive(Hessian, Debug, PartialEq)]
    #[hessian(class = "com.example.Container")]
    struct Container {
        #[hessian(rename = "maybeVal")]
        maybe_val: Option<i32>,
        #[hessian(rename = "nums")]
        nums: Vec<i32>,
    }

    // None field → 4e (null), vec [1,2,3] → 7b 91 92 93
    let c = Container {
        maybe_val: None,
        nums: vec![1, 2, 3],
    };
    let bytes = hessian_to_vec(&c)?;
    let s = hex::encode(&bytes);
    // null appears
    assert!(s.contains("4e"), "None should encode as null (4e)");
    // list [1,2,3] appears: 7b = BC_LIST_DIRECT_UNTYPED+3, 91 92 93 = 1,2,3
    assert!(
        s.contains("7b919293"),
        "Vec<i32> [1,2,3] should encode as 7b919293"
    );

    // Some field
    let c2 = Container {
        maybe_val: Some(42),
        nums: vec![],
    };
    let bytes2 = hessian_to_vec(&c2)?;
    let s2 = hex::encode(&bytes2);
    // 42 as i32: 0x90 + 42 = 0xba
    assert!(s2.contains("ba"), "Some(42) should encode as i32 value ba");
    // empty Vec: 78 = BC_LIST_DIRECT_UNTYPED+0
    assert!(s2.contains("78"), "empty Vec should encode as 78");

    Ok(())
}

#[test]
fn test_derive_roundtrip_simple() -> Result<()> {
    init();

    let point = Point { x: 1, y: 2 };
    let bytes = hessian_to_vec(&point)?;
    let back: Point = hessian_from_slice(&bytes)?;
    assert_eq!(point, back);

    Ok(())
}

#[test]
fn test_derive_roundtrip_with_rename() -> Result<()> {
    init();

    let user = User {
        id: 1234,
        name: String::from("杨幂"),
        age: 18,
    };
    let bytes = hessian_to_vec(&user)?;
    let back: User = hessian_from_slice(&bytes)?;
    assert_eq!(user, back);

    Ok(())
}

#[test]
fn test_derive_roundtrip_nested() -> Result<()> {
    init();

    let user = UserFull {
        id: 1234,
        name: String::from("杨幂"),
        age: 18,
        home: Address {
            city: String::from("Shanghai"),
            zipcode: String::from("200000"),
        },
        company: Address {
            city: String::from("Beijing"),
            zipcode: String::from("100000"),
        },
    };

    debug!("*** begin encoding...");

    // the second Address is only a class *reference* on the wire; decoding
    // must resolve it through the shared Context.
    let bytes = hessian_to_vec(&user)?;
    let expect = hex::decode(
        "4314636f6d2e6578616d706c652e5573657246756c6c95026964046e616d650361676504686f6d6507636f6d70616e7960fcd202e69da8e5b982a24313636f6d2e6578616d706c652e41646472657373920463697479077a6970636f646561085368616e676861690632303030303061074265696a696e6706313030303030",
    )?;
    assert_eq!(bytes, expect);

    debug!("*** begin decoding...");

    let back: UserFull = hessian_from_slice(&bytes)?;
    assert_eq!(user, back);

    Ok(())
}

#[test]
fn test_derive_roundtrip_option_and_vec() -> Result<()> {
    init();

    #[derive(Hessian, Debug, PartialEq)]
    #[hessian(class = "com.example.Container")]
    struct Container {
        #[hessian(rename = "maybeVal")]
        maybe_val: Option<i32>,
        #[hessian(rename = "nums")]
        nums: Vec<i32>,
    }

    for c in [
        Container {
            maybe_val: None,
            nums: vec![1, 2, 3],
        },
        Container {
            maybe_val: Some(42),
            nums: vec![],
        },
    ] {
        let bytes = hessian_to_vec(&c)?;
        let back: Container = hessian_from_slice(&bytes)?;
        assert_eq!(c, back);
    }

    Ok(())
}

#[test]
fn test_derive_deserialize_wrong_shape_errors() -> Result<()> {
    init();

    // a bare integer is not an object
    let b = hessian_to_vec(&123i32)?;
    assert!(hessian_from_slice::<Point>(&b).is_err());

    Ok(())
}
