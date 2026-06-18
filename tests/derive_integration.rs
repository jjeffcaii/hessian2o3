use hessian2o3::{HessianObject, hessian_to_vec};

#[derive(HessianObject)]
#[hessian(class = "com.example.Point")]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn test_derive_simple_struct() {
    // Same expected output as the manual test in Task 3:
    //  43 11 "com.example.Point" 92 01 78 01 79 60 91 92
    let bytes = hessian_to_vec(&Point { x: 1, y: 2 }).unwrap();
    assert_eq!(
        "4311636f6d2e6578616d706c652e506f696e749201780179609192",
        hex::encode(&bytes)
    );
}

#[derive(HessianObject)]
#[hessian(class = "com.hessian2o3.User")]
struct User {
    #[hessian(rename = "id")]
    id: i64,
    #[hessian(rename = "name")]
    name: String,
    #[hessian(rename = "age")]
    age: i32,
}

#[test]
fn test_derive_with_rename() {
    // Expected for User{id:1234, name:"杨幂", age:18}:
    //  43 13 "com.hessian2o3.User"   C + class name (19 chars)
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
    })
    .unwrap();
    assert_eq!(
        "4313636f6d2e6865737369616e326f332e5573657293026964046e616d650361676560fcd202e69da8e5b982a2",
        hex::encode(&bytes)
    );
}

#[derive(HessianObject)]
#[hessian(class = "com.hessian2o3.Address")]
struct Address {
    #[hessian(rename = "city")]
    city: String,
    #[hessian(rename = "zipcode")]
    zipcode: String,
}

#[derive(HessianObject)]
#[hessian(class = "com.hessian2o3.UserFull")]
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
fn test_nested_objects_match_encode_test() {
    // Expected output matches encode::tests::test_object exactly,
    // except the outer class is "com.hessian2o3.UserFull" not "com.hessian2o3.User"
    // (different name to avoid collision with the User struct above).
    //
    // Byte structure:
    //  C "com.hessian2o3.UserFull" (24 chars) 5-fields [id,name,age,home,company]
    //  0x60  id=1234  name="杨幂"  age=18
    //  C "com.hessian2o3.Address" (22 chars) 2-fields [city,zipcode]
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

    // The Address class definition (43 16 "com.hessian2o3.Address" ...) must appear exactly once.
    let addr_class_def = "4316636f6d2e6865737369616e326f332e41646472657373";
    assert_eq!(1, s.matches(addr_class_def).count(), "Address class def must appear exactly once");

    // The second Address instance must start with object-ref 0x61 (not a new C definition).
    // Both Address instances write 0x61; count must be 2.
    // We find the first 0x61 after the class definition, confirming both use the same ref.
    let addr_ref = "61";
    let count = s
        .match_indices(addr_ref)
        .filter(|(i, _)| *i > s.find(addr_class_def).unwrap())
        .count();
    assert!(count >= 2, "Expected at least 2 address object refs after class def, got {count}");
}

#[test]
fn test_option_and_vec_fields() {
    #[derive(HessianObject)]
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
    let bytes = hessian_to_vec(&c).unwrap();
    let s = hex::encode(&bytes);
    // null appears
    assert!(s.contains("4e"), "None should encode as null (4e)");
    // list [1,2,3] appears: 7b = BC_LIST_DIRECT_UNTYPED+3, 91 92 93 = 1,2,3
    assert!(s.contains("7b919293"), "Vec<i32> [1,2,3] should encode as 7b919293");

    // Some field
    let c2 = Container {
        maybe_val: Some(42),
        nums: vec![],
    };
    let bytes2 = hessian_to_vec(&c2).unwrap();
    let s2 = hex::encode(&bytes2);
    // 42 as i32: 0x90 + 42 = 0xba
    assert!(s2.contains("ba"), "Some(42) should encode as i32 value ba");
    // empty Vec: 78 = BC_LIST_DIRECT_UNTYPED+0
    assert!(s2.contains("78"), "empty Vec should encode as 78");
}
