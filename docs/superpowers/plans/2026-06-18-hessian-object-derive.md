# Hessian Object Derive Macro — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `#[derive(HessianObject)]` that auto-generates `HessianSerialize` impls producing correct Hessian 2.0 object encoding with shared `Context` for class-ref deduplication across nested objects.

**Architecture:** A new `HessianSerialize` trait (with `Context` param) lives in `src/hessian.rs`; primitive impls cover all common field types. A `proc-macro = true` crate `hessian2o3-derive` generates the trait impl from `#[hessian(class = "...")]` / `#[hessian(rename = "...")]` annotations. Top-level `hessian_to_writer` / `hessian_to_vec` functions create the `Context` and drive serialization.

**Tech Stack:** Rust 2024 edition, `syn 1` (attribute parsing), `quote 1`, `proc-macro2 1`, existing `encode::{begin_object, put_*, begin_list}` helpers.

## Global Constraints

- Edition 2024 throughout; minimum Rust version implied by that (≥ 1.85).
- `Context` and `begin_object` in `encode.rs` must be `pub` (not `pub(crate)`) so the derive-generated code can reference them from outside the crate.
- The derive macro generates code that paths through `::hessian2o3::HessianSerialize` and `::hessian2o3::encode::Context`; tests that exercise the derive macro live in `tests/derive_integration.rs` (external crate scope) to avoid needing `extern crate self`.
- Primitive HessianSerialize tests live in `src/hessian.rs` `#[cfg(test)]`.
- `Vec<u8>` serializes as a Hessian untyped list of int values (not binary); binary data is out of scope.
- Do not touch `src/ser.rs`, `src/serde.rs`, or the existing serde-based `to_writer`/`to_vec` path.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add `[workspace]`, add `hessian2o3-derive` path dependency |
| Create | `hessian2o3-derive/Cargo.toml` | `proc-macro = true` crate manifest |
| Create | `hessian2o3-derive/src/lib.rs` | `HessianObject` derive macro |
| Modify | `src/lib.rs` | `pub mod encode;`, `pub mod hessian;`, re-exports |
| Modify | `src/encode.rs` | Make `Context` and `begin_object` `pub` |
| Create | `src/hessian.rs` | `HessianSerialize` trait, primitive impls, `hessian_to_*` |
| Create | `tests/derive_integration.rs` | End-to-end derive macro tests |

---

### Task 1: Cargo Workspace + Derive Crate Scaffold

**Files:**
- Modify: `Cargo.toml`
- Create: `hessian2o3-derive/Cargo.toml`
- Create: `hessian2o3-derive/src/lib.rs`

**Interfaces:**
- Produces: `hessian2o3_derive::HessianObject` derive macro (stub — compiles, emits no tokens yet)

- [ ] **Step 1: Add `[workspace]` to root `Cargo.toml` and add the derive crate as a path dependency**

Replace the full contents of `Cargo.toml` with:

```toml
[workspace]
members = [".", "hessian2o3-derive"]

[package]
name = "hessian2o3"
version = "0.1.0"
edition = "2024"

[build-dependencies]
string_cache_codegen = "0.6"

[dependencies]
log = "0.4.32"
serde = { version = "1", features = ["derive"] }
smallvec = "1.15.2"
string_cache = "0.9.0"
thiserror = "2"
hessian2o3-derive = { path = "hessian2o3-derive" }

[dev-dependencies]
pretty_env_logger = "0.5"
chrono = "0.4.45"
hex = "0.4.3"
```

- [ ] **Step 2: Create `hessian2o3-derive/Cargo.toml`**

```toml
[package]
name = "hessian2o3-derive"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = { version = "1", features = ["full"] }
quote = "1"
proc-macro2 = "1"
```

- [ ] **Step 3: Create `hessian2o3-derive/src/lib.rs` (stub)**

```rust
use proc_macro::TokenStream;

#[proc_macro_derive(HessianObject, attributes(hessian))]
pub fn derive_hessian_object(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
```

- [ ] **Step 4: Verify the workspace builds**

```bash
cargo build
```

Expected: compiles without errors (the stub macro produces no output, which is fine).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml hessian2o3-derive/
git commit -m "chore: set up cargo workspace and hessian2o3-derive stub"
```

---

### Task 2: Make `Context` and `begin_object` Public + Create `HessianSerialize` Trait + Primitive Impls

**Files:**
- Modify: `src/encode.rs` (two visibility changes)
- Modify: `src/lib.rs` (expose `encode` and `hessian` modules + re-exports)
- Create: `src/hessian.rs`

**Interfaces:**
- Consumes: `encode::put_i32`, `put_i64`, `put_f64`, `put_bool`, `put_str`, `put_bytes`, `put_null`, `begin_list`, `begin_object`, `Context`
- Produces:
  - `hessian2o3::HessianSerialize` — trait with `fn hessian_serialize<W: Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()>`
  - `hessian2o3::encode::Context` — now `pub`
  - `hessian2o3::encode::begin_object` — now `pub`

- [ ] **Step 1: Write the failing test in `src/hessian.rs`**

Create the file with the trait and a test that checks all primitive impls. The test will fail to compile until the impls exist.

```rust
use crate::encode::{self, Context};
use std::io;

pub trait HessianSerialize {
    fn hessian_serialize<W: io::Write>(
        &self,
        w: &mut W,
        ctx: &mut Context,
    ) -> io::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Context;

    fn hex<T: HessianSerialize>(v: &T) -> String {
        let mut buf = vec![];
        let mut ctx = Context::default();
        v.hessian_serialize(&mut buf, &mut ctx).unwrap();
        hex::encode(buf)
    }

    #[test]
    fn test_primitives() {
        // bool
        assert_eq!("54", hex(&true));
        assert_eq!("46", hex(&false));
        // i8 / i16 / i32 → put_i32
        assert_eq!("90", hex(&0i32));
        assert_eq!("91", hex(&1i32));
        assert_eq!("90", hex(&0i8));
        assert_eq!("90", hex(&0i16));
        // i64 → put_i64
        assert_eq!("e0", hex(&0i64));
        assert_eq!("e1", hex(&1i64));
        // u8 / u16 → put_i32
        assert_eq!("90", hex(&0u8));
        assert_eq!("90", hex(&0u16));
        // u32 / u64 → put_i64
        assert_eq!("e0", hex(&0u32));
        assert_eq!("e0", hex(&0u64));
        // f32 / f64
        assert_eq!("5b", hex(&0.0f64));
        assert_eq!("5c", hex(&1.0f64));
        assert_eq!("5b", hex(&0.0f32));
        // String / &str
        assert_eq!("00", hex(&String::from("")));
        assert_eq!("0568656c6c6f", hex(&String::from("hello")));
        assert_eq!("00", hex(&""));
        assert_eq!("0568656c6c6f", hex(&"hello"));
        // Option
        assert_eq!("4e", hex(&None::<i32>));
        assert_eq!("91", hex(&Some(1i32)));
        // Vec<T: HessianSerialize>
        assert_eq!("78", hex(&Vec::<i32>::new()));
        assert_eq!("7b919293", hex(&vec![1i32, 2, 3]));
    }
}
```

- [ ] **Step 2: Run — expect compile error**

```bash
cargo test -p hessian2o3 hessian::tests::test_primitives 2>&1 | head -20
```

Expected: error about missing impls and missing module in `lib.rs`.

- [ ] **Step 3: In `src/encode.rs`, make `Context` and `begin_object` public**

Change line ~56:
```rust
// before
pub(crate) struct Context {
// after
pub struct Context {
```

Change line ~394:
```rust
// before
pub(crate) fn begin_object<W, S>(
// after
pub fn begin_object<W, S>(
```

- [ ] **Step 4: In `src/lib.rs`, expose `encode` module and add `hessian` module**

Change the existing `pub(crate) mod encode;` line:
```rust
// before
pub(crate) mod encode;
// after
pub mod encode;
```

Add new lines after the existing `mod` declarations:
```rust
pub mod hessian;
pub use hessian::HessianSerialize;
pub use hessian2o3_derive::HessianObject;
```

- [ ] **Step 5: Add primitive impls to `src/hessian.rs`** (append below the trait definition)

```rust
impl HessianSerialize for bool {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_bool(w, *self)
    }
}

impl HessianSerialize for i8 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for i16 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for i32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self)
    }
}

impl HessianSerialize for i64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i64(w, *self)
    }
}

impl HessianSerialize for u8 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for u16 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i32(w, *self as i32)
    }
}

impl HessianSerialize for u32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i64(w, *self as i64)
    }
}

impl HessianSerialize for u64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_i64(w, *self as i64)
    }
}

impl HessianSerialize for f32 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_f64(w, *self as f64)
    }
}

impl HessianSerialize for f64 {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_f64(w, *self)
    }
}

impl HessianSerialize for str {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_str(w, self)
    }
}

impl HessianSerialize for String {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, _ctx: &mut Context) -> io::Result<()> {
        encode::put_str(w, self.as_str())
    }
}

impl<T: HessianSerialize> HessianSerialize for Option<T> {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        match self {
            None => encode::put_null(w),
            Some(v) => v.hessian_serialize(w, ctx),
        }
    }
}

impl<T: HessianSerialize> HessianSerialize for Vec<T> {
    fn hessian_serialize<W: io::Write>(&self, w: &mut W, ctx: &mut Context) -> io::Result<()> {
        encode::begin_list(w, None, self.len())?;
        for item in self {
            item.hessian_serialize(w, ctx)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 6: Run the test — expect PASS**

```bash
cargo test -p hessian2o3 hessian::tests::test_primitives
```

Expected: `test hessian::tests::test_primitives ... ok`

- [ ] **Step 7: Run full test suite to verify nothing regressed**

```bash
cargo test -p hessian2o3
```

Expected: all existing tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/encode.rs src/lib.rs src/hessian.rs
git commit -m "feat: add HessianSerialize trait with primitive impls"
```

---

### Task 3: `hessian_to_writer` / `hessian_to_vec` + Manual-Impl Integration Test

**Files:**
- Modify: `src/hessian.rs` (append two public functions + a test)
- Modify: `src/lib.rs` (re-export the new functions)

**Interfaces:**
- Consumes: `HessianSerialize`, `encode::Context`, `crate::Error`
- Produces:
  - `hessian2o3::hessian_to_writer<W: Write, T: HessianSerialize>(writer: &mut W, value: &T) -> Result<()>`
  - `hessian2o3::hessian_to_vec<T: HessianSerialize>(value: &T) -> Result<Vec<u8>>`

- [ ] **Step 1: Write the failing test in `src/hessian.rs` `#[cfg(test)]`**

Append this test to the existing `tests` module inside `src/hessian.rs`:

```rust
    #[test]
    fn test_manual_object() {
        // Manually implement HessianSerialize for a Point struct to verify
        // hessian_to_vec produces the correct object encoding.
        struct Point { x: i32, y: i32 }

        impl HessianSerialize for Point {
            fn hessian_serialize<W: io::Write>(
                &self,
                w: &mut W,
                ctx: &mut Context,
            ) -> io::Result<()> {
                encode::begin_object(w, ctx, "com.example.Point", &["x", "y"])?;
                self.x.hessian_serialize(w, ctx)?;
                self.y.hessian_serialize(w, ctx)?;
                Ok(())
            }
        }

        // Expected byte-by-byte:
        //  43               C (class definition)
        //  11               17 chars (direct string)
        //  636f6d2e6578616d706c652e506f696e74  "com.example.Point"
        //  92               put_i32(2) = 0x90+2 (field count)
        //  01 78            "x" (1 char)
        //  01 79            "y" (1 char)
        //  60               BC_OBJECT_DIRECT + 0 (ref 0)
        //  91               put_i32(1)
        //  92               put_i32(2)
        let bytes = crate::hessian_to_vec(&Point { x: 1, y: 2 }).unwrap();
        assert_eq!(
            "4311636f6d2e6578616d706c652e506f696e749201780179609192",
            hex::encode(&bytes)
        );
    }
```

- [ ] **Step 2: Run — expect compile error** (functions don't exist yet)

```bash
cargo test -p hessian2o3 hessian::tests::test_manual_object 2>&1 | head -10
```

Expected: error `cannot find function hessian_to_vec`.

- [ ] **Step 3: Add `hessian_to_writer` and `hessian_to_vec` to `src/hessian.rs`** (append above `#[cfg(test)]`)

```rust
pub fn hessian_to_writer<W: io::Write, T: HessianSerialize>(
    writer: &mut W,
    value: &T,
) -> crate::Result<()> {
    let mut ctx = Context::default();
    value.hessian_serialize(writer, &mut ctx).map_err(crate::Error::IO)
}

pub fn hessian_to_vec<T: HessianSerialize>(value: &T) -> crate::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(128);
    hessian_to_writer(&mut buf, value)?;
    Ok(buf)
}
```

- [ ] **Step 4: Re-export both functions from `src/lib.rs`**

Add to the re-export block in `src/lib.rs`:

```rust
pub use hessian::{hessian_to_vec, hessian_to_writer};
```

- [ ] **Step 5: Run the test — expect PASS**

```bash
cargo test -p hessian2o3 hessian::tests::test_manual_object
```

Expected: `test hessian::tests::test_manual_object ... ok`

- [ ] **Step 6: Run full suite**

```bash
cargo test -p hessian2o3
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/hessian.rs src/lib.rs
git commit -m "feat: add hessian_to_writer and hessian_to_vec"
```

---

### Task 4: `HessianObject` Derive Macro Implementation

**Files:**
- Modify: `hessian2o3-derive/src/lib.rs` (full implementation)

**Interfaces:**
- Consumes (at code-gen time): `syn::DeriveInput`, struct attrs `#[hessian(class = "...")]`, field attrs `#[hessian(rename = "...")]`
- Produces (at compile time of user crate): `impl ::hessian2o3::HessianSerialize for StructName { ... }`

- [ ] **Step 1: Write the test first** — create `tests/derive_integration.rs`

```rust
use hessian2o3::{HessianObject, HessianSerialize, hessian_to_vec};

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
```

- [ ] **Step 2: Run — expect compile error** (derive macro is a stub)

```bash
cargo test --test derive_integration 2>&1 | head -20
```

Expected: the macro expands to nothing → `Point` doesn't implement `HessianSerialize` → compile error.

- [ ] **Step 3: Implement the derive macro in `hessian2o3-derive/src/lib.rs`**

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Error, Fields, Lit, Meta, MetaList, MetaNameValue,
    NestedMeta,
};

#[proc_macro_derive(HessianObject, attributes(hessian))]
pub fn derive_hessian_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let class_name = extract_class(&input)?;

    let named_fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(Error::new_spanned(
                    name,
                    "HessianObject only supports named-field structs",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                name,
                "HessianObject only supports structs",
            ))
        }
    };

    let mut java_names: Vec<String> = Vec::new();
    let mut rust_idents: Vec<&syn::Ident> = Vec::new();

    for field in named_fields {
        let ident = field.ident.as_ref().unwrap();
        let java = extract_rename(&field.attrs)?.unwrap_or_else(|| ident.to_string());
        java_names.push(java);
        rust_idents.push(ident);
    }

    let field_serializers = rust_idents.iter().map(|ident| {
        quote! {
            ::hessian2o3::HessianSerialize::hessian_serialize(&self.#ident, w, ctx)?;
        }
    });

    Ok(quote! {
        impl ::hessian2o3::HessianSerialize for #name {
            fn hessian_serialize<W: ::std::io::Write>(
                &self,
                w: &mut W,
                ctx: &mut ::hessian2o3::encode::Context,
            ) -> ::std::io::Result<()> {
                ::hessian2o3::encode::begin_object(
                    w,
                    ctx,
                    #class_name,
                    &[#(#java_names),*],
                )?;
                #(#field_serializers)*
                ::std::result::Result::Ok(())
            }
        }
    })
}

fn extract_class(input: &DeriveInput) -> syn::Result<String> {
    for attr in &input.attrs {
        if attr.path.is_ident("hessian") {
            if let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() {
                for item in &nested {
                    if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) = item
                    {
                        if path.is_ident("class") {
                            return Ok(s.value());
                        }
                    }
                }
            }
        }
    }
    Err(Error::new_spanned(
        &input.ident,
        "HessianObject requires #[hessian(class = \"...\")]",
    ))
}

fn extract_rename(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path.is_ident("hessian") {
            if let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() {
                for item in &nested {
                    if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) = item
                    {
                        if path.is_ident("rename") {
                            return Ok(Some(s.value()));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}
```

- [ ] **Step 4: Run the integration tests — expect PASS**

```bash
cargo test --test derive_integration
```

Expected:
```
test test_derive_simple_struct ... ok
test test_derive_with_rename ... ok
```

- [ ] **Step 5: Run full suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add hessian2o3-derive/src/lib.rs tests/derive_integration.rs
git commit -m "feat: implement HessianObject derive macro"
```

---

### Task 5: Integration — Nested Objects + Class-Ref Deduplication

**Files:**
- Modify: `tests/derive_integration.rs` (add two more tests)

**Interfaces:**
- Consumes: everything from Tasks 1–4 — `HessianObject`, `HessianSerialize`, `hessian_to_vec`, `encode::Context`

- [ ] **Step 1: Write the failing tests** (append to `tests/derive_integration.rs`)

```rust
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
```

- [ ] **Step 2: Run — expect compile error** (new structs use `HessianObject` which already works, but tests are new)

```bash
cargo test --test derive_integration 2>&1 | head -10
```

Expected: compile succeeds (derive works from Task 4), but tests fail if results don't match. Run to see actual vs expected.

- [ ] **Step 3: Run tests — expect PASS**

```bash
cargo test --test derive_integration
```

Expected: all 4 tests pass:
```
test test_derive_simple_struct ... ok
test test_derive_with_rename ... ok
test test_nested_objects_match_encode_test ... ok
test test_option_and_vec_fields ... ok
```

- [ ] **Step 4: Run full suite one final time**

```bash
cargo test
```

Expected: all tests in all crates pass.

- [ ] **Step 5: Commit**

```bash
git add tests/derive_integration.rs
git commit -m "test: add nested object and option/vec integration tests"
```

---

## Self-Review

**Spec coverage check:**
- ✅ Workspace with `hessian2o3-derive` — Task 1
- ✅ `HessianSerialize` trait + primitive impls — Task 2
- ✅ `Option<T>` and `Vec<T>` blanket impls — Task 2
- ✅ `hessian_to_writer` / `hessian_to_vec` — Task 3
- ✅ `#[derive(HessianObject)]` + `#[hessian(class = "...")]` — Task 4
- ✅ `#[hessian(rename = "...")]` field attribute — Task 4
- ✅ Defaults to Rust field name when no rename — Task 4 (extract_rename returns None → ident.to_string())
- ✅ Compile error when `class` missing — Task 4 (extract_class returns Err)
- ✅ Compile error for non-named-field struct — Task 4 (Fields::Named match arm)
- ✅ Context sharing across nested objects — Task 5 (single Context created in hessian_to_writer, passed by &mut ref)
- ✅ Class-ref deduplication — Task 5 (test_nested_objects_match_encode_test)
- ✅ Single struct test — Task 4 (test_derive_simple_struct)
- ✅ Option field — Task 5 (test_option_and_vec_fields)
- ✅ Vec field — Task 5 (test_option_and_vec_fields)
- ✅ Serde path untouched — no modifications to ser.rs / serde.rs

**Type consistency:**
- `hessian_to_vec` / `hessian_to_writer` match the signatures used in all tests ✅
- Generated code paths `::hessian2o3::HessianSerialize`, `::hessian2o3::encode::Context`, `::hessian2o3::encode::begin_object` — all made public in Task 2 ✅
- `begin_object(w, ctx, class, &[field_names])` — matches `encode.rs:394` signature `fn begin_object<W, S>(w: &mut W, ctx: &mut Context, class: &str, fields: &[S]) where S: AsRef<str>` ✅
