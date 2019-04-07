extern crate cc;

fn main() {
    println!("cargo:rerun-if-changed=src/CDeps");
    println!("cargo:rerun-if-changed=\"build.rs\"");

    cc::Build::new()
        .file("src/CDeps/sqlite_dump.c")
        .include("src/CDeps")
        .compile("libsqlite_dump.a");
}
