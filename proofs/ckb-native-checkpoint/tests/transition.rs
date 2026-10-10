use tactus_o1_native_checkpoint_script::{advanced, STATE_BYTES};
fn states() -> Vec<[u8; STATE_BYTES]> {
    let d: serde_json::Value = serde_json::from_str(include_str!(
        "../../../specs/evidence/native-publication/0.210.0/deployment.json"
    ))
    .unwrap();
    let e: serde_json::Value = serde_json::from_str(include_str!(
        "../../../specs/evidence/native-publication/0.210.0/execution.json"
    ))
    .unwrap();
    let bytes = hex::decode(
        d["genesis_state"]
            .as_str()
            .unwrap()
            .trim_start_matches("0x"),
    )
    .unwrap();
    let mut result = vec![bytes.as_slice().try_into().unwrap()];
    for step in e["cases"][0]["steps"].as_array().unwrap() {
        let mut s = bytes.clone();
        s[172..].copy_from_slice(
            &hex::decode(step["after"].as_str().unwrap().trim_start_matches("0x")).unwrap(),
        );
        result.push(s.try_into().unwrap());
    }
    result
}
#[test]
fn authentic_native_transitions_and_immutable_domains() {
    let s = states();
    assert!(advanced(&s[0], &s[1]));
    assert!(advanced(&s[1], &s[2]));
    assert!(!advanced(&s[0], &s[0]));
    assert!(!advanced(&s[0], &s[2]));
    assert!(!advanced(&s[2], &s[1]));
    for offset in (0..212).chain(268..372) {
        let mut bad = s[1];
        bad[offset] ^= 1;
        assert!(!advanced(&s[0], &bad), "offset {offset}");
    }
}
#[test]
fn cursor_regression_unchanged_and_bounds() {
    let s = states();
    let mut after = s[2];
    after[372..].copy_from_slice(&s[1][372..]);
    assert!(advanced(&s[1], &after));
    after[396] ^= 1;
    assert!(!advanced(&s[1], &after));
    for n in [0, 34, u64::MAX] {
        let mut bad = s[2];
        bad[372..380].copy_from_slice(&n.to_le_bytes());
        assert!(!advanced(&s[1], &bad));
    }
    let mut bad = s[2];
    bad[380..396].copy_from_slice(&s[1][380..396]);
    assert!(!advanced(&s[1], &bad));
    let mut before = s[1];
    before[212..220].fill(255);
    bad = s[2];
    bad[212..220].fill(0);
    assert!(!advanced(&before, &bad));
}
