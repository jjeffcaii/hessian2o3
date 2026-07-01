# hessian2o3

A Rust implementation of the [Hessian 2.0 Serialization Protocol](http://hessian.caucho.com/doc/hessian-serialization.html), commonly used for Java/Dubbo RPC interop.

> **⚠ Warning: This project is a work in progress and not ready for production use.**

## Features

- **Encoding** — serialize Rust values to Hessian 2.0 binary format
- **Decoding** — deserialize Hessian 2.0 binary data into a dynamic `Value` type
- **serde integration** — encode any `serde::Serialize` type via `to_vec` / `to_writer`
- **`#[derive(Hessian)]`** — auto-implement `HessianSerialize` for structs mapped to Java classes

## QuickStart

Add to `Cargo.toml`:

```toml
[dependencies]
hessian2o3 = { path = "." }
```

### Encoding with serde

```rust
use hessian2o3::to_vec;
use serde::Serialize;

#[derive(Serialize)]
struct Point { x: i32, y: i32 }

let bytes = to_vec(&Point { x: 1, y: 2 })?;
```

### Encoding a Java object with `#[derive(Hessian)]`

```rust
use hessian2o3::{Hessian, hessian_to_vec};

#[derive(Hessian)]
#[hessian(class = "com.example.Point")]
struct Point {
    x: i32,
    #[hessian(rename = "yCoord")]
    y: i32,
}

let bytes = hessian_to_vec(&Point { x: 1, y: 2 })?;
```

### Decoding into `Value`

```rust
use hessian2o3::codec::{get_value, Context};

let data: &[u8] = &[ /* hessian bytes */ ];
let mut ctx = Context::default();
let value = get_value(&mut ctx, &mut &data[..])?;

// index into maps and lists
println!("{}", value["name"]);
println!("{}", value[0]);
```

## Supported types

| Rust type | Hessian type |
|---|---|
| `bool` | boolean |
| `i8` / `i16` / `i32` / `u8` / `u16` | int (compact) |
| `i64` / `u32` / `u64` | long (compact) |
| `f32` / `f64` | double (compact) |
| `String` / `&str` | string (chunked UTF-8) |
| `Vec<u8>` | binary (chunked) |
| `Option<T>` | null or T |
| `Vec<T>` | untyped fixed list |
| structs via `#[derive(Hessian)]` | Java object |

## Value type

`get_value` returns a `Value` enum that covers all Hessian types:

```rust
pub enum Value {
    Null,
    Primitive(PrimitiveValue),  // bool, int, long, double, date, binary, string
    List(List),
    Map(Map),
    Object(Object),             // Java object with class name and named fields
}
```

`Value` supports `Display`, `Debug`, `PartialEq`, and indexing by integer (lists) or string key (maps).

## Commands

```bash
cargo build
cargo test
cargo clippy
cargo fmt
```
