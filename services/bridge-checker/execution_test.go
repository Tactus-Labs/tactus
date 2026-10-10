package bridgechecker

// Independent Geth transitions for candidate v2 batches. This does not prove CKB
// publication authenticity, validate the zkVM, or authorize a vault payout.
import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/json"
	"math/big"
	"os"
	"reflect"
	"testing"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/hexutil"
	"github.com/ethereum/go-ethereum/core"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/tracing"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/core/vm/runtime"
	"github.com/ethereum/go-ethereum/params"
	"github.com/ethereum/go-ethereum/trie"
	"github.com/holiman/uint256"
)

type bridgeLog struct {
	Address common.Address
	Topics  []common.Hash
	Data    hexutil.Bytes
}
type bridgeAccount struct {
	Balance *hexutil.Big
	Nonce   uint64
	Code    hexutil.Bytes
	Storage map[string]string
}
type bridgeStep struct {
	Blocks []struct {
		Header       *types.Header
		Hash         common.Hash
		Transactions []hexutil.Bytes
		Receipts     []hexutil.Bytes
	}
	Deposits []struct {
		Record    string
		DepositID string `json:"deposit_id"`
		Calldata  hexutil.Bytes
		Gas       uint64 `json:"gas_used"`
		Logs      []bridgeLog
	}
}

func TestCandidateNativeExecutionAgainstGeth(t *testing.T) {
	path := os.Getenv("TACTUS_NATIVE_BRIDGE_VECTOR")
	if path == "" {
		path = "../../specs/evidence/native-bridge-execution/candidate.json"
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var input struct {
		Config      string
		Publication bool `json:"authenticated_publication"`
		Settled     bool `json:"proof_settled"`
		Release     bool `json:"custody_release"`
		Production  bool `json:"production_ready"`
		Cases       []struct {
			Name    string
			Genesis struct {
				Accounts map[common.Address]bridgeAccount
			}
			Header *types.Header `json:"genesis_header"`
			Steps  []bridgeStep
		}
	}
	if err = json.Unmarshal(data, &input); err != nil {
		t.Fatal(err)
	}
	if input.Publication || input.Settled || input.Release || input.Production || len(input.Cases) != 2 {
		t.Fatal("candidate scope/cases")
	}
	config := common.FromHex(input.Config)
	if len(config) != 164 {
		t.Fatal("config length")
	}
	address := common.BytesToAddress(config[112:132])
	chain := *params.AllDevChainProtocolChanges
	chain.ChainID = new(big.Int).SetUint64(binary.LittleEndian.Uint64(config[104:112]))
	chain.CancunTime = nil
	chain.PragueTime = nil
	chain.OsakaTime = nil
	chain.BogotaTime = nil
	chain.AmsterdamTime = nil
	chain.UBTTime = nil
	var rows []map[string]any
	deposits, transactions, reverts := 0, 0, 0
	for _, c := range input.Cases {
		db, err := state.New(types.EmptyRootHash, state.NewDatabaseForTesting())
		if err != nil {
			t.Fatal(err)
		}
		for a, v := range c.Genesis.Accounts {
			db.SetBalance(a, uint256.MustFromBig((*big.Int)(v.Balance)), tracing.BalanceChangeUnspecified)
			db.SetNonce(a, v.Nonce, tracing.NonceChangeUnspecified)
			db.SetCode(a, v.Code, tracing.CodeChangeUnspecified)
			for k, value := range v.Storage {
				db.SetState(a, common.HexToHash(k), common.HexToHash(value))
			}
		}
		if db.IntermediateRoot(chain.Rules(c.Header.Number, true, c.Header.Time)) != c.Header.Root {
			t.Fatal("genesis root")
		}
		hashes := map[uint64]common.Hash{0: c.Header.Hash()}
		parent := c.Header
		for _, step := range c.Steps {
			if len(step.Blocks) != 1 {
				t.Fatal("fixture block count")
			}
			block := step.Blocks[0]
			h := block.Header
			if h.ParentHash != parent.Hash() || h.Number.Uint64() != parent.Number.Uint64()+1 {
				t.Fatal("header succession")
			}
			rules := chain.Rules(h.Number, true, h.Time)
			if !rules.IsShanghai || rules.IsCancun || rules.IsPrague || rules.IsOsaka || rules.IsAmsterdam || rules.IsBogota {
				t.Fatal("fork rules")
			}
			for _, d := range step.Deposits {
				raw := common.FromHex(d.Record)
				if len(raw) != 124 {
					t.Fatal("deposit wire")
				}
				// Reconstruct ABI independently, using the transcript's unique ID.
				calldata := callData("creditDeposit(bytes32,address,uint64)", common.FromHex(d.DepositID), raw[16:36], word(binary.LittleEndian.Uint64(raw[36:44])))
				if !bytes.Equal(calldata, d.Calldata) {
					t.Fatal("credit calldata")
				}
				id := common.HexToHash(d.DepositID)
				db.SetTxContext(id, 0, 0)
				cfg := &runtime.Config{ChainConfig: &chain, State: db, Origin: common.Address{}, Coinbase: h.Coinbase,
					BlockNumber: h.Number, Time: h.Time, GasLimit: 30_000_000, BaseFee: h.BaseFee, Random: &h.MixDigest}
				_, left, err := runtime.Call(address, calldata, cfg)
				if err != nil || 30_000_000-left != d.Gas {
					t.Fatalf("%s deposit gas: %d != %d / %v", c.Name, 30_000_000-left, d.Gas, err)
				}
				logs := db.GetLogs(id, h.Number.Uint64(), h.Hash(), h.Time)
				if len(logs) != len(d.Logs) {
					t.Fatal("deposit logs length")
				}
				for i, l := range logs {
					if l.Address != d.Logs[i].Address || !reflect.DeepEqual(l.Topics, d.Logs[i].Topics) || !bytes.Equal(l.Data, d.Logs[i].Data) {
						t.Fatal("deposit logs")
					}
				}
				db.Finalise(rules)
				deposits++
			}
			blockCtx := vm.BlockContext{CanTransfer: core.CanTransfer, Transfer: core.Transfer, GetHash: func(n uint64) common.Hash { return hashes[n] },
				Coinbase: h.Coinbase, GasLimit: h.GasLimit, BlockNumber: h.Number, Time: h.Time, Difficulty: h.Difficulty, BaseFee: h.BaseFee, Random: &h.MixDigest}
			evm := vm.NewEVM(blockCtx, db, &chain, vm.Config{})
			gp := core.NewGasPool(h.GasLimit)
			var receipts types.Receipts
			var txs types.Transactions
			for i, raw := range block.Transactions {
				tx := new(types.Transaction)
				if err := tx.UnmarshalBinary(raw); err != nil {
					t.Fatal(err)
				}
				db.SetTxContext(tx.Hash(), i, uint32(i))
				receipt, _, err := core.ApplyTransaction(context.Background(), evm, gp, db, h, tx)
				if err != nil {
					t.Fatal(err)
				}
				encoded, err := receipt.MarshalBinary()
				if err != nil || !bytes.Equal(encoded, block.Receipts[i]) {
					t.Fatalf("receipt differs: %v", err)
				}
				if receipt.Status == 0 {
					reverts++
				}
				transactions++
				receipts = append(receipts, receipt)
				txs = append(txs, tx)
			}
			root := db.IntermediateRoot(rules)
			if root != h.Root || gp.CumulativeUsed() != h.GasUsed {
				t.Fatalf("%s state/gas root %s expected %s; gas %d != %d", c.Name, root, h.Root, gp.CumulativeUsed(), h.GasUsed)
			}
			if types.DeriveSha(txs, trie.NewStackTrie(nil)) != h.TxHash || types.DeriveSha(receipts, trie.NewStackTrie(nil)) != h.ReceiptHash || types.MergeBloom(receipts) != h.Bloom {
				t.Fatal("transaction/receipt roots or bloom")
			}
			if block.Hash != h.Hash() {
				t.Fatal("header hash")
			}
			rows = append(rows, map[string]any{"case": c.Name, "number": h.Number.Uint64(), "state_root": root, "gas_used": h.GasUsed, "transaction_root": h.TxHash, "receipt_root": h.ReceiptHash})
			hashes[h.Number.Uint64()] = h.Hash()
			parent = h
		}
	}
	if len(rows) != 7 || deposits != 5 || transactions != 5 || reverts != 1 {
		t.Fatalf("coverage: %d blocks %d deposits %d transactions %d reverts", len(rows), deposits, transactions, reverts)
	}
	if path := os.Getenv("TACTUS_NATIVE_BRIDGE_GETH_EXPORT"); path != "" {
		report := map[string]any{"geth": "1.17.8", "fork": "Shanghai", "blocks": rows, "deposits": deposits, "signed_transactions": transactions, "reverts": reverts, "authenticated_publication": false, "proof_settled": false, "custody_release": false, "production_ready": false}
		data, err := json.MarshalIndent(report, "", "  ")
		if err != nil {
			t.Fatal(err)
		}
		if err = os.WriteFile(path, append(data, '\n'), 0644); err != nil {
			t.Fatal(err)
		}
	}
}
