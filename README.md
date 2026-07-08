# Hessian2

A Rust implementation of the [Hessian 2.0 Serialization Protocol](http://hessian.caucho.com/doc/hessian-serialization.html), commonly used for Java/Dubbo RPC interop.

> **:warning: Warning: This project is a work in progress and not ready for production use.**

## Features

- **Encoding** — serialize Rust values to Hessian 2.0 binary format
- **Decoding** — deserialize Hessian 2.0 binary data into a dynamic `Value` type
- **serde integration** — encode/decode any `serde::Serialize` / `serde::Deserialize` type via `to_vec` / `to_writer` / `from_slice` / `from_reader`
- **`#[derive(Hessian)]`** — auto-implement `HessianSerialize` / `HessianDeserialize` for structs mapped to Java classes
- **`hessian!` macro** — build a `Value` (map, list, object, or scalar) from a JSON-like literal, similar to `serde_json::json!`

## QuickStart

Add to `Cargo.toml`:

```toml
[dependencies]
hessian2 = { path = "." }
```

### Encoding with serde

```rust
use hessian2::to_vec;
use serde::Serialize;

#[derive(Serialize)]
struct Point { x: i32, y: i32 }

let bytes = to_vec(&Point { x: 1, y: 2 })?;
```

### Decoding with serde

```rust
use hessian2::from_slice;
use serde::Deserialize;

#[derive(Deserialize)]
struct Point { x: i32, y: i32 }

let point: Point = from_slice(&bytes)?;
```

### Building a `Value` with the `hessian!` macro

```rust
use hessian2::hessian;

// map
let user = hessian!({
    "id": 123,
    "name": "Jerry",
    "age": 18,
});

// a "$class" entry turns the literal into a Value::Object instead of a Value::Map
let user_obj = hessian!({
    "$class": "com.example.User",
    "id": 123,
    "name": "Jerry",
    "age": 18,
});

// lists, scalars, null, and variables all work too
let list = hessian!([1, "two", [3, 4], null]);
let age = 18;
let v = hessian!(age);
```

### Encoding a Java object with `#[derive(Hessian)]`

```rust
use hessian2::{Hessian, hessian_to_vec};

#[derive(Hessian)]
#[hessian(class = "com.example.Point")]
struct Point {
    x: i32,
    #[hessian(rename = "yCoord")]
    y: i32,
}

let bytes = hessian_to_vec(&Point { x: 1, y: 2 })?;
```

### Decoding a Java object with `#[derive(Hessian)]`

```rust
use hessian2::{Hessian, hessian_from_slice};

#[derive(Hessian)]
#[hessian(class = "com.example.Point")]
struct Point {
    x: i32,
    #[hessian(rename = "yCoord")]
    y: i32,
}

let point: Point = hessian_from_slice(&bytes)?;
```

### Decoding into `Value`

```rust
use hessian2::codec::{get_value, Context};

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
