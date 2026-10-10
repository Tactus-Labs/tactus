// Independent contract semantics only: VM calls here do not authenticate L1 deposits.
package bridgechecker

import (
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/core/vm/runtime"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/params"
	"math/big"
	"os"
	"testing"
)

func word(n uint64) []byte { return common.LeftPadBytes(new(big.Int).SetUint64(n).Bytes(), 32) }
func callData(sig string, args ...[]byte) []byte {
	data := append([]byte{}, crypto.Keccak256([]byte(sig))[:4]...)
	for _, arg := range args {
		data = append(data, common.LeftPadBytes(arg, 32)...)
	}
	return data
}
func artifact(t *testing.T) []byte {
	t.Helper()
	data, err := os.ReadFile("../../contracts/bridge/NativeCKB.json")
	if err != nil {
		t.Fatal(err)
	}
	var a struct {
		Contract struct {
			EVM struct {
				Bytecode struct {
					Object string `json:"object"`
				} `json:"bytecode"`
			} `json:"evm"`
		} `json:"contract"`
	}
	if err = json.Unmarshal(data, &a); err != nil {
		t.Fatal(err)
	}
	code, err := hex.DecodeString(a.Contract.EVM.Bytecode.Object)
	if err != nil {
		t.Fatal(err)
	}
	return code
}
func TestIndependentDepositBurnConservation(t *testing.T) {
	chain := *params.AllDevChainProtocolChanges
	chain.ChainID = big.NewInt(31337)
	chain.CancunTime = nil
	chain.PragueTime = nil
	chain.OsakaTime = nil
	chain.BogotaTime = nil
	chain.AmsterdamTime = nil
	chain.UBTTime = nil
	rules := chain.Rules(big.NewInt(1), true, 1)
	if !rules.IsShanghai || rules.IsCancun || rules.IsPrague || rules.IsOsaka || rules.IsAmsterdam || rules.IsBogota {
		t.Fatal("expected Shanghai-only EVM rules")
	}
	db, err := state.New(types.EmptyRootHash, state.NewDatabaseForTesting())
	if err != nil {
		t.Fatal(err)
	}
	owner := common.HexToAddress("0x1234")
	recipient := common.HexToAddress("0x5678")
	bridgeDomain := bytes.Repeat([]byte{0x44}, 32)
	cfg := &runtime.Config{ChainConfig: &chain, State: db, Origin: owner, GasLimit: 1_000_000, BaseFee: new(big.Int)}
	code, address, _, err := runtime.Create(append(artifact(t), bridgeDomain...), cfg)
	if err != nil || len(code) == 0 {
		t.Fatalf("constructor: %v", err)
	}
	var rows []map[string]any
	invoke := func(caller common.Address, sig string, want bool, args ...[]byte) []byte {
		t.Helper()
		cfg.Origin = caller
		out, remaining, err := runtime.Call(address, callData(sig, args...), cfg)
		if (err == nil) != want {
			t.Fatalf("%s: success=%t error=%v", sig, want, err)
		}
		rows = append(rows, map[string]any{"signature": sig, "caller": caller, "success": err == nil, "output": common.Bytes2Hex(out), "gas": cfg.GasLimit - remaining})
		return out
	}
	domain := crypto.Keccak256([]byte("TO1BRDG1"), bridgeDomain, word(31337), address[:])
	if !bytes.Equal(invoke(owner, "DOMAIN()", true), domain) {
		t.Fatal("constructor domain")
	}
	zero := common.Address{}
	invoke(owner, "creditDeposit(bytes32,address,uint64)", false, word(1), owner[:], word(1000))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", false, word(0), owner[:], word(1000))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", false, word(1), owner[:], word(0))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", false, word(1), zero[:], word(1000))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", false, word(1), address[:], word(1000))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", true, word(1), owner[:], word(1000))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", false, word(1), recipient[:], word(2000))
	invoke(zero, "creditDeposit(bytes32,address,uint64)", false, word(2), owner[:], word(^uint64(0)))
	invoke(owner, "transfer(address,uint256)", true, recipient[:], word(300))
	lockHash := bytes.Repeat([]byte{0x55}, 32)
	invoke(recipient, "withdraw(uint64,bytes32)", true, word(300), lockHash)
	invoke(recipient, "withdraw(uint64,bytes32)", false, word(1), lockHash)
	invoke(owner, "withdraw(uint64,bytes32)", true, word(700), lockHash)
	invoke(owner, "withdraw(uint64,bytes32)", false, word(1), lockHash)
	for i, ownerAmount := range []struct {
		owner  common.Address
		amount uint64
	}{{recipient, 300}, {owner, 700}} {
		id := uint64(i + 1)
		ordinal := make([]byte, 8)
		amount := make([]byte, 8)
		binary.BigEndian.PutUint64(ordinal, id)
		binary.BigEndian.PutUint64(amount, ownerAmount.amount)
		expected := crypto.Keccak256([]byte("TO1EXIT1"), domain, ordinal, amount, lockHash, ownerAmount.owner[:])
		actual := invoke(owner, "withdrawals(uint64)", true, word(id))
		if !bytes.Equal(expected, actual) {
			t.Fatal("withdrawal commitment")
		}
		slot := crypto.Keccak256Hash(word(id), word(7))
		if db.GetState(address, slot) != common.BytesToHash(expected) {
			t.Fatal("withdrawal storage slot")
		}
	}
	for sig, want := range map[string]uint64{"totalSupply()": 0, "cumulativeDeposited()": 1000, "cumulativeWithdrawn()": 1000, "withdrawalCount()": 2} {
		if !bytes.Equal(invoke(owner, sig, true), word(want)) {
			t.Fatal(sig)
		}
	}
	if path := os.Getenv("TACTUS_BRIDGE_GETH_EXPORT"); path != "" {
		report := map[string]any{"geth_version": "1.17.8", "evm_fork": "Shanghai", "contract": address, "runtime_keccak": crypto.Keccak256Hash(code), "domain": common.BytesToHash(domain), "calls": rows, "credited": 1000, "burnt": 1000, "supply": 0, "withdrawals": 2, "authenticated_l1_deposits": false, "custody_release": false, "production_ready": false}
		data, err := json.MarshalIndent(report, "", "  ")
		if err != nil {
			t.Fatal(err)
		}
		if err = os.WriteFile(path, append(data, '\n'), 0644); err != nil {
			t.Fatal(err)
		}
	}
}
