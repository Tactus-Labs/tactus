use std::{env, fs, path::PathBuf};
fn main() {
    let path = "../../contracts/bridge/NativeCKB.json";
    println!("cargo:rerun-if-changed={path}");
    let artifact: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let deployed = &artifact["contract"]["evm"]["deployedBytecode"];
    let hex = deployed["object"].as_str().unwrap().as_bytes();
    let bytes: Vec<u8> = hex
        .chunks_exact(2)
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect();
    let refs = deployed["immutableReferences"].as_object().unwrap();
    assert_eq!(refs.len(), 1, "only DOMAIN may be immutable");
    let offsets: Vec<usize> = refs
        .values()
        .next()
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            assert_eq!(v["length"], 32);
            let offset = v["start"].as_u64().unwrap() as usize;
            assert_eq!(&bytes[offset..offset + 32], &[0; 32]);
            offset
        })
        .collect();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("runtime.rs");
    fs::write(out,format!("pub const TEMPLATE: &[u8] = &{bytes:?};\npub const DOMAIN_OFFSETS: &[usize] = &{offsets:?};\n")).unwrap();
}
