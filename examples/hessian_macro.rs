#[macro_use]
extern crate log;

use hessian2::{hessian, to_vec};

fn main() -> anyhow::Result<()> {
    pretty_env_logger::try_init_timed().ok();

    // ── Section 1: object literal ─────────────────────────────────────────
    info!("=== Object literal ===");
    let user = hessian!({
        "id": 123,
        "name": "Jerry",
        "age": 18,
    });
    info!("Value: {:?}", user);
    let bytes = to_vec(&user)?;
    info!("Encoded: {}\n", hex::encode(&bytes));

    // ── Section 2: arrays and nested containers ─────────────────────────────
    info!("=== Arrays & nested containers ===");
    let profile = hessian!({
        "user": {
            "name": "Jerry",
            "age": 18,
        },
        "roles": ["admin", "user"],
    });
    info!("Value: {:?}\n", profile);

    // ── Section 3: "$class" produces a Value::Object ─────────────────────────
    info!("=== \"$class\" => Value::Object ===");
    let user_obj = hessian!({
        "$class": "com.example.User",
        "id": 123,
        "name": "Jerry",
        "age": 18,
    });
    info!("Value: {}", user_obj);
    let bytes = to_vec(&user_obj)?;
    info!("Encoded: {}\n", hex::encode(&bytes));

    // ── Section 4: scalars, null, and variables ─────────────────────────────
    info!("=== Scalars, null, and variables ===");
    let age = 18;
    info!("null    => {:?}", hessian!(null));
    info!("true    => {:?}", hessian!(true));
    info!("123     => {:?}", hessian!(123));
    info!("\"foo\"   => {:?}", hessian!("foo"));
    info!("age (var) => {:?}", hessian!(age));

    Ok(())
}
