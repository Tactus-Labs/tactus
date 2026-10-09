#!/usr/bin/env bash
# Run against an isolated, funded, loopback-only chain. Never touches an existing node.
set -euo pipefail
cd "$(dirname "$0")/.."
CKB_BIN="${CKB_BIN:-ckb}"
CKB_BIN="$(command -v "$CKB_BIN")"
required_version="${TACTUS_CKB_VERSION:-0.121.0}"
if [[ "$required_version" != 0.121.0 && "$required_version" != 0.210.0 ]]; then
  echo 'Supported laboratory versions: 0.121.0, 0.210.0.' >&2
  exit 1
fi
if [[ "$($CKB_BIN --version)" != "ckb $required_version "* ]]; then
  echo "Experiments require CKB $required_version; set CKB_BIN to that binary." >&2
  exit 1
fi
suite="${TACTUS_DEVNET_SUITE:-replay-a123}"
case "$suite" in replay-a123|replay-batch|replay-evm|replay-priority|replay-sealed|replay-admission|replay-network|replay-load) ;; *) echo 'Unknown devnet suite' >&2; exit 1 ;; esac
cargo build --locked --bin "$suite"
if [[ "$suite" == replay-evm || "$suite" == replay-network ]]; then
  cargo build --locked --bin recover-execution
fi
if [[ "$suite" == replay-sealed || "$suite" == replay-network ]]; then
  cargo build --locked --bin recover-sealed
fi
bash scripts/build-ordering-script.sh
mkdir -p artifacts
run_dir="$(mktemp -d "$PWD/artifacts/${suite#replay-}-XXXXXXXX")"
export TACTUS_CKB_RPC_ADDR="127.0.0.1:${TACTUS_DEVNET_RPC_PORT:-18714}"
export TACTUS_PEER_RPC_ADDR="127.0.0.1:${TACTUS_PEER_RPC_PORT:-18716}"
export TACTUS_DEVNET_AUTOMINE=1
export TACTUS_EVIDENCE_PATH="$run_dir/evidence.json"
export TACTUS_RUN_DIR="$run_dir"
export TACTUS_CKB_BIN="$CKB_BIN"
export TACTUS_DEVNET_SUITE="$suite"
python3 - <<'PY'
import os,socket
ports=[int(os.environ['TACTUS_CKB_RPC_ADDR'].split(':')[1]),int(os.getenv('TACTUS_DEVNET_P2P_PORT','18715'))]
if os.environ['TACTUS_DEVNET_SUITE']=='replay-network':ports.extend([int(os.environ['TACTUS_PEER_RPC_ADDR'].split(':')[1]),int(os.getenv('TACTUS_PEER_P2P_PORT','18717'))])
assert len(set(ports))==len(ports), 'Node ports must be distinct'
for port in ports:
 s=socket.socket();s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
 try:s.bind(('127.0.0.1',port));s.listen(1)
 except OSError:raise SystemExit(f'Port {port} is occupied; refusing to touch an existing node')
 finally:s.close()

PY
"$CKB_BIN" init -C "$run_dir/node" --chain dev --rpc-port "${TACTUS_DEVNET_RPC_PORT:-18714}" \
  --p2p-port "${TACTUS_DEVNET_P2P_PORT:-18715}" \
  --ba-arg 0xc155c0113355a061173d1ff21075ec37754ec1ca --genesis-message tactus-o1-experiment-a-v1 > "$run_dir/init.log"
python3 - <<'PY'
import os,pathlib,hashlib,json,subprocess,shutil
root=pathlib.Path(os.environ['TACTUS_RUN_DIR']);p=root/'node/ckb.toml'
s=p.read_text().replace('"Experiment", "Debug"','"Experiment", "Debug", "IntegrationTest"').replace('/ip4/0.0.0.0/','/ip4/127.0.0.1/')
assert '"IntegrationTest"' in s
p.write_text(s)
p=root/'node/specs/dev.toml';s=p.read_text();i=s.index('[params]')
s=s[:i]+'''[[genesis.issued_cells]]
capacity = 20000000000000000
lock.code_hash = "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8"
lock.args = "0xc155c0113355a061173d1ff21075ec37754ec1ca"
lock.hash_type = "type"

'''+s[i:];p.write_text(s)
if os.environ['TACTUS_DEVNET_SUITE']=='replay-network':
 shutil.copytree(root/'node',root/'peer')
 peer=root/'peer/ckb.toml'
 peer.write_text(peer.read_text().replace(os.environ['TACTUS_CKB_RPC_ADDR'],os.environ['TACTUS_PEER_RPC_ADDR']).replace('/tcp/'+os.getenv('TACTUS_DEVNET_P2P_PORT','18715')+'"','/tcp/'+os.getenv('TACTUS_PEER_P2P_PORT','18717')+'"'))
manifest={'node_version':subprocess.check_output([os.environ['TACTUS_CKB_BIN'],'--version'],text=True).strip(),
 'rustc':subprocess.check_output(['rustc','-Vv'],text=True),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 'git_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD'])).hexdigest(),
 'files':{}}
for name in ['Cargo.lock','scripts/build-ordering-script.sh','scripts/ordering-script.ld','artifacts/tactus_o1_ordering_script.elf','artifacts/tactus_o1_head_lock.elf','artifacts/tactus_o1_anchor_script.elf','artifacts/tactus_o1_priority_script.elf','artifacts/tactus_o1_sealed_script.elf',str(root/'node/ckb.toml'),str(p),os.environ['TACTUS_CKB_BIN']]:
 manifest['files'][name]=hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest()
binaries=['target/debug/'+os.environ['TACTUS_DEVNET_SUITE']]
if os.environ['TACTUS_DEVNET_SUITE'] in ('replay-evm','replay-network'):binaries.append('target/debug/recover-execution')
if os.environ['TACTUS_DEVNET_SUITE'] in ('replay-sealed','replay-network'):binaries.append('target/debug/recover-sealed')
if os.environ['TACTUS_DEVNET_SUITE']=='replay-network':binaries.extend([str(root/'peer/ckb.toml'),str(root/'peer/specs/dev.toml')])
for name in binaries:manifest['files'][name]=hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest()

paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],text=True).split('\0')
manifest['source_files']={name:hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest()
 for name in sorted(set(paths)) if name and pathlib.Path(name).is_file()
 and (name.startswith(('crates/','scripts/','.github/','.cargo/','specs/test-vectors/')) or name in ('Cargo.toml','Cargo.lock','rust-toolchain.toml'))}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
PY
"$CKB_BIN" run -C "$run_dir/node" --indexer > "$run_dir/node.log" 2>&1 &
node_pid=$!
peer_pid=""
cleanup() {
  kill "$node_pid" 2>/dev/null || true
  if [[ -n "$peer_pid" ]]; then kill "$peer_pid" 2>/dev/null || true; wait "$peer_pid" 2>/dev/null || true; fi
  wait "$node_pid" 2>/dev/null || true
}
trap cleanup EXIT
if [[ "$suite" == replay-network ]]; then
  "$CKB_BIN" run -C "$run_dir/peer" --indexer > "$run_dir/peer.log" 2>&1 &
  peer_pid=$!
fi
python3 - <<'PY'
import os,urllib.request,json,time
addresses=[os.environ['TACTUS_CKB_RPC_ADDR']]
if os.environ['TACTUS_DEVNET_SUITE']=='replay-network':addresses.append(os.environ['TACTUS_PEER_RPC_ADDR'])
for address in addresses:
 url='http://'+address
 for _ in range(150):
  try:
   req=urllib.request.Request(url,json.dumps({'jsonrpc':'2.0','id':1,'method':'get_tip_block_number','params':[]}).encode(),{'Content-Type':'application/json'})
   if json.load(urllib.request.urlopen(req,timeout=1))['result']=='0x0':break
  except (OSError,KeyError):pass
  time.sleep(.1)
 else:raise SystemExit('Isolated CKB node did not start at genesis: '+address)

PY
printf 'Evidence directory: %s\n' "$run_dir"
"target/debug/$suite" 2>&1 | tee "$run_dir/replay.log"
python3 scripts/summarize-experiments.py "$run_dir/evidence.json" "$run_dir/summary.json"
printf 'Complete: %s\n' "$run_dir/summary.json"
