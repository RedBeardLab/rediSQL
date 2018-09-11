extern crate bindgen;

use bindgen::callbacks::{IntKind, ParseCallbacks};

extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=sqlite_dependencies.h");
    println!("cargo:rerun-if-changed=redis_dependencies.h");

    cc::Build::new()
        .file("src/CDeps/Redis/redismodule.c")
        .include("src/CDeps/Redis/include")
        .compile("libredismodule.a");

    cc::Build::new()
        .file("src/CDeps/SQLite/sqlite3.c")
        .include("src/CDeps/SQLite/include")
        .define("HAVE_USLEEP", Some("1"))
        .define("NDEBUG", Some("1"))
        .define("HAVE_FDATASYNC", Some("1"))
        .define("SQLITE_THREADSAFE", Some("2"))
        .define("SQLITE_ENABLE_JSON1", Some("1"))
        .define("SQLITE_ENABLE_FTS3", Some("1"))
        .define("SQLITE_ENABLE_FTS4", Some("1"))
        .define("SQLITE_ENABLE_FTS5", Some("1"))
        .define("SQLITE_ENABLE_RTREE", Some("1"))
        .compile("libsqlite3.a");

    #[derive(Debug)]
    struct SqliteTypeChooser;

    impl ParseCallbacks for SqliteTypeChooser {
        fn int_macro(
            &self,
            _name: &str,
            value: i64,
        ) -> Option<IntKind> {
            if value >= i32::min_value() as i64
                && value <= i32::max_value() as i64
            {
                Some(IntKind::I32)
            } else {
                None
            }
        }
    }

    let engine_pro = "-DENGINE_PRO=1";
    //let engine_pro = "-DENGINE_PRO=0";

    let bindings = bindgen::Builder::default()
            .parse_callbacks(Box::new(SqliteTypeChooser))
            // .rustfmt_bindings(false)
            .header("sqlite_dependencies.h")
            .clang_arg(engine_pro)
            .generate()
            .expect("Unable to generate bindings for SQLite");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings_sqlite.rs"))
        .expect("Couldn't write bindings for SQLite!");

    let bindings = bindgen::Builder::default()
            .parse_callbacks(Box::new(SqliteTypeChooser))
            // .rustfmt_bindings(false) // see https://github.com/rust-lang-nursery/rust-bindgen/issues/1306#event-1597477817
            .header("redis_dependencies.h")
            .generate()
            .expect("Unable to generate bindings for Redis");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings_redis.rs"))
        .expect("Couldn't write bindings for Redis!");
}
