package bridgechecker

import (
	"bytes"
	"compress/gzip"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"io"
	"os"
	"testing"
)

func TestVaultDomainAndPinnedRuntime(t *testing.T) {
	// Independent byte/domain audit, not a vault release or settled EVM proof.
	file, err := os.Open("../../specs/evidence/native-vault/0.210.0/evidence.json.gz")
	if err != nil {
		t.Fatal(err)
	}
	defer file.Close()
	compressed, err := gzip.NewReader(file)
	if err != nil {
		t.Fatal(err)
	}
	defer compressed.Close()
	data, err := io.ReadAll(compressed)
	if err != nil {
		t.Fatal(err)
	}
	var e struct {
		Results struct {
			Config  string `json:"config"`
			Domain  string `json:"domain"`
			Runtime string `json:"runtime_code_hash"`
		} `json:"results"`
	}
	if err = json.Unmarshal(data, &e); err != nil {
		t.Fatal(err)
	}
	cfg := common.FromHex(e.Results.Config)
	if len(cfg) != 164 || !bytes.Equal(cfg[:8], []byte("TO1VAU01")) {
		t.Fatal("config")
	}
	bridge := crypto.Keccak256([]byte("TO1CKBD1"), cfg[40:72], cfg[72:104], cfg[8:40])
	domain := crypto.Keccak256([]byte("TO1BRDG1"), bridge, word(binary.LittleEndian.Uint64(cfg[104:112])), cfg[112:132])
	if !bytes.Equal(domain, common.FromHex(e.Results.Domain)) {
		t.Fatal("domain")
	}
	source, err := os.ReadFile("../../contracts/bridge/NativeCKB.json")
	if err != nil {
		t.Fatal(err)
	}
	var artifact struct {
		Contract struct {
			EVM struct {
				Deployed struct {
					Object string                                   `json:"object"`
					Refs   map[string][]struct{ Start, Length int } `json:"immutableReferences"`
				} `json:"deployedBytecode"`
			} `json:"evm"`
		} `json:"contract"`
	}
	if err = json.Unmarshal(source, &artifact); err != nil {
		t.Fatal(err)
	}
	deployed := artifact.Contract.EVM.Deployed
	code, err := hex.DecodeString(deployed.Object)
	if err != nil {
		t.Fatal(err)
	}
	if len(deployed.Refs) != 1 {
		t.Fatal("immutable set")
	}
	for _, refs := range deployed.Refs {
		for _, ref := range refs {
			if ref.Length != 32 || ref.Start < 0 || ref.Start+32 > len(code) || !bytes.Equal(code[ref.Start:ref.Start+32], make([]byte, 32)) {
				t.Fatal("immutable patch")
			}
			copy(code[ref.Start:ref.Start+32], domain)
		}
	}
	if !bytes.Equal(crypto.Keccak256(code), common.FromHex(e.Results.Runtime)) {
		t.Fatal("runtime code hash")
	}
}
