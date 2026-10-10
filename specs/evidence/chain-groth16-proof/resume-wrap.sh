#!/usr/bin/env bash
set -euo pipefail
cd /home/arthur/RustRoverProjects/tactus-o1
exec 9>artifacts/.proof-run.lock
flock -n 9
export GOMAXPROCS=2 GOMEMLIMIT=16GiB GOGC=25 SP1_CIRCUIT_MODE=release
unset WITHOUT_VK_VERIFICATION
root="$PWD/artifacts/chain-proof-mEeIJqcp"
/usr/bin/time -v artifacts/prover-host/target/release/wrap-witness \
 "$PWD/artifacts/prover-tools/circuits/groth16/v6.1.0" \
 "$root/retained-wrapped-witness.json" "$root/proof/public-values.bin" \
 0x00eb2677694b390a31db8883db49c1a828d73618173ce6f3e5790b7678958b14 \
 "$root/resumed-proof" > "$root/resume-wrap.txt" 2>&1
export RAYON_NUM_THREADS=4 TOKIO_WORKER_THREADS=4
artifacts/prover-host/target/release/chain-proof verify \
 proofs/sp1/target/elf-compilation/riscv64im-succinct-zkvm-elf/release/tactus-o1-sp1-guest \
 "$root/proving-input.json" "$root/resumed-proof" > "$root/resume-fresh-verification.txt" 2>&1
python3 - <<'PY'
import pathlib,json,hashlib
root=pathlib.Path('artifacts/chain-proof-mEeIJqcp');p=root/'resumed-proof'
source=json.loads((root/'proving-input.json').read_bytes());report=json.loads((p/'result.json').read_bytes())
assert 'Fresh process verified real proof and exact expected journal' in (root/'resume-fresh-verification.txt').read_text()
assert report['proof_generated'] is True and report['proof_kind']=='SP1 real Groth16'
assert report['guest_verifying_key']==source['guest_verifying_key']
assert (p/'public-values.bin').read_bytes()==bytes.fromhex(source['expected_journal_hex'][2:])
assert len(report['negative_controls'])==15
(p/'wrap-result.json').write_bytes((p/'result.json').read_bytes())
report.update(native_chain_replay_match=True,prefix_batches=source['prefix_batches'],interval_batches=len(source['batches'])-source['prefix_batches'],fresh_chain_verification=True,continuous_full_proof_success=False,chain_input_sha256=hashlib.sha256((root/'proving-input.json').read_bytes()).hexdigest(),fresh_verification_sha256=hashlib.sha256((root/'resume-fresh-verification.txt').read_bytes()).hexdigest())
(p/'result.json').write_text(json.dumps(report,indent=2)+'\n')
print('Retained real-domain proof completed and independently verified:',p)
PY
