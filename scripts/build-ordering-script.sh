#!/usr/bin/env bash
# Builds the OrderingHead type script ELF for CKB-VM.
#
# The Rust→CKB-VM chain: host-tested validation core (cargo test) →
# riscv64imac staticlib → rust-lld static ELF with _start entry.
# Note: cdylib is dropped by recent rustc on none-elf targets, so the
# final ELF is linked directly from the staticlib archive.
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add riscv64imac-unknown-none-elf
cargo build --release --target riscv64imac-unknown-none-elf -p tactus-ordering-script

LLD="$(find "$(rustc --print sysroot)/lib/rustlib" -name rust-lld | head -1)"
mkdir -p artifacts
"$LLD" -flavor gnu \
  -o artifacts/tactus_ordering_script.elf \
  target/riscv64imac-unknown-none-elf/release/libtactus_ordering_script.a \
  --entry _start -nostdlib -n --gc-sections --strip-all

ls -la artifacts/tactus_ordering_script.elf
