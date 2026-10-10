// Independently verify Ethereum account/storage RPC witnesses using Go Ethereum.
// The caller must authenticate stateRoot separately; this tool does not verify CKB.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"math/big"
	"os"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/hexutil"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethdb/memorydb"
	"github.com/ethereum/go-ethereum/rlp"
	"github.com/ethereum/go-ethereum/trie"
)

type Storage struct {
	Key   string          `json:"key"`
	Value *hexutil.Big    `json:"value"`
	Proof []hexutil.Bytes `json:"proof"`
}
type Account struct {
	Address      common.Address  `json:"address"`
	Balance      *hexutil.Big    `json:"balance"`
	Nonce        hexutil.Uint64  `json:"nonce"`
	CodeHash     common.Hash     `json:"codeHash"`
	StorageHash  common.Hash     `json:"storageHash"`
	AccountProof []hexutil.Bytes `json:"accountProof"`
	StorageProof []Storage       `json:"storageProof"`
}
type Input struct {
	Root   common.Hash `json:"stateRoot"`
	Result Account     `json:"result"`
}
type Summary struct {
	Accounts int `json:"accounts"`
	Absent   int `json:"absent_accounts"`
	Storage  int `json:"storage_keys"`
	Nonzero  int `json:"nonzero_storage_keys"`
}

func verify(root common.Hash, key []byte, nodes []hexutil.Bytes) ([]byte, error) {
	if len(nodes) > 128 {
		return nil, fmt.Errorf("too many proof nodes")
	}
	if root == types.EmptyRootHash && len(nodes) == 0 {
		return nil, nil
	}
	database := memorydb.New()
	defer database.Close()
	for _, node := range nodes {
		if len(node) == 0 || len(node) > 4096 {
			return nil, fmt.Errorf("invalid node size")
		}
		if err := database.Put(crypto.Keccak256(node), node); err != nil {
			return nil, err
		}
	}
	return trie.VerifyProof(root, key, database)
}
func check(input Input, summary *Summary) error {
	account := input.Result
	if account.Balance == nil || len(account.StorageProof) > 64 {
		return fmt.Errorf("invalid account fields")
	}
	leaf, err := verify(input.Root, crypto.Keccak256(account.Address[:]), account.AccountProof)
	if err != nil {
		return fmt.Errorf("account proof: %w", err)
	}
	exists := leaf != nil
	if !exists {
		if account.Balance.ToInt().Sign() != 0 || account.Nonce != 0 || account.CodeHash != (common.Hash{}) || account.StorageHash != (common.Hash{}) {
			return fmt.Errorf("absent account claims nonzero state")
		}
		summary.Absent++
	} else {
		expected, err := rlp.EncodeToBytes([]any{uint64(account.Nonce), account.Balance.ToInt(), account.StorageHash, account.CodeHash})
		if err != nil || !bytes.Equal(expected, leaf) {
			return fmt.Errorf("account fields differ from authenticated leaf")
		}
	}
	for _, storage := range account.StorageProof {
		if storage.Value == nil {
			return fmt.Errorf("missing storage value")
		}
		key, err := hexutil.DecodeBig(storage.Key)
		if err != nil || key.BitLen() > 256 {
			return fmt.Errorf("invalid storage key")
		}
		value := storage.Value.ToInt()
		if !exists {
			if value.Sign() != 0 || len(storage.Proof) != 0 {
				return fmt.Errorf("absent account claims storage")
			}
		} else {
			encoded, err := verify(account.StorageHash, crypto.Keccak256(common.BigToHash(key).Bytes()), storage.Proof)
			if err != nil {
				return fmt.Errorf("storage proof: %w", err)
			}
			if value.Sign() == 0 {
				if encoded != nil {
					return fmt.Errorf("zero storage must be absent from trie")
				}
			} else {
				expected, err := rlp.EncodeToBytes(new(big.Int).Set(value))
				if err != nil || !bytes.Equal(encoded, expected) {
					return fmt.Errorf("storage value differs from authenticated leaf")
				}
				summary.Nonzero++
			}
		}
		summary.Storage++
	}
	summary.Accounts++
	return nil
}
func run() error {
	if len(os.Args) != 2 {
		return fmt.Errorf("usage: proof-checker PROOFS_JSON (array of {stateRoot,result})")
	}
	file, err := os.Open(os.Args[1])
	if err != nil {
		return err
	}
	defer file.Close()
	data, err := io.ReadAll(io.LimitReader(file, 16*1024*1024+1))
	if err != nil {
		return err
	}
	if len(data) > 16*1024*1024 {
		return fmt.Errorf("proof document too large")
	}
	var inputs []Input
	if err = json.Unmarshal(data, &inputs); err != nil {
		return err
	}
	if len(inputs) == 0 || len(inputs) > 4096 {
		return fmt.Errorf("invalid proof document count")
	}
	summary := Summary{}
	for i, input := range inputs {
		if err = check(input, &summary); err != nil {
			return fmt.Errorf("entry %d: %w", i, err)
		}
	}
	return json.NewEncoder(os.Stdout).Encode(summary)
}
func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
