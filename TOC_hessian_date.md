# Hessian Date Type

## API Design

```rust
#[derive(Serialize, Deserialize)]
pub struct Employee {
    pub name: String,
    
    // following rules below:
    // - serialize to hessian Date format, see also `PrimitiveValue::Date(unix_millis)`
    // - deserialize from hessian Date
    #[serde(with = "hessian2::date")]
    pub created_at: i64,
}

```
