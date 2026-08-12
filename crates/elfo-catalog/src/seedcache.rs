//! A committed, repo-portable cache of converged first members.
//!
//! Every family in the catalog starts from one expensive act: seed the `M:k`
//! geometry analytically and drive the multiple-shooting corrector into its basin.
//! For the easy families that costs seconds; for the hard ones it costs a
//! *campaign* (see `docs/superpowers/dual-resonance-implementation-notes.md`), and
//! the campaign's result — a converged state and period — is a handful of numbers.
//! Throwing those away after every run and re-deriving them from the analytic seed
//! is both slow and, for the hard families, impossible: the analytic seed is outside
//! the basin, which is why the campaign existed.
//!
//! So they are cached on disk, under version control:
//!
//! ```text
//! seeds/{combo_id}/n{M}.json        # k = 1, e.g. seeds/full/n45.json
//! seeds/{combo_id}/n{M}_{k}.json    # k > 1, e.g. seeds/full/n149_2.json
//! seeds/{combo_id}/absent.json      # resonances confirmed not to converge
//! ```
//!
//! The file names are exactly `Resonance::dir()`, so the cache layout mirrors the
//! catalog's output layout and a human can pair them up by eye.
//!
//! A cached record is a *warm start*, never an answer: the generator still runs the
//! corrector, from the cached state instead of the analytic one, and still requires
//! it to converge (it takes 1–3 Newton steps rather than 5–15, or fails loudly if
//! the physics underneath it has changed). Nothing in the catalog is ever read
//! straight out of the cache, so a stale seed can slow a run down or make a family
//! absent, but it cannot silently corrupt a published orbit.
//!
//! `absent.json` is the other half: a family that has been *confirmed* not to
//! converge costs ~5 minutes of corrector time per run to re-confirm (2M segments,
//! then the 400-segment retry). Recording the confirmation turns that into a stderr
//! line. `ELFO_RETRY_ABSENT=1` ignores the list, which is what you set the day you
//! change the corrector.

use std::path::{Path, PathBuf};

use elfo_core::constants::R_MOON_ND;
use serde::{Deserialize, Serialize};

use crate::config::Resonance;

/// On-disk schema of the seed files. Bump when the *meaning* of a field changes;
/// a record whose version this build does not recognise is ignored with a warning
/// rather than trusted or fatal, because a stale warm start is not worth a failed
/// catalog run.
pub const SEED_SCHEMA_VERSION: u32 = 1;

/// One converged first member: everything needed to re-seed its family.
///
/// Deliberately *not* the whole `PeriodicOrbit` — the node set is `2M` states that
/// `seed_nodes` reconstructs exactly from `state0` and `period_nd` by propagation,
/// so storing it would multiply the file size by 400 to save one integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeedRecord {
    pub schema_version: u32,
    pub combo_id: String,
    pub revs: u32,
    pub closures: u32,
    /// Rotating-frame nondimensional state at t = 0, anchored on `y = 0`.
    pub state0: [f64; 6],
    /// Closure period, nondimensional (the full `k`-closure period).
    pub period_nd: f64,
    /// Corrector residual this state was accepted at, for provenance only.
    pub residual: f64,
    /// Short git hash of the build that produced it.
    pub generated_by: String,
}

impl SeedRecord {
    /// Whether `self` and `other` describe the same orbit to within round-trip
    /// noise. Used to keep a `--write-seeds` run from rewriting (and dirtying the
    /// git index of) every seed file whose only change is `generated_by`.
    pub fn same_orbit(&self, other: &SeedRecord) -> bool {
        self.revs == other.revs
            && self.closures == other.closures
            && self.combo_id == other.combo_id
            && self.period_nd.to_bits() == other.period_nd.to_bits()
            && self.state0.iter().zip(&other.state0).all(|(a, b)| a.to_bits() == b.to_bits())
    }

    /// Structural sanity of a record read off disk, independent of what was asked
    /// for. A NaN state or a non-positive period would not warm-start anything.
    ///
    /// The geometric bounds are deliberately loose — a radius between the lunar
    /// surface and well outside the Hill sphere, a period between a fraction of a
    /// frame period and several — because this is not a basin test. It exists to
    /// refuse a record that is *corrupt or mis-scaled* (a state written in km rather
    /// than nondimensional units lands 380,000× out and would drive the variational
    /// integrator into a step-size collapse), not to second-guess whether a
    /// plausible seed will converge. That question is answered by running the
    /// corrector, which is what the caller does next.
    fn is_usable(&self) -> bool {
        if self.schema_version != SEED_SCHEMA_VERSION
            || self.revs == 0
            || self.closures == 0
            || !self.period_nd.is_finite()
            || !self.state0.iter().all(|c| c.is_finite())
        {
            return false;
        }
        let r = (self.state0[0].powi(2) + self.state0[1].powi(2) + self.state0[2].powi(2)).sqrt();
        let v = (self.state0[3].powi(2) + self.state0[4].powi(2) + self.state0[5].powi(2)).sqrt();
        (R_MOON_ND..0.5).contains(&r) && v < 10.0 && (0.1..64.0).contains(&self.period_nd)
    }
}

/// One confirmed-absent family, with the human note explaining the confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbsentEntry {
    /// The resonance in `Resonance`'s own spelling: `"45"`, `"149:2"`.
    pub resonance: String,
    pub note: String,
}

/// `seeds/{combo}/absent.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsentFile {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub combo_id: String,
    pub absent: Vec<AbsentEntry>,
}

fn default_schema_version() -> u32 {
    SEED_SCHEMA_VERSION
}

/// A cache rooted at a directory. Reads never fail — a missing, malformed or
/// wrong-version file is a cache miss with a stderr note, because the fallback
/// (the analytic seed) is always available.
#[derive(Debug, Clone)]
pub struct SeedCache {
    root: PathBuf,
}

impl SeedCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        SeedCache { root: root.into() }
    }

    /// The committed `seeds/` directory at the repo root, overridable with
    /// `ELFO_SEEDS_DIR` (which is what the tests use, so they never touch the
    /// real cache).
    pub fn default_root() -> PathBuf {
        if let Some(dir) = std::env::var_os("ELFO_SEEDS_DIR") {
            return PathBuf::from(dir);
        }
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../seeds")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn seed_path(&self, combo_id: &str, res: Resonance) -> PathBuf {
        self.root.join(combo_id).join(format!("{}.json", res.dir()))
    }

    pub fn absent_path(&self, combo_id: &str) -> PathBuf {
        self.root.join(combo_id).join("absent.json")
    }

    /// The cached first member for `(combo_id, res)`, or `None` on any kind of miss.
    ///
    /// The record's own `combo_id`/`revs`/`closures` are checked against the key it
    /// was filed under: a seed copied to the wrong path would otherwise warm-start
    /// the wrong family, and since the corrector converges to *a* periodic orbit
    /// near whatever it is given, the result would be a mislabelled family rather
    /// than an error. That is the exact class of bug this catalog has been bitten by
    /// before (the apoapsis off-by-one), so it is checked rather than assumed.
    pub fn load(&self, combo_id: &str, res: Resonance) -> Option<SeedRecord> {
        let path = self.seed_path(combo_id, res);
        let text = std::fs::read_to_string(&path).ok()?;
        let rec: SeedRecord = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("seed cache: ignoring unreadable {}: {e}", path.display());
                return None;
            }
        };
        if !rec.is_usable() {
            eprintln!(
                "seed cache: ignoring {} (schema_version {}, expected {SEED_SCHEMA_VERSION})",
                path.display(),
                rec.schema_version
            );
            return None;
        }
        if rec.combo_id != combo_id || rec.revs != res.revs || rec.closures != res.closures {
            eprintln!(
                "seed cache: ignoring {}: it describes combo {} n={}:{}, not {combo_id} n={res}",
                path.display(),
                rec.combo_id,
                rec.revs,
                rec.closures
            );
            return None;
        }
        Some(rec)
    }

    /// Write (or overwrite) one seed record.
    pub fn store(&self, rec: &SeedRecord) -> anyhow::Result<()> {
        let res = Resonance { revs: rec.revs, closures: rec.closures };
        let path = self.seed_path(&rec.combo_id, res);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut text = serde_json::to_string_pretty(rec)?;
        text.push('\n');
        std::fs::write(&path, text)?;
        Ok(())
    }

    /// The absence list for `combo_id`; an empty list if there is no file.
    pub fn absent(&self, combo_id: &str) -> AbsentFile {
        let empty = AbsentFile {
            schema_version: SEED_SCHEMA_VERSION,
            combo_id: combo_id.to_string(),
            absent: Vec::new(),
        };
        let path = self.absent_path(combo_id);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return empty;
        };
        match serde_json::from_str::<AbsentFile>(&text) {
            Ok(f) if f.schema_version == SEED_SCHEMA_VERSION => f,
            Ok(f) => {
                eprintln!(
                    "seed cache: ignoring {} (schema_version {}, expected {SEED_SCHEMA_VERSION})",
                    path.display(),
                    f.schema_version
                );
                empty
            }
            Err(e) => {
                eprintln!("seed cache: ignoring unreadable {}: {e}", path.display());
                empty
            }
        }
    }

    /// The recorded note for a confirmed-absent family, if any.
    pub fn absent_note(&self, combo_id: &str, res: Resonance) -> Option<String> {
        let key = res.to_string();
        self.absent(combo_id)
            .absent
            .into_iter()
            .find(|e| e.resonance == key)
            .map(|e| e.note)
    }

    /// Merge a batch of absences into `absent.json`, and drop any listed resonance
    /// that is *not* in `absent` but *is* in `present` — a family that has since
    /// been conquered must not stay on the skip list, or the campaign's own seed
    /// would be ignored on the next run.
    pub fn update_absences(
        &self,
        combo_id: &str,
        newly_absent: &[(Resonance, String)],
        present: &[Resonance],
    ) -> anyhow::Result<()> {
        let mut file = self.absent(combo_id);
        file.schema_version = SEED_SCHEMA_VERSION;
        file.combo_id = combo_id.to_string();
        let present: Vec<String> = present.iter().map(|r| r.to_string()).collect();
        file.absent.retain(|e| !present.contains(&e.resonance));
        for (res, note) in newly_absent {
            let key = res.to_string();
            match file.absent.iter_mut().find(|e| e.resonance == key) {
                Some(e) => e.note = note.clone(),
                None => file.absent.push(AbsentEntry { resonance: key, note: note.clone() }),
            }
        }
        // Sorted by the parsed resonance, not the string, so "9" does not sort after
        // "45" and the committed file has a stable, reviewable order.
        file.absent.sort_by_key(|e| {
            e.resonance
                .parse::<Resonance>()
                .map(|r| (r.closures, r.revs))
                .unwrap_or((u32::MAX, u32::MAX))
        });
        let path = self.absent_path(combo_id);
        // Don't create an empty absent.json for a combo that has never had an
        // absence: an all-green combo should have no file, not a file saying nothing.
        // (An existing file that has just been emptied by a conquest *is* rewritten,
        // so the conquest shows up in the diff.)
        if file.absent.is_empty() && !path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut text = serde_json::to_string_pretty(&file)?;
        text.push('\n');
        std::fs::write(&path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(combo: &str, revs: u32, closures: u32) -> SeedRecord {
        SeedRecord {
            schema_version: SEED_SCHEMA_VERSION,
            combo_id: combo.to_string(),
            revs,
            closures,
            state0: [0.03, 0.0, 0.01, -0.1, 0.7, 0.2],
            period_nd: 6.0155,
            residual: 3.2e-12,
            generated_by: "abcdef0".into(),
        }
    }

    #[test]
    fn seed_round_trips_through_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SeedCache::new(dir.path());
        let n45 = Resonance { revs: 45, closures: 1 };
        let d149 = Resonance { revs: 149, closures: 2 };

        assert!(cache.load("full", n45).is_none(), "empty cache must miss");

        cache.store(&rec("full", 45, 1)).unwrap();
        cache.store(&rec("full", 149, 2)).unwrap();

        // the paths are the catalog's own directory spelling
        assert!(dir.path().join("full/n45.json").exists());
        assert!(dir.path().join("full/n149_2.json").exists());

        let got = cache.load("full", n45).expect("stored seed must load");
        assert_eq!(got, rec("full", 45, 1));
        assert!(got.same_orbit(&rec("full", 45, 1)));
        assert_eq!(cache.load("full", d149).unwrap().closures, 2);
        // ...and a different combo is a different cache entry, not a fallback
        assert!(cache.load("no-c22", n45).is_none());
    }

    #[test]
    fn misfiled_or_stale_records_are_misses_not_lies() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SeedCache::new(dir.path());
        let n45 = Resonance { revs: 45, closures: 1 };

        // A record for n=50 written to n45.json: warm-starting the 45 family from it
        // would converge to a *mislabelled* family, so it must be refused.
        std::fs::create_dir_all(dir.path().join("full")).unwrap();
        std::fs::write(
            dir.path().join("full/n45.json"),
            serde_json::to_string(&rec("full", 50, 1)).unwrap(),
        )
        .unwrap();
        assert!(cache.load("full", n45).is_none(), "misfiled seed must be ignored");

        // A future schema version, and outright garbage, are both misses.
        let mut future = rec("full", 45, 1);
        future.schema_version = SEED_SCHEMA_VERSION + 1;
        std::fs::write(
            dir.path().join("full/n45.json"),
            serde_json::to_string(&future).unwrap(),
        )
        .unwrap();
        assert!(cache.load("full", n45).is_none(), "future schema must be ignored");

        std::fs::write(dir.path().join("full/n45.json"), "{not json").unwrap();
        assert!(cache.load("full", n45).is_none(), "garbage must be ignored");

        // A NaN state is structurally unusable even at the right version.
        let mut nan = rec("full", 45, 1);
        nan.state0[3] = f64::NAN;
        // serde_json cannot even serialise NaN, so write the JSON by hand
        std::fs::write(
            dir.path().join("full/n45.json"),
            r#"{"schema_version":1,"combo_id":"full","revs":45,"closures":1,
                "state0":[0.03,0.0,0.01,null,0.7,0.2],"period_nd":6.0,
                "residual":1e-12,"generated_by":"x"}"#,
        )
        .unwrap();
        assert!(cache.load("full", n45).is_none(), "unparseable state must be ignored");

        // A state written in km rather than nondimensional units: structurally
        // perfect JSON, and the one form of corruption that would blow up the
        // variational integrator rather than merely fail to converge.
        let mut km = rec("full", 45, 1);
        for c in km.state0.iter_mut() {
            *c *= 384_400.0;
        }
        cache.store(&km).unwrap();
        assert!(cache.load("full", n45).is_none(), "a mis-scaled state must be ignored");
    }

    #[test]
    fn absences_merge_and_conquests_clear_them() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SeedCache::new(dir.path());
        let n45 = Resonance { revs: 45, closures: 1 };
        let n50 = Resonance { revs: 50, closures: 1 };
        let d149 = Resonance { revs: 149, closures: 2 };

        assert!(cache.absent("full").absent.is_empty());
        assert!(cache.absent_note("full", n45).is_none());

        cache
            .update_absences(
                "full",
                &[(n45, "stalled at 4.9e-4".into()), (n50, "stalled at 1e-3".into())],
                &[],
            )
            .unwrap();
        assert_eq!(cache.absent_note("full", n45).as_deref(), Some("stalled at 4.9e-4"));
        assert_eq!(cache.absent("full").absent.len(), 2);

        // a second batch merges rather than replaces, and re-notes an existing entry
        cache
            .update_absences(
                "full",
                &[(d149, "corrector stalled".into()), (n45, "still stalled".into())],
                &[],
            )
            .unwrap();
        assert_eq!(cache.absent("full").absent.len(), 3);
        assert_eq!(cache.absent_note("full", n45).as_deref(), Some("still stalled"));
        // k=1 entries sort before k=2, by revs
        let order: Vec<String> =
            cache.absent("full").absent.iter().map(|e| e.resonance.clone()).collect();
        assert_eq!(order, vec!["45", "50", "149:2"]);

        // conquering 45 removes it from the skip list
        cache.update_absences("full", &[], &[n45]).unwrap();
        assert!(cache.absent_note("full", n45).is_none());
        assert_eq!(cache.absent_note("full", n50).as_deref(), Some("stalled at 1e-3"));
    }
}
