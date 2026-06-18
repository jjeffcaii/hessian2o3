extern crate string_cache_codegen;

use std::env;
use std::path::Path;

fn main() {
    string_cache_codegen::AtomType::new("cachestr::Cachestr", "cachestr!")
        .atoms(&["java.math.BigDecimal"])
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("cachestr.rs"))
        .unwrap();
}
