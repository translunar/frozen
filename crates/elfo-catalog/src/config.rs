use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use elfo_core::forces::ForceModel;
use serde::{Deserialize, Deserializer};

/// One force-model/combo entry driving a family sweep.
#[derive(Debug, Clone, Deserialize)]
pub struct ComboCfg {
    pub id: String,
    pub name: String,
    pub force_model: ForceModel,
}

/// A rational rotating-frame resonance `M:k` — the orbit closes after the node has
/// regressed through `k` full sweeps, having completed `M` revolutions.
///
/// Written in TOML as a string: `"25"` (= `25:1`, the classical case) or `"149:2"`.
/// Integers are *not* accepted as TOML integers, deliberately: one spelling per
/// concept keeps `catalog.toml` readable and the parse total.
///
/// Non-reduced fractions (`"50:2"`) are accepted and are not normalised — they
/// describe the reduced family (`25:1`) traversed twice, which is a legitimate if
/// wasteful thing to ask for, and silently renaming a requested family would be
/// worse than honouring it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Resonance {
    pub revs: u32,
    pub closures: u32,
}

impl Resonance {
    /// Directory-name fragment: `n25` for `k = 1` (unchanged from the integer-only
    /// catalog, so existing paths and existing web builds keep working), `n149_2`
    /// otherwise.
    pub fn dir(&self) -> String {
        if self.closures == 1 {
            format!("n{}", self.revs)
        } else {
            format!("n{}_{}", self.revs, self.closures)
        }
    }
}

impl fmt::Display for Resonance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.closures == 1 {
            write!(f, "{}", self.revs)
        } else {
            write!(f, "{}:{}", self.revs, self.closures)
        }
    }
}

impl FromStr for Resonance {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || format!("bad resonance {s:?}: expected \"M\" or \"M:k\" with M, k ≥ 1");
        let (revs, closures) = match s.split_once(':') {
            Some((m, k)) => (m, k),
            None => (s, "1"),
        };
        let revs: u32 = revs.trim().parse().map_err(|_| bad())?;
        let closures: u32 = closures.trim().parse().map_err(|_| bad())?;
        if revs == 0 || closures == 0 {
            return Err(bad());
        }
        Ok(Resonance { revs, closures })
    }
}

impl<'de> Deserialize<'de> for Resonance {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Top-level catalog-generation config, loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogConfig {
    pub combos: Vec<ComboCfg>,
    pub resonances: Vec<Resonance>,
    pub members_per_direction: usize,
    pub ds0: f64,
    /// Per-resonance member-count overrides, keyed by the same string spelling used
    /// in `resonances`. The `M:2` families run 200–350 shooting segments per
    /// correction, so their continuation is deliberately shorter than the default.
    #[serde(default)]
    pub members_per_direction_override: BTreeMap<Resonance, usize>,
}

impl CatalogConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    /// Members to continue in each direction for `res`: the override if one is
    /// configured, else the global default.
    pub fn members_for(&self, res: Resonance) -> usize {
        self.members_per_direction_override
            .get(&res)
            .copied()
            .unwrap_or(self.members_per_direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonance_strings_round_trip() {
        assert_eq!("25".parse::<Resonance>().unwrap(), Resonance { revs: 25, closures: 1 });
        assert_eq!("149:2".parse::<Resonance>().unwrap(), Resonance { revs: 149, closures: 2 });
        // Display is the inverse, and k = 1 renders without the ":1" so that the
        // override table can be keyed with the same spelling used in `resonances`.
        assert_eq!(Resonance { revs: 25, closures: 1 }.to_string(), "25");
        assert_eq!(Resonance { revs: 149, closures: 2 }.to_string(), "149:2");
        // ...but "25:1" is still accepted and equal to "25", so a config that spells
        // it out does not silently miss its own override.
        assert_eq!("25:1".parse::<Resonance>().unwrap(), "25".parse::<Resonance>().unwrap());
        assert_eq!(Resonance { revs: 25, closures: 1 }.dir(), "n25");
        assert_eq!(Resonance { revs: 149, closures: 2 }.dir(), "n149_2");
        for bad in ["", "abc", "0", "25:0", "25:", ":2", "-3", "1:2:3", "25.5", "2:x"] {
            assert!(bad.parse::<Resonance>().is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn config_parses_mixed_resonances_and_overrides() {
        let toml_text = r#"
members_per_direction = 40
ds0 = 2e-2
resonances = ["18", "25", "149:2"]

[members_per_direction_override]
"149:2" = 10

[[combos]]
id = "full"
name = "J2 + C22 + J3 + Earth"
force_model = { j2 = true, c22 = true, j3 = true, earth = true }
"#;
        let cfg: CatalogConfig = toml::from_str(toml_text).unwrap();
        assert_eq!(
            cfg.resonances,
            vec![
                Resonance { revs: 18, closures: 1 },
                Resonance { revs: 25, closures: 1 },
                Resonance { revs: 149, closures: 2 },
            ]
        );
        assert_eq!(cfg.members_for(Resonance { revs: 149, closures: 2 }), 10);
        assert_eq!(cfg.members_for(Resonance { revs: 25, closures: 1 }), 40);
    }

    #[test]
    fn garbage_resonance_fails_the_whole_load() {
        let toml_text = r#"
members_per_direction = 40
ds0 = 2e-2
resonances = ["18", "twenty"]
combos = []
"#;
        let err = toml::from_str::<CatalogConfig>(toml_text).unwrap_err().to_string();
        assert!(err.contains("bad resonance"), "unhelpful error: {err}");
        // an integer where a string belongs is also a hard error, not a silent 0
        let ints = r#"
members_per_direction = 40
ds0 = 2e-2
resonances = [18]
combos = []
"#;
        assert!(toml::from_str::<CatalogConfig>(ints).is_err());
    }

    #[test]
    fn shipped_catalog_toml_still_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog.toml");
        let cfg = CatalogConfig::load(&path).expect("catalog.toml must parse");
        assert!(cfg.resonances.contains(&Resonance { revs: 149, closures: 2 }));
        assert_eq!(cfg.members_for(Resonance { revs: 111, closures: 2 }), 10);
    }
}
