use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i32),
    Long(i64),
    Double(f64),
    Date(SystemTime),
    List(Vec<Value>),
    Binary(Vec<u8>),
    String(String),
    Map(HashMap<String, Value>),
}
