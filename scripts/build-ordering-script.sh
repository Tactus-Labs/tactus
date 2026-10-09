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
cargo build --locked --release --target riscv64imac-unknown-none-elf \
  -p tactus-o1-ordering-script -p tactus-o1-head-lock -p tactus-o1-anchor-script

LLD="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-lld"
mkdir -p artifacts
for name in tactus_o1_ordering_script tactus_o1_head_lock tactus_o1_anchor_script; do
  "$LLD" -flavor gnu \
    -o "artifacts/$name.elf" \
    "target/riscv64imac-unknown-none-elf/release/lib$name.a" \
    --entry _start -nostdlib --gc-sections --strip-all \
    -T scripts/ordering-script.ld -z max-page-size=0x1000 -z separate-code
  sha256sum "artifacts/$name.elf"
done
