use hessian2o3::{HessianObject, hessian_to_vec};

// ── Section 1: simple struct ──────────────────────────────────────────────
#[derive(HessianObject)]
#[hessian(class = "com.example.User")]
struct User {
    id: i64,
    name: String,
    age: i32,
}

// ── Section 2: nested objects ─────────────────────────────────────────────
#[derive(HessianObject)]
#[hessian(class = "com.example.Address")]
struct Address {
    city: String,
    zipcode: String,
}

#[derive(HessianObject)]
#[hessian(class = "com.example.UserWithAddress")]
struct UserWithAddress {
    id: i64,
    name: String,
    home: Address,
    company: Address,
}

// ── Section 3: field rename (Rust snake_case → Java camelCase) ───────────
#[derive(HessianObject)]
#[hessian(class = "com.example.Product")]
struct Product {
    #[hessian(rename = "productId")]
    product_id: i64,
    #[hessian(rename = "productName")]
    product_name: String,
}

fn main() {
    // ── Section 1 ──
    println!("=== Simple struct ===");
    let user = User {
        id: 1,
        name: String::from("Alice"),
        age: 30,
    };
    let bytes = hessian_to_vec(&user).unwrap();
    println!("User: {}\n", hex::encode(&bytes));

    // ── Section 2 ──
    println!("=== Nested objects (class-ref reuse) ===");
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
    let bytes = hessian_to_vec(&uwaddr).unwrap();
    let hex_str = hex::encode(&bytes);
    println!("UserWithAddress: {}", hex_str);
    // "com.example.Address" in hex: 636f6d2e6578616d706c652e41646472657373
    let class_def_count = hex_str
        .matches("636f6d2e6578616d706c652e41646472657373")
        .count();
    println!(
        "Address class definition appears {} time(s) (expected 1 — second instance reuses ref)\n",
        class_def_count
    );

    // ── Section 3 ──
    println!("=== Field rename (snake_case → camelCase) ===");
    let product = Product {
        product_id: 42,
        product_name: String::from("Widget"),
    };
    let bytes = hessian_to_vec(&product).unwrap();
    println!("Product: {}", hex::encode(&bytes));
    println!("(wire fields are 'productId' / 'productName', not Rust's snake_case names)");
}
