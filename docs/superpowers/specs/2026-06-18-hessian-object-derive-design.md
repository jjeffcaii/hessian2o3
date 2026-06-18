# Hessian Object Derive Macro — Design Spec

**Date:** 2026-06-18  
**Status:** Approved

## Goal

Implement a procedural macro `#[derive(HessianObject)]` that automatically generates Hessian 2.0 object serialization for Rust structs mapped to Java classes. The generated code uses the full Hessian object encoding format (`C`/`O` bytes) with shared `Context` for class-ref deduplication across nested objects.

## User-Facing API

```rust
#[derive(HessianObject)]
#[hessian(class = "com.hessian2o3.User")]
pub struct User {
    #[hessian(rename = "id")]
    pub id: i64,
    #[hessian(rename = "name")]
    pub name: String,
    pub age: i32,   // no rename → uses Rust field name "age"
}

// Top-level serialization
let bytes = hessian2o3::hessian_to_vec(&user)?;
let mut w = vec![];
hessian2o3::hessian_to_writer(&mut w, &user)?;
```

## Architecture

### Workspace Layout

```
hessian2o3/                   ← workspace root (also the main lib crate)
├── Cargo.toml                ← [workspace] + [package] coexist
├── src/
│   ├── lib.rs                ← adds `pub mod hessian;`, re-exports HessianObject derive + API
│   ├── encode.rs             ← existing (Context changed from pub(crate) → pub)
│   ├── hessian.rs            ← new: HessianSerialize trait + primitive impls + hessian_to_*
│   ├── ser.rs                ← unchanged
│   ├── serde.rs              ← unchanged
│   └── …
└── hessian2o3-derive/
    ├── Cargo.toml            ← proc-macro = true
    └── src/
        └── lib.rs            ← HessianObject derive macro implementation
```

The main crate depends on the derive crate via `hessian2o3-derive = { path = "hessian2o3-derive" }` and re-exports the macro at the crate root.

## HessianSerialize Trait

Defined in `src/hessian.rs`:

```rust
pub trait HessianSerialize {
    fn hessian_serialize<W: std::io::Write>(
        &self,
        w: &mut W,
        ctx: &mut crate::encode::Context,
    ) -> std::io::Result<()>;
}
```

### Primitive Blanket Impls

| Rust type | Encoding call |
|---|---|
| `i8`, `i16`, `i32`, `u8`, `u16` | `encode::put_i32` |
| `i64`, `u32`, `u64` | `encode::put_i64` |
| `f32`, `f64` | `encode::put_f64` |
| `bool` | `encode::put_bool` |
| `String`, `&str` | `encode::put_str` |
| `Vec<u8>`, `&[u8]` | `encode::put_bytes` |
| `Option<T: HessianSerialize>` | `None → put_null`, `Some → recurse` |
| `Vec<T: HessianSerialize>` | `begin_list(None, len)` + element recursion |

### Top-Level Functions

```rust
pub fn hessian_to_writer<W: std::io::Write, T: HessianSerialize>(
    writer: &mut W,
    value: &T,
) -> crate::Result<()> {
    let mut ctx = crate::encode::Context::default();
    value.hessian_serialize(writer, &mut ctx).map_err(crate::Error::IO)
}

pub fn hessian_to_vec<T: HessianSerialize>(value: &T) -> crate::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(128);
    hessian_to_writer(&mut buf, value)?;
    Ok(buf)
}
```

## Proc-Macro Design

### Attributes

- `#[hessian(class = "com.example.Foo")]` on the struct — required; Java fully-qualified class name.
- `#[hessian(rename = "javaName")]` on a field — optional; defaults to the Rust field name.

### Generated Code

For the `User` struct above, the macro generates:

```rust
impl hessian2o3::HessianSerialize for User {
    fn hessian_serialize<W: std::io::Write>(
        &self,
        w: &mut W,
        ctx: &mut hessian2o3::encode::Context,
    ) -> std::io::Result<()> {
        hessian2o3::encode::begin_object(
            w, ctx,
            "com.hessian2o3.User",
            &["id", "name", "age"],  // static &[&str], compile-time known
        )?;
        hessian2o3::HessianSerialize::hessian_serialize(&self.id, w, ctx)?;
        hessian2o3::HessianSerialize::hessian_serialize(&self.name, w, ctx)?;
        hessian2o3::HessianSerialize::hessian_serialize(&self.age, w, ctx)?;
        Ok(())
    }
}
```

`begin_object` (already in `encode.rs`) handles Context lookup internally: first occurrence writes the class definition (`C` + class name + field count + field names), subsequent occurrences write a back-reference (`0x60+i` or `O`+int).

### Compile-Time Errors

| Condition | Error |
|---|---|
| Missing `#[hessian(class = "...")]` | `compile_error!` |
| Applied to tuple struct or enum | `compile_error!` |

## Context Sharing

`encode::Context` tracks `class_refs: SmallVec<[Cachestr; 16]>`. It is created once in `hessian_to_writer` and passed by `&mut` through every recursive `hessian_serialize` call. Nested objects of the same Java class therefore share a single class-ref table, so the class definition is written only once per message.

## Coexistence with serde Path

The existing `to_writer` / `to_vec` (serde-based) are untouched. The new `hessian_to_writer` / `hessian_to_vec` are separate entry points. A struct may implement both `serde::Serialize` (via `#[derive(Serialize)]`) and `HessianSerialize` (via `#[derive(HessianObject)]`) independently.

## Testing Strategy

All tests live in `src/hessian.rs` under `#[cfg(test)]`:

1. **Single struct** — serialize a `User`; compare hex output against the expected bytes from `encode::tests::test_object`.
2. **Nested struct with class-ref reuse** — `User` contains two `Address` fields; assert the `Address` class definition appears exactly once in the output.
3. **Primitive coverage** — at least one test per blanket impl type.
4. **Option fields** — `None` → `4e`, `Some(x)` → normal encoding.
5. **Vec field** — verify list header + element encoding.
