#!/bin/sh
cargo build --release --target=wasm32-wasi

deno run --allow-all --unstable ./run.ts
