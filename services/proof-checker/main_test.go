package main

import (
	"encoding/json"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/hexutil"
	"math/big"
	"os"
	"testing"
)

func fixtures(t *testing.T, name string) []Input {
	t.Helper()
	data, err := os.ReadFile("../../specs/evidence/observer-state-proofs/" + name + ".json")
	if err != nil {
		t.Fatal(err)
	}
	var inputs []Input
	if err = json.Unmarshal(data, &inputs); err != nil {
		t.Fatal(err)
	}
	return inputs
}
func TestIndependentArchivedProofs(t *testing.T) {
	for name, expected := range map[string]Summary{"geth-fixture-proofs": {57, 18, 285, 5}, "state-proofs": {6, 3, 18, 0}} {
		var got Summary
		for _, input := range fixtures(t, name) {
			if err := check(input, &got); err != nil {
				t.Fatal(err)
			}
		}
		if got != expected {
			t.Fatalf("%s: got %+v want %+v", name, got, expected)
		}
	}
}
func TestAdversarialAccountProofs(t *testing.T) {
	changes := map[string]func(*Input){
		"wrong-root":         func(i *Input) { i.Root[0] ^= 1 },
		"wrong-address":      func(i *Input) { i.Result.Address[0] ^= 1 },
		"wrong-balance":      func(i *Input) { i.Result.Balance = (*hexutil.Big)(big.NewInt(1)) },
		"wrong-nonce":        func(i *Input) { i.Result.Nonce++ },
		"wrong-code":         func(i *Input) { i.Result.CodeHash[0] ^= 1 },
		"wrong-storage-root": func(i *Input) { i.Result.StorageHash[0] ^= 1 },
		"corrupt-node":       func(i *Input) { i.Result.AccountProof[0][0] ^= 1 },
		"truncated-proof":    func(i *Input) { i.Result.AccountProof = i.Result.AccountProof[:len(i.Result.AccountProof)-1] },
		"excess-storage-keys": func(i *Input) {
			for len(i.Result.StorageProof) <= 64 {
				i.Result.StorageProof = append(i.Result.StorageProof, i.Result.StorageProof[0])
			}
		},
	}
	for name, change := range changes {
		t.Run(name, func(t *testing.T) {
			input := fixtures(t, "geth-fixture-proofs")[0]
			change(&input)
			if err := check(input, &Summary{}); err == nil {
				t.Fatal("forgery accepted")
			}
		})
	}
}
func TestStorageAndExclusionForgery(t *testing.T) {
	for _, mode := range []string{"value", "key", "missing-nodes", "absent-account"} {
		t.Run(mode, func(t *testing.T) {
			for _, input := range fixtures(t, "geth-fixture-proofs") {
				if mode == "absent-account" {
					if input.Result.CodeHash != (common.Hash{}) {
						continue
					}
					input.Result.StorageProof[0].Value = (*hexutil.Big)(big.NewInt(1))
					if err := check(input, &Summary{}); err == nil {
						t.Fatal("absent account storage accepted")
					}
					return
				}
				for index, s := range input.Result.StorageProof {
					if s.Value.ToInt().Sign() == 0 {
						continue
					}
					switch mode {
					case "value":
						input.Result.StorageProof[index].Value = (*hexutil.Big)(new(big.Int).Add(s.Value.ToInt(), big.NewInt(1)))
					case "key":
						input.Result.StorageProof[index].Key = "0xffffffffffffffff"
					case "missing-nodes":
						input.Result.StorageProof[index].Proof = nil
					}
					if err := check(input, &Summary{}); err == nil {
						t.Fatal("storage forgery accepted")
					}
					return
				}
			}
			t.Fatal("no applicable fixture")
		})
	}
}
