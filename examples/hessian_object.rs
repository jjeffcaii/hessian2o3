#[macro_use]
extern crate log;

use hessian2::HessianSerialize;
use hessian2::hessian::hessian_from_slice;
use hessian2::prelude::*;
use hessian2::to_vec;

// ── Section 1: simple struct ──────────────────────────────────────────────
#[derive(HessianSerialize, HessianDeserialize, Debug, PartialEq)]
#[hessian(class = "com.example.User")]
struct User {
    id: i64,
    name: String,
    age: i32,
}

// ── Section 2: nested objects ─────────────────────────────────────────────
#[derive(HessianSerialize, HessianDeserialize, Debug, PartialEq)]
#[hessian(class = "com.example.Address")]
struct Address {
    city: String,
    zipcode: String,
}

#[derive(HessianSerialize, HessianDeserialize, Debug, PartialEq)]
#[hessian(class = "com.example.UserWithAddress")]
struct UserWithAddress {
    id: i64,
    name: String,
    home: Address,
    company: Address,
}

// ── Section 3: field rename (Rust snake_case → Java camelCase) ───────────
#[derive(HessianSerialize, HessianDeserialize, Debug, PartialEq)]
#[hessian(class = "com.example.Product")]
struct Product {
    #[hessian(rename = "productId")]
    product_id: i64,
    #[hessian(rename = "productName")]
    product_name: String,
}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::try_init_timed().ok();

    // ── Section 1 ──
    info!("=== Simple struct ===");
    let user = User {
        id: 1,
        name: String::from("Alice"),
        age: 30,
    };

    let bytes = to_vec(&Hessian(&user))?;
    info!("User: {}", hex::encode(&bytes));
    let back: User = hessian_from_slice(&bytes)?;
    assert_eq!(user, back);
    info!("Decoded: {:?}\n", back);

    // ── Section 2 ──
    info!("=== Nested objects (class-ref reuse) ===");
    let uwaddr = UserWithAddress {
        id: 2,
        name: String::from("Bob"),
        home: Address {
            city: String::from("Shanghai"),
            zipcode: String::from("200000"),
        },
        company: Address {
            city: String::from("Beijing"),
            zipcode: String::from("100000"),
        },
    };
    let bytes = to_vec(&Hessian(&uwaddr))?;
    let hex_str = hex::encode(&bytes);
    info!("UserWithAddress: {}", hex_str);
    // "com.example.Address" in hex: 636f6d2e6578616d706c652e41646472657373
    let class_def_count = hex_str
        .matches("636f6d2e6578616d706c652e41646472657373")
        .count();
    info!(
        "Address class definition appears {} time(s) (expected 1 — second instance reuses ref)",
        class_def_count
    );
    let back: UserWithAddress = hessian_from_slice(&bytes)?;
    assert_eq!(uwaddr, back);
    info!("Decoded: {:?}\n", back);

    // ── Section 3 ──
    info!("=== Field rename (snake_case → camelCase) ===");
    let product = Product {
        product_id: 42,
        product_name: String::from("Widget"),
    };
    let bytes = to_vec(&Hessian(&product))?;
    info!("Product: {}", hex::encode(&bytes));
    info!("(wire fields are 'productId' / 'productName', not Rust's snake_case names)");
    let back: Product = hessian_from_slice(&bytes)?;
    assert_eq!(product, back);
    info!("Decoded: {:?}", back);

    Ok(())
}
