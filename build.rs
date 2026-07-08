extern crate string_cache_codegen;

use std::env;
use std::path::Path;

fn main() {
    string_cache_codegen::AtomType::new("cachestr::Cachestr", "cachestr!")
        .atoms(&[
            // lists
            "java.util.ArrayList",
            "java.util.LinkedList",
            "java.util.LinkedHashSet",
            // maps
            "java.util.HashMap",
            "java.util.LinkedHashMap",
            "java.util.TreeMap",
            "java.util.concurrent.ConcurrentHashMap",
            // common fields
            "id",
            "uuid",
            "name",
            "username",
            "age",
            "account",
            "password",
            "email",
            "address",
            "nickname",
            "phone",
            "avatar",
            "gender",
            "createTime",
            "createdTime",
            "updateTime",
            "updatedTime",
            "startTime",
            "endTime",
            "birth",
            "status",
            "userId",
            "orderId",
            "amount",
            "type",
            "price",
            "description",
            "version",
            // others
            "java.math.BigDecimal",
        ])
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("cachestr.rs"))
        .unwrap();
}
