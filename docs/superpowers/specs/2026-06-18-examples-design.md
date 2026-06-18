# Examples Directory — Design Spec

**Date:** 2026-06-18  
**Status:** Approved

## Goal

Add `examples/hessian_object.rs` to demonstrate the `#[derive(HessianObject)]` proc-macro to users via `cargo run --example hessian_object`.

## File

**`examples/hessian_object.rs`** — single runnable example, three sequential sections:

### Section 1: Simple struct

```rust
#[derive(HessianObject)]
#[hessian(class = "com.example.User")]
struct User {
    id: i64,
    name: String,
    age: i32,
}
```

Calls `hessian_to_vec(&user)`, prints the hex output to stdout.

### Section 2: Nested objects + class-ref reuse

```rust
#[derive(HessianObject)]
#[hessian(class = "com.example.Address")]
struct Address { city: String, zipcode: String }

#[derive(HessianObject)]
#[hessian(class = "com.example.UserWithAddress")]
struct UserWithAddress {
    id: i64,
    name: String,
    home: Address,
    company: Address,
}
```

Serializes a `UserWithAddress` with two `Address` fields, prints hex. A comment points out that the `Address` class definition (`C ...`) appears only once in the output.

### Section 3: Field rename

```rust
#[derive(HessianObject)]
#[hessian(class = "com.example.Product")]
struct Product {
    #[hessian(rename = "productId")]
    product_id: i64,
    #[hessian(rename = "productName")]
    product_name: String,
}
```

Demonstrates that `productId` / `productName` (Java naming) differ from `product_id` / `product_name` (Rust naming).

## Cargo

No `Cargo.toml` changes needed. Cargo auto-discovers `examples/*.rs`. The `hex` crate is already in `[dev-dependencies]`.

## Running

```bash
cargo run --example hessian_object
```
