#!/usr/bin/env bash
# Prove a retained canonical CKB export using the pinned, already built guest.
set -euo pipefail
task_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$task_root"
[[ $# == 1 ]] || { echo 'Usage: run-chain-proof.sh CANONICAL_INPUT_JSON' >&2; exit 1; }
input_file="$(realpath "$1")"
: "${SP1_GROTH16_CIRCUIT_PATH:?Set release Groth16 circuit parent directory}"
[[ -f "$SP1_GROTH16_CIRCUIT_PATH/v6.1.0/groth16_vk.bin" ]] || exit 1
mkdir -p artifacts
exec 9>artifacts/.proof-run.lock
flock -n 9 || { echo 'Another local proof experiment is running.' >&2; exit 1; }
run_dir="$(mktemp -d "$task_root/artifacts/chain-proof-XXXXXXXX")"
mkdir "$run_dir/tmp"
cp "$input_file" "$run_dir/proving-input.json"
export CARGO_TARGET_DIR="$task_root/artifacts/prover-host/target" CARGO_BUILD_JOBS=4
cargo +1.97.1 build --release --locked --manifest-path proofs/sp1/Cargo.toml -p tactus-o1-sp1-host --features native-gnark --bin chain-proof > "$run_dir/build.txt" 2>&1
host="$CARGO_TARGET_DIR/release/chain-proof"
guest="$task_root/proofs/sp1/target/elf-compilation/riscv64im-succinct-zkvm-elf/release/tactus-o1-sp1-guest"
[[ -f "$guest" ]] || { echo 'Build pinned proof guest first' >&2; exit 1; }
export RAYON_NUM_THREADS=4 TOKIO_WORKER_THREADS=4
export SP1_WORKER_NUM_CORE_WORKERS=1 SP1_WORKER_CORE_BUFFER_SIZE=1
export SP1_WORKER_NUM_SETUP_WORKERS=1 SP1_WORKER_SETUP_BUFFER_SIZE=1
export SP1_WORKER_NUM_PREPARE_REDUCE_WORKERS=1 SP1_WORKER_PREPARE_REDUCE_BUFFER_SIZE=1
export SP1_WORKER_NUM_RECURSION_EXECUTOR_WORKERS=1 SP1_WORKER_RECURSION_EXECUTOR_BUFFER_SIZE=1
export SP1_WORKER_NUM_RECURSION_PROVER_WORKERS=1 SP1_WORKER_RECURSION_PROVER_BUFFER_SIZE=1
export GOMAXPROCS=2 GOMEMLIMIT=16GiB GOGC=25 RUST_LOG=info SP1_CIRCUIT_MODE=release
export ELEMENT_THRESHOLD=50331648 HEIGHT_THRESHOLD=524288 SHARD_SIZE=1048576
export TMPDIR="$run_dir/tmp"
unset WITHOUT_VK_VERIFICATION SP1_DUMP
sha256sum "$host" "$guest" Cargo.lock proofs/sp1/Cargo.lock proofs/sp1/host/src/bin/chain-proof.rs "$run_dir/proving-input.json" "$SP1_GROTH16_CIRCUIT_PATH/v6.1.0/groth16_vk.bin" > "$run_dir/input-hashes.txt"
python3 - "$run_dir" <<'PY'
import os,json,pathlib,subprocess,sys
keys=['RAYON_NUM_THREADS','TOKIO_WORKER_THREADS','GOMAXPROCS','GOMEMLIMIT','GOGC','SP1_CIRCUIT_MODE','SP1_GROTH16_CIRCUIT_PATH','ELEMENT_THRESHOLD','HEIGHT_THRESHOLD','SHARD_SIZE','TMPDIR']
keys+=sorted(k for k in os.environ if k.startswith('SP1_WORKER_'))
pathlib.Path(sys.argv[1],'environment.json').write_text(json.dumps({'environment':{k:os.environ[k] for k in keys},'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'proof_completed':False},indent=2)+'\n')
PY
printf 'Canonical chain proof run: %s\n' "$run_dir"
/usr/bin/time -v "$host" prove-groth16 "$guest" "$run_dir/proving-input.json" "$run_dir/proof" > "$run_dir/prove.txt" 2>&1
"$host" verify "$guest" "$run_dir/proving-input.json" "$run_dir/proof" > "$run_dir/fresh-verification.txt" 2>&1
printf 'Verified real-domain proof: %s\n' "$run_dir/proof/result.json"
