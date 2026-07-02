use hessian2o3::hessian;
use hessian2o3::to_vec;

fn main() {
    // ── Section 1: object literal ─────────────────────────────────────────
    println!("=== Object literal ===");
    let user = hessian!({
        "id": 123,
        "name": "Jerry",
        "age": 18,
    });
    println!("Value: {:?}", user);
    let bytes = to_vec(&user).unwrap();
    println!("Encoded: {}\n", hex::encode(&bytes));

    // ── Section 2: arrays and nested containers ─────────────────────────────
    println!("=== Arrays & nested containers ===");
    let profile = hessian!({
        "user": {
            "name": "Jerry",
            "age": 18,
        },
        "roles": ["admin", "user"],
    });
    println!("Value: {:?}\n", profile);

    // ── Section 3: "$class" produces a Value::Object ─────────────────────────
    println!("=== \"$class\" => Value::Object ===");
    let user_obj = hessian!({
        "$class": "com.example.User",
        "id": 123,
        "name": "Jerry",
        "age": 18,
    });
    println!("Value: {}", user_obj);
    let bytes = to_vec(&user_obj).unwrap();
    println!("Encoded: {}\n", hex::encode(&bytes));

    // ── Section 4: scalars, null, and variables ─────────────────────────────
    println!("=== Scalars, null, and variables ===");
    let age = 18;
    println!("null    => {:?}", hessian!(null));
    println!("true    => {:?}", hessian!(true));
    println!("123     => {:?}", hessian!(123));
    println!("\"foo\"   => {:?}", hessian!("foo"));
    println!("age (var) => {:?}", hessian!(age));
}
