# Fuzz targets

These targets are isolated from the stable production workspace because
`cargo-fuzz` requires a nightly Rust compiler and LLVM sanitizer support.

From the repository root, run bounded campaigns with:

```sh
cargo +nightly fuzz run json_input fuzz/seeds/json_input -- -runs=10000
cargo +nightly fuzz run small_instances fuzz/seeds/small_instances -- -runs=10000
```

`json_input` exercises syntax errors, DTO field-path diagnostics, and exact
input conversion. `small_instances` derives at most four containers and six
items from each byte slice, runs the selected one-work-unit portfolio, and
independently validates and serializes its result.
