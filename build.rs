extern crate string_cache_codegen;

use std::env;
use std::path::Path;

fn main() {
    string_cache_codegen::AtomType::new("cachestr::Cachestr", "cachestr!")
        .atoms(&[
            // COMMON FIELDS
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
            // ARRAY
            "[B",                   // byte[],
            "[S",                   // short[],
            "[I",                   // int[]
            "[J",                   // long[]
            "[F",                   // float[]
            "[D",                   // double[]
            "[C",                   // char[]
            "[Z",                   // boolean[]
            "[Ljava.lang.Byte;",    // Byte[]
            "[Ljava.lang.Short;",   // Short[]
            "[Ljava.lang.Integer;", // Integer[]
            "[Ljava.lang.Long;",    // Long[]
            "[Ljava.lang.Float;",   // Float[]
            "[Ljava.lang.Double;",  // Double[]
            "[Ljava.lang.String;",  // String[]
            "[Ljava.lang.Object;",  // Object[]
            // COMMON CLASSES
            "byte",
            "short",
            "int",
            "long",
            "float",
            "double",
            "java.lang.Byte",
            "java.lang.Short",
            "java.lang.Integer",
            "java.lang.Long",
            "java.lang.Float",
            "java.lang.Double",
            "java.lang.String",
            "java.lang.Object",
            "java.util.Collection",
            "java.util.List",
            "java.util.Set",
            "java.util.Map",
            "java.util.Date",
            "java.math.BigDecimal",
            // LISTS
            "java.util.ArrayList",
            "java.util.LinkedList",
            "java.util.LinkedHashSet",
            // MAPS
            "java.util.HashMap",
            "java.util.LinkedHashMap",
            "java.util.TreeMap",
            "java.util.concurrent.ConcurrentHashMap",
            // COMMON VALUES
            "0",
            "1",
            "-1",
            "true",
            "false",
        ])
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("cachestr.rs"))
        .unwrap();
}
