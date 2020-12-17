# example-analyzer

## Setup

```bash
git clone -b linked-examples https://github.com/willcrichton/rust
cd rust
./x.py build
# replace `apple-darwin` with your current target as appropriate
rustup toolchain link custom-rustdoc build/x86_64-apple-darwin/stage1
cd ..
git clone https://github.com/willcrichton/example-analyzer
cd example-analyzer
cargo build
```

## Example

```bash
# NOTE: the directory you run this from is important since the project uses
# `rust-toolchain`
# On MacOS, use `DYLD_LIBRARY_PATH` instead.
export LD_LIBRARY_PATH=$(rustc --print sysroot)/lib
cd doctest
../target/debug/example-analyzer
cargo +custom-rustdoc rustdoc --open -- -Z unstable-options \
    --repository-url $(git remote get-url origin)/tree/$(git rev-parse HEAD) \
    --call-locations .call_locations.json
```

## Development

If you change the Rust repo (i.e. rustdoc) then run:

```
(cd ../../rust && ./x.py build)
# also re-run `cargo rustdoc`
```

If you change example-analyzer then run:

```
(cd .. && cargo build)
../target/debug/example-analyzer
# also re-run `cargo rustdoc`
```
