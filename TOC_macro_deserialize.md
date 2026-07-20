# Macro `deserialize`

## 设计目标

创建一个宏`deserialize`, 代码放置在`./src/macros/deserialize.rs`中, 实现自动根据目标泛型`T`的类型来选择适合的反序列化方式, 具体规则遵循:
1. 如果`T`实现了`HDeserialize`, 那么调用`hessian2::hessian::hessian_from_reader`
2. 如果`T`实现了`Deserialize`, 那么调用`hessian2::serde::from_reader`
3. 如果`T`为`hessian::Value`, 那么调用`hessian2::de::Deserializer`的`read_value`

## 样例
```rust

// ── Section 1: struct implement HDeserialize ──────────────────────────────────────────────
#[derive(HessianSerialize, Debug, PartialEq)]
#[hessian(class = "com.example.User")]
struct User {
    id: i64,
    name: String,
    age: i32,
}

#[derive(Deserialize)]
struct SimpleUser {
    id: i64,
    name: String,
    age: i32,
}

fn main() {
    let dst:User = deserialize!(r)?; // 底层使用`hessian2::hessian::hessian_from_reader`
    let dst:SimpleUser = deserialize!(r)?; // 底层使用`hessian2::serde::from_reader`

    let dst:Value = deserialize!(r)?; // 底层使用`hessian2::de::Deserializer`的`read_value`


}


```