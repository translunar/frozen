use std::io::Write;
use std::path::Path;

use elfo_core::constants::{
    A_EM_KM, GM_EARTH_KM3S2, GM_MOON_KM3S2, MOON_C22, MOON_J2, MOON_J3, R_MOON_KM,
};
use elfo_core::forces::ForceModel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementsOut {
    pub a_km: f64,
    pub e: f64,
    pub i_deg: f64,
    pub omega_deg: f64,
    pub raan_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberOut {
    pub index: usize,
    pub state0: [f64; 6],
    pub period_s: f64,
    pub period_nd: f64,
    pub elements: ElementsOut,
    pub nu1: f64,
    pub nu2: f64,
    pub r_peri_km: f64,
    pub r_apo_km: f64,
    pub residual: f64,
    pub traj: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyOut {
    /// Revolutions per closure period, `M` of the `M:k` resonance.
    pub resonance_n: u32,
    /// Node closures per period, `k` of the `M:k` resonance. `1` for every family
    /// that existed before rational resonances, and defaulted to `1` on read so a
    /// consumer written against schema_version 1 (the web app, until it is taught
    /// about `k`) sees an unchanged field set plus one it can ignore.
    #[serde(default = "one_closure")]
    pub closures: u32,
    pub members: Vec<MemberOut>,
    pub preview: String,
    pub preview_counts: Vec<u32>,
}

fn one_closure() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize)]
pub struct ComboOut {
    pub id: String,
    pub name: String,
    pub terms: ForceModel,
    pub families: Vec<FamilyOut>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub date: String,
    pub git_hash: String,
}

/// Write a trajectory as little-endian f32 xyz km triples, concatenated with
/// no header. Creates parent directories as needed.
pub fn write_f32(path: &Path, pts: &[[f64; 3]]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut buf = Vec::with_capacity(pts.len() * 3 * std::mem::size_of::<f32>());
    for p in pts {
        for &c in p {
            buf.extend_from_slice(&(c as f32).to_le_bytes());
        }
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(&buf)?;
    Ok(())
}

/// Decimate a trajectory to at most `max_points` points, evenly spaced across
/// the full range and always including the final point. Used to build
/// per-family preview tracks.
///
/// A plain `i * (len/max_points)` stride from the start never lands exactly on
/// the last sample, so the previous version silently dropped the trajectory's
/// closing point. Spacing indices across `[0, len-1]` instead (rather than
/// `[0, len)`) fixes that at the same cost.
pub fn decimate(pts: &[[f64; 3]], max_points: usize) -> Vec<[f64; 3]> {
    if max_points == 0 || pts.is_empty() {
        return Vec::new();
    }
    if pts.len() <= max_points {
        return pts.to_vec();
    }
    if max_points == 1 {
        return vec![pts[0]];
    }
    let last = pts.len() - 1;
    let step = last as f64 / (max_points - 1) as f64;
    (0..max_points)
        .map(|i| pts[((i as f64 * step).round() as usize).min(last)])
        .collect()
}

/// Write `catalog.json` at the root of `out_dir`, echoing the physical
/// constants used to generate the catalog for provenance.
pub fn write_catalog(
    out_dir: &Path,
    combos: Vec<ComboOut>,
    provenance: Provenance,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(out_dir)?;

    let catalog = serde_json::json!({
        "schema_version": 1,
        "generated": {
            "date": provenance.date,
            "git_hash": provenance.git_hash,
        },
        "constants": {
            "GM_EARTH_KM3S2": GM_EARTH_KM3S2,
            "GM_MOON_KM3S2": GM_MOON_KM3S2,
            "A_EM_KM": A_EM_KM,
            "R_MOON_KM": R_MOON_KM,
            "MOON_J2": MOON_J2,
            "MOON_C22": MOON_C22,
            "MOON_J3": MOON_J3,
            "source": "DE440 / GRGM1200-series",
        },
        "combos": combos,
    });

    let text = serde_json::to_string_pretty(&catalog)?;
    std::fs::write(out_dir.join("catalog.json"), text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_round_trip_and_catalog_json_shape() {
        let dir = tempfile::tempdir().unwrap();
        // write a 3-point trajectory, read bytes back, check little-endian f32 km
        let pts = vec![[1000.0f64, -2000.0, 3000.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        write_f32(&dir.path().join("t.f32"), &pts).unwrap();
        let bytes = std::fs::read(dir.path().join("t.f32")).unwrap();
        assert_eq!(bytes.len(), 3 * 3 * 4);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 1000.0);
        // catalog.json minimal shape
        write_catalog(dir.path(), vec![], Provenance { date: "d".into(), git_hash: "h".into() }).unwrap();
        let v: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("catalog.json")).unwrap()).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert!(v["combos"].as_array().unwrap().is_empty());
    }

    #[test]
    fn family_without_closures_reads_back_as_single_closure() {
        // A schema_version-1 catalog.json (written before rational resonances) has no
        // `closures` key. Reading one must yield the N:1 meaning, not a 0:0 family.
        let fam: FamilyOut = serde_json::from_str(
            r#"{"resonance_n":25,"members":[],"preview":"full/n25/preview.f32",
                "preview_counts":[]}"#,
        )
        .unwrap();
        assert_eq!((fam.resonance_n, fam.closures), (25, 1));
        // and a rational family round-trips through the same JSON
        let dual = FamilyOut { closures: 2, ..fam };
        let text = serde_json::to_string(&dual).unwrap();
        assert!(text.contains("\"closures\":2"), "{text}");
        assert_eq!(serde_json::from_str::<FamilyOut>(&text).unwrap().closures, 2);
    }

    #[test]
    fn decimate_includes_final_point() {
        let pts: Vec<[f64; 3]> = (0..1000).map(|i| [i as f64, 0.0, 0.0]).collect();
        let dec = decimate(&pts, 37);
        assert_eq!(dec.len(), 37);
        assert_eq!(dec[0], pts[0]);
        assert_eq!(dec[36], pts[999], "decimate must include the trajectory's final point");
    }
}
