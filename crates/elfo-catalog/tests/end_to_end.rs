//! End-to-end test of the catalog generator: a tiny config (one combo, one
//! resonance, 2 members per direction) run through `elfo_catalog::run_with`
//! directly (no binary spawn), asserting the on-disk layout and invariants
//! the web browser (Task 16) will rely on.

use std::path::Path;

use elfo_catalog::seedcache::SeedCache;
use elfo_catalog::GenOptions;

const MEMBERS_PER_DIRECTION: usize = 2;

const TINY_CONFIG: &str = r#"
members_per_direction = 2
ds0 = 5e-4
resonances = ["25"]

[[combos]]
id = "full"
name = "J2 + C22 + J3 + Earth"
force_model = { j2 = true, c22 = true, j3 = true, earth = true }
"#;

/// Options pointing at a private, empty seed cache. Every test here supplies its
/// own: the default cache is the repo's committed `seeds/`, and a test that read it
/// would warm-start from whatever the last campaign happened to commit — passing or
/// failing for reasons outside the test.
fn opts(cache_root: &Path, write_seeds: bool) -> GenOptions {
    GenOptions {
        seeds: SeedCache::new(cache_root),
        write_seeds,
        retry_absent: false,
    }
}

/// Assert everything the catalog layout guarantees, and return the parsed
/// `catalog.json` for further per-test assertions.
fn check_layout(out_dir: &Path) -> serde_json::Value {
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
    assert_eq!(
        families[0]["closures"], 1,
        "an integer-spelled resonance must be written as the k=1 rational"
    );

    let members = families[0]["members"].as_array().expect("members array");
    assert!(
        members.len() >= 3,
        "expected >= 3 members, got {}",
        members.len()
    );
    let max_members = 2 * MEMBERS_PER_DIRECTION + 1;
    assert!(
        members.len() <= max_members,
        "expected <= {max_members} members (2*members_per_direction + 1), got {}",
        members.len()
    );

    let expected_bytes = (100 * 25 * 3 * 4) as u64; // 100*N samples * xyz * f32
    let mut prev_state0: Option<Vec<f64>> = None;
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

        let state0: Vec<f64> = m["state0"]
            .as_array()
            .expect("state0 array")
            .iter()
            .map(|v| v.as_f64().expect("state0 component"))
            .collect();
        if let Some(prev) = &prev_state0 {
            let identical = state0.iter().zip(prev).all(|(a, b)| (a - b).abs() < 1e-15);
            assert!(
                !identical,
                "member {i} state0 duplicates member {}'s state0 (duplicated first member?)",
                i - 1
            );
        }
        prev_state0 = Some(state0);

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
    v
}

/// The `state0`/`residual` of every member of the run's single family.
fn members(v: &serde_json::Value) -> Vec<(Vec<f64>, f64)> {
    v["combos"][0]["families"][0]["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| {
            (
                m["state0"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| c.as_f64().unwrap())
                    .collect(),
                m["residual"].as_f64().unwrap(),
            )
        })
        .collect()
}

#[test]
fn end_to_end_generates_one_family() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("catalog.toml");
    std::fs::write(&config_path, TINY_CONFIG).unwrap();
    let out_dir = dir.path().join("out");

    elfo_catalog::run_with(&config_path, &out_dir, &opts(&dir.path().join("seeds"), false))
        .expect("catalog generation must succeed");

    check_layout(&out_dir);
}

#[test]
fn a_written_seed_warm_starts_an_identical_second_run() {
    // The seed cache's whole promise: run once from the analytic seed with
    // `--write-seeds`, and the *next* run reaches the same family from the cached
    // state. Timing is not asserted — a wall-clock comparison in CI is a flake
    // generator — but the two things that would make a fast run worthless are:
    // the warm-started corrector must still converge (residuals under the catalog's
    // own 1e-9 bar), and it must converge to the *same* family, not a neighbouring
    // one that happens to be nearby.
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("catalog.toml");
    std::fs::write(&config_path, TINY_CONFIG).unwrap();
    let cache_root = dir.path().join("seeds");

    let cold_out = dir.path().join("cold");
    elfo_catalog::run_with(&config_path, &cold_out, &opts(&cache_root, true))
        .expect("cold generation must succeed");
    let cold = check_layout(&cold_out);

    // --write-seeds must have left exactly the file the next run will look for.
    let seed_path = cache_root.join("full/n25.json");
    let seed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&seed_path).expect("n25.json must exist"))
            .expect("seed must parse");
    assert_eq!(seed["schema_version"], 1);
    assert_eq!(seed["combo_id"], "full");
    assert_eq!((seed["revs"].as_u64(), seed["closures"].as_u64()), (Some(25), Some(1)));
    assert!(
        seed["residual"].as_f64().unwrap() < 1e-9,
        "a cached seed must be a converged one"
    );
    // The cached state is the continuation's *origin*, so it must be one of the
    // family's own members — not a re-derived approximation of one.
    let cached_state: Vec<f64> =
        seed["state0"].as_array().unwrap().iter().map(|c| c.as_f64().unwrap()).collect();
    let cold_members = members(&cold);
    assert!(
        cold_members
            .iter()
            .any(|(s, _)| s.iter().zip(&cached_state).all(|(a, b)| (a - b).abs() < 1e-12)),
        "the cached seed must be one of the family's members"
    );
    // A family that converged is not an absence, so no absent.json is written.
    assert!(
        !cache_root.join("full/absent.json").exists(),
        "an all-converged combo must not get an absence file"
    );

    // Second run: same config, same (now populated) cache, no writing.
    let warm_out = dir.path().join("warm");
    elfo_catalog::run_with(&config_path, &warm_out, &opts(&cache_root, false))
        .expect("warm generation must succeed");
    let warm = check_layout(&warm_out);

    let warm_members = members(&warm);
    assert_eq!(
        cold_members.len(),
        warm_members.len(),
        "warm-started family has a different member count"
    );
    // The continuation *origin* — the member the cache actually seeded — must come
    // back essentially bit-identical: this is the assertion that says the warm start
    // reproduced the orbit rather than merely something in the neighbourhood.
    let closest = warm_members
        .iter()
        .map(|(s, _)| {
            s.iter().zip(&cached_state).map(|(a, b)| (a - b).abs()).fold(0.0f64, f64::max)
        })
        .fold(f64::MAX, f64::min);
    // 1e-7 nondimensional is 38 m. The warm start does not land *exactly* back on
    // the cached state (measured: 2.3e-9, ~0.9 m): `seed_nodes` rebuilds the node
    // set by propagating from state0, so the corrector is handed a slightly
    // different point of the same orbit and re-converges to a slightly different
    // one. What matters is that the displacement is metres, not the ~200 km spacing
    // between adjacent family members.
    assert!(
        closest < 1e-7,
        "the warm run must reproduce the cached member itself; closest match differs by {closest:e}"
    );
    for (i, ((cs, _), (ws, wr))) in cold_members.iter().zip(&warm_members).enumerate() {
        // Convergence, independently of `check_layout`'s per-member bar: the warm
        // start is only worth anything if the corrector still lands on a real orbit.
        assert!(*wr < 1e-9, "warm member {i}: residual {wr} not converged");
        // The other members are reached by arclength continuation *from* that
        // origin, and each corrected step amplifies the origin's ~1e-9 displacement
        // (measured: 2.3e-5 by member 0, two steps out). The scale to judge that
        // against is the continuation step itself — `ds0 = 5e-4` in the packed
        // state norm — so 1e-4 says the warm family sits within a fifth of one step
        // of the cold one, i.e. they are the same members, not merely the same
        // family.
        for (k, (a, b)) in cs.iter().zip(ws).enumerate() {
            assert!(
                (a - b).abs() < 1e-4,
                "warm member {i} component {k}: {b} differs from the cold run's {a}"
            );
        }
    }
}

#[test]
fn a_recorded_absence_is_skipped_and_a_stale_seed_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("catalog.toml");
    std::fs::write(&config_path, TINY_CONFIG).unwrap();
    let cache_root = dir.path().join("seeds");
    std::fs::create_dir_all(cache_root.join("full")).unwrap();

    // n=25 converges from a cold start, but an absence record must be honoured
    // without even trying — that is the point of the record, and the only way to
    // observe it is a family that *would* have converged going missing.
    std::fs::write(
        cache_root.join("full/absent.json"),
        r#"{"schema_version":1,"combo_id":"full",
            "absent":[{"resonance":"25","note":"planted by a test"}]}"#,
    )
    .unwrap();
    let out = dir.path().join("skipped");
    elfo_catalog::run_with(&config_path, &out, &opts(&cache_root, false)).unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("catalog.json")).unwrap()).unwrap();
    assert!(
        v["combos"][0]["families"].as_array().unwrap().is_empty(),
        "a recorded absence must skip the family"
    );

    // ...and `retry_absent` overrides it, recovering the family. Writing seeds on
    // that run must also *retract* the absence: a family that has been conquered
    // cannot stay on the skip list, or its own fresh seed would be ignored.
    let out = dir.path().join("retried");
    let mut o = opts(&cache_root, true);
    o.retry_absent = true;
    elfo_catalog::run_with(&config_path, &out, &o).unwrap();
    check_layout(&out);
    let absent: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(cache_root.join("full/absent.json")).unwrap(),
    )
    .unwrap();
    assert!(
        absent["absent"].as_array().unwrap().is_empty(),
        "converging must retract the recorded absence"
    );

    // A seed that no longer converges must not take the family down with it: the
    // generator falls back to the analytic path and the family still appears. Build
    // the stale record the way a real one goes stale — a genuine converged state
    // that a change in the dynamics has moved out from under — by taking the run's
    // own seed and displacing it 1 % in position.
    //
    // 1 % is enough to send the variational integrator's step size to zero, which it
    // reports by *panicking*. That is the case worth pinning: without the
    // `catch_unwind` in `warm_start`, this test aborts the process rather than
    // failing, and in production one stale committed seed would take down a
    // multi-hour catalog build from inside a rayon worker. The expected behaviour is
    // a stderr complaint and a family generated from the analytic seed instead.
    let seed_path = cache_root.join("full/n25.json");
    let mut seed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&seed_path).unwrap()).unwrap();
    for k in 0..3 {
        let c = seed["state0"][k].as_f64().unwrap();
        seed["state0"][k] = serde_json::json!(c * 1.01);
    }
    std::fs::write(&seed_path, serde_json::to_string(&seed).unwrap()).unwrap();
    let out = dir.path().join("stale");
    elfo_catalog::run_with(&config_path, &out, &opts(&cache_root, false))
        .expect("a stale seed must not fail the run");
    check_layout(&out);
}
