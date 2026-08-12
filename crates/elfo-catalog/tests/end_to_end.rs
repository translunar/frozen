//! End-to-end test of the catalog generator: a tiny config (one combo, one
//! resonance, 2 members per direction) run through `elfo_catalog::run`
//! directly (no binary spawn), asserting the on-disk layout and invariants
//! the web browser (Task 16) will rely on.

const TINY_CONFIG: &str = r#"
members_per_direction = 2
ds0 = 5e-4
resonances = [25]

[[combos]]
id = "full"
name = "J2 + C22 + J3 + Earth"
force_model = { j2 = true, c22 = true, j3 = true, earth = true }
"#;

#[test]
fn end_to_end_generates_one_family() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("catalog.toml");
    std::fs::write(&config_path, TINY_CONFIG).unwrap();
    let out_dir = dir.path().join("out");

    elfo_catalog::run(&config_path, &out_dir).expect("catalog generation must succeed");

    let catalog_text = std::fs::read_to_string(out_dir.join("catalog.json"))
        .expect("catalog.json must exist at the out root");
    let v: serde_json::Value =
        serde_json::from_str(&catalog_text).expect("catalog.json must parse");

    let combos = v["combos"].as_array().expect("combos array");
    assert_eq!(combos.len(), 1, "expected exactly 1 combo");
    assert_eq!(combos[0]["id"], "full");

    let families = combos[0]["families"].as_array().expect("families array");
    assert_eq!(families.len(), 1, "expected exactly 1 family (n=25)");
    assert_eq!(families[0]["resonance_n"], 25);

    let members = families[0]["members"].as_array().expect("members array");
    assert!(
        members.len() >= 3,
        "expected >= 3 members, got {}",
        members.len()
    );

    let expected_bytes = (100 * 25 * 3 * 4) as u64; // 100*N samples * xyz * f32
    for (i, m) in members.iter().enumerate() {
        assert_eq!(
            m["index"], i as u64,
            "member index must be 0..len contiguous"
        );

        let residual = m["residual"].as_f64().expect("residual");
        assert!(
            residual < 1e-9,
            "member {i}: residual {residual} not < 1e-9"
        );

        let traj_rel = m["traj"].as_str().expect("traj path");
        let traj_path = out_dir.join(traj_rel);
        let meta = std::fs::metadata(&traj_path)
            .unwrap_or_else(|e| panic!("member {i} traj file {traj_path:?} missing: {e}"));
        assert_eq!(meta.len(), expected_bytes, "member {i} traj file size");
    }

    let preview_rel = families[0]["preview"].as_str().expect("preview path");
    let preview_path = out_dir.join(preview_rel);
    assert!(
        preview_path.exists(),
        "preview.f32 must exist at {preview_path:?}"
    );
}
