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
