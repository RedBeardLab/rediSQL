# Buil the project on your machine

Cargo is the default manager to build rust project and this particular project is not an exception.

## Bindgen

Normally no other dependencies are required outside Cargo itself, however, we use [`bindgen`][bindgen] to automatically create the FFI binds between the C code from Redis and SQLite and the Rust code.

At the moment we have decided to keep generating the bindings during compilation, this is not strictly necessary, but at the moment it seems the best alternative.

`bindgen` has as dependencies: `llvm` and `clang` that need to be installed in your machine.
Please refer to the `bindgen` [documentation][bindgen_dependencies] in order to understand how to install the dependencies, it is quite easy in modern systems.

## GLIBC

Rust usually try to link statically as much as possible, however, leave libc dynamically linked.

Usually, it is not a problem, but this requires that the machine where the object will execute has at least the version of glibc assumed during compilation.

If your servers are running relatively modern operative systems this problem will never affect you, however, if you are running more "battle tested" OSs, then you may encounter these difficulties.

We decided to support GLIBC 2.14 that should cover the vast majority of used operative systems with the notable exception of CentOS 6, if there is enough request for CentOS 6 we will drop to GLIBC 2.12

If you are building the module yourself, and if you need a lower GLIBC version than the one automatically used by cargo you can use [crosstool-ng][x-tool]:

1. Configure your toolchain: `./ct-ng menuconfig`

Most likely you will need to select `Use obsolete features` under `Paths and misc options` and then choose the glibc version you need under `c-library`

2. Build your toolchain (it will take a while ~30min/1 hour): `./ct-ng build`

3. Configure the rustc linker in such a way to use the one you have just build.

In my case it looks a little bit like this:

```
RUSTFLAGS="-C linker=~/x-tools/x86_64-unknown-linux-gnu/bin/x86_64-unknown-linux-gnu-gcc" cargo build --release 
```

Beware that your path could be a little different.

## Conclusion

In the general case, it is quite simple to build RediSQL, however specific cases need to be addressed with more attention.

Please, if you find any issues following this instruction we would really appreciate you reporting them via github issues or pull requests. 


[bindgen]: https://rust-lang-nursery.github.io/rust-bindgen/introduction.html
[bindgen_dependencies]: https://rust-lang-nursery.github.io/rust-bindgen/requirements.html
[x-tool]: https://crosstool-ng.github.io/
