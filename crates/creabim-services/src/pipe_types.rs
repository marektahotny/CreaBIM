//! Pipe and duct types as PilonDesigner catalogues them: each a series of
//! sizes in one material (copper STN EN 1057, HERZ PipeFix, PPR, steel
//! STN EN 10255, PP-HT drainage, Spiro ducts...), with the media it is for,
//! its roughness, joints, standards and, for manufacturers' series, their
//! fittings. The data is `data/pipe-types.json`, exported from
//! PilonDesigner's catalogue by `tools/pipe-types/export.sh`.

use crate::network;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// What a type of pipe carries; a system offers the types for its medium.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Medium {
    /// Water supply, cold and hot.
    Voda,
    /// Drainage.
    Kanal,
    /// Fuel gas.
    Plyn,
    /// Heating.
    Ukur,
    /// Cooling.
    Chlad,
    /// Medical gases.
    Medplyn,
    /// Fire water.
    Poziar,
    /// Flue.
    Dym,
    /// Ventilation ducts.
    Vzt,
}

/// One size in a series, mm.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Size {
    /// The size it is chosen and labelled by: the outside diameter of
    /// plastic and stainless pipe, the inner of copper, the DN of steel and
    /// drainage pipe, a duct's diameter or side.
    pub nominal_mm: f64,
    pub inner_mm: f64,
    pub outer_mm: f64,
    /// Weight, kg/m.
    pub kgm: Option<f64>,
    /// The manufacturer's segment ("SDR 11 PN16", "16 x 2").
    pub segment: Option<String>,
    /// The manufacturer's product code.
    pub code: Option<String>,
    /// Its DN, for steel and drainage pipe.
    pub dn: Option<f64>,
    /// The aluminium layer of a multilayer pipe.
    pub al_mm: Option<f64>,
    /// Water it holds, l/m.
    pub water_lm: Option<f64>,
    /// Supplied in coils of so many metres.
    pub coil_m: Option<f64>,
    /// Supplied in bars of so many metres.
    pub bar_m: Option<f64>,
    pub code_bar: Option<String>,
    /// Spacing of its fixings, m.
    pub fix_m: Option<f64>,
}

/// A fitting or adaptor from a manufacturer's catalogue: `d1` the size it
/// joins, `d2` the other (a tee's branch, a reducer's small end).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fitting {
    /// Elbow, Tee, Reducer, Coupling, PeCu, PeSteelBend, Flue...
    pub kind: String,
    pub name: String,
    pub code: String,
    pub d1: f64,
    pub d2: Option<f64>,
    pub angle: Option<f64>,
    pub thread: Option<String>,
    #[serde(rename = "D")]
    pub outer_d: Option<f64>,
    #[serde(rename = "L")]
    pub length: Option<f64>,
    pub kg: Option<f64>,
}

/// How a pipe that bends is bent: an arc rather than an elbow up to this
/// outside diameter, of radius `r_tool`·d with a bender, `r_hand`·d by hand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bend {
    pub max_outer_mm: f64,
    pub r_tool: f64,
    pub r_hand: f64,
}

/// How a size is labelled on drawings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelBy {
    /// Outside diameter × wall, "16x2".
    #[default]
    Ds,
    /// "DN 15 (21,3)".
    Dn,
    /// The segment's name ("PP/AL koax 60/100").
    Segment,
}

/// A duct's section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DuctShape {
    Round,
    Rect,
}

/// A type of pipe or duct: one series of sizes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PipeType {
    pub id: String,
    /// Its name, in Slovak: "Meď STN EN 1057".
    pub name: String,
    /// Its material, as a calculation tells materials apart.
    pub material: String,
    /// Its material, short, for a drawing's labels.
    pub short: Option<String>,
    pub manufacturer: Option<String>,
    pub roughness_mm: f64,
    pub sizes: Vec<Size>,
    pub media: Vec<Medium>,
    pub density_kg_dm3: Option<f64>,
    pub joint: String,
    pub standards: String,
    pub note: Option<String>,
    pub label: Option<LabelBy>,
    pub fittings: Vec<Fitting>,
    pub shape: Option<DuctShape>,
    pub flex: bool,
    pub bend: Option<Bend>,
    /// Supplied in bars of so many metres, joined by couplings.
    pub bar_m: Option<f64>,
    pub max_temp_c: Option<f64>,
    pub max_bar: Option<f64>,
}

/// Every type, in PilonDesigner's order: the first for a medium is its
/// default.
pub fn all() -> &'static [PipeType] {
    static ALL: OnceLock<Vec<PipeType>> = OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str(include_str!("../data/pipe-types.json"))
            .expect("data/pipe-types.json is a list of pipe types")
    })
}

/// The type with this id ("Cu", "HERZ", "PPHT"...).
pub fn by_id(id: &str) -> Option<&'static PipeType> {
    all().iter().find(|t| t.id == id)
}

/// The types for a medium, the default first.
pub fn for_medium(m: Medium) -> impl Iterator<Item = &'static PipeType> {
    all().iter().filter(move |t| t.media.contains(&m))
}

/// A number as Slovak writes it: "21,3".
fn sk(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    let s = if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    };
    s.replace('.', ",")
}

impl PipeType {
    pub fn is_duct(&self) -> bool {
        self.shape.is_some()
    }

    /// The size nearest a nominal size.
    pub fn size(&self, nominal_mm: f64) -> Option<&Size> {
        self.sizes.iter().min_by(|a, b| {
            (a.nominal_mm - nominal_mm)
                .abs()
                .total_cmp(&(b.nominal_mm - nominal_mm).abs())
        })
    }

    /// The size nearest an outside diameter: where a run changes material.
    pub fn closest_outer(&self, outer_mm: f64) -> Option<&Size> {
        self.sizes.iter().min_by(|a, b| {
            (a.outer_mm - outer_mm)
                .abs()
                .total_cmp(&(b.outer_mm - outer_mm).abs())
        })
    }

    /// A size as a drawing labels it: "16x2", "DN 15 (21,3)", "Ø160",
    /// "400×200" (a rectangular duct's `height_mm`, else square).
    pub fn size_label(&self, nominal_mm: f64, height_mm: Option<f64>) -> String {
        let Some(s) = self.size(nominal_mm) else {
            return sk(nominal_mm);
        };
        match self.shape {
            Some(DuctShape::Rect) => {
                return format!(
                    "{}×{}",
                    sk(s.nominal_mm),
                    sk(height_mm.unwrap_or(s.nominal_mm))
                )
            }
            Some(DuctShape::Round) => return format!("Ø{}", sk(s.nominal_mm)),
            None => {}
        }
        match (self.label.unwrap_or_default(), s.dn, &s.segment) {
            (LabelBy::Dn, Some(dn), _) => format!("DN {} ({})", sk(dn), sk(s.outer_mm)),
            (LabelBy::Segment, _, Some(seg)) => match seg.find(" (") {
                Some(i) if seg.ends_with(')') => seg[..i].trim_end().to_string(),
                _ => seg.clone(),
            },
            _ => format!(
                "{}x{}",
                sk(s.outer_mm),
                sk(((s.outer_mm - s.inner_mm) / 2.0 * 10.0).round() / 10.0)
            ),
        }
    }

    /// Weight of a size, kg/m: the catalogue's, else from the density.
    pub fn kg_per_m(&self, nominal_mm: f64) -> Option<f64> {
        let s = self.size(nominal_mm)?;
        s.kgm.or_else(|| {
            let rho = self.density_kg_dm3?;
            Some(
                std::f64::consts::PI * (s.outer_mm * s.outer_mm - s.inner_mm * s.inner_mm) / 4.0
                    * rho
                    / 1000.0,
            )
        })
    }

    /// Its sizes as a calculation chooses among them.
    pub fn calc_sizes(&self) -> Vec<network::PipeSize> {
        self.sizes
            .iter()
            .map(|s| network::PipeSize {
                nominal_mm: s.nominal_mm,
                inner_mm: s.inner_mm,
            })
            .collect()
    }

    /// Its roughness, m.
    pub fn roughness_m(&self) -> f64 {
        self.roughness_mm / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_reads_whole() {
        let all = all();
        assert_eq!(all.len(), 30);
        let mut ids: Vec<_> = all.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 30, "ids are unique");
        for t in all {
            assert!(!t.sizes.is_empty() && !t.media.is_empty(), "{}", t.id);
            assert!(t.roughness_mm > 0.0, "{}", t.id);
            for w in t.sizes.windows(2) {
                assert!(w[0].nominal_mm < w[1].nominal_mm, "{} sizes rise", t.id);
            }
            for s in &t.sizes {
                assert!(
                    s.inner_mm > 0.0 && s.inner_mm <= s.outer_mm,
                    "{} {s:?}",
                    t.id
                );
            }
        }
        // manufacturers' fittings come with them
        assert!(by_id("HERZ").unwrap().fittings.len() > 50);
        // Wavin's water and heating: PP-R welded, Tigris multilayer pressed
        for id in ["WPPR", "WPPRCT", "WPPRB", "WTIGK", "WTIGM"] {
            let t = by_id(id).unwrap();
            assert!(t.sizes.len() >= 8, "{id}");
            for k in ["Elbow", "Tee", "Reducer", "Coupling"] {
                assert!(t.fittings.iter().any(|f| f.kind == k), "{id} {k}");
            }
        }
        // Wavin's drainage: pipes and the fittings it is built from
        for id in ["WAS", "WSI", "WHDPE"] {
            let t = by_id(id).unwrap();
            assert!(t.media.contains(&Medium::Kanal), "{id}");
            for k in ["Elbow", "Tee", "Reducer", "Coupling"] {
                assert!(t.fittings.iter().any(|f| f.kind == k), "{id} {k}");
            }
            assert!(t
                .sizes
                .iter()
                .all(|s| s.inner_mm < s.outer_mm && s.dn.is_some()));
        }
        assert!(by_id("CUMED")
            .unwrap()
            .fittings
            .iter()
            .any(|f| f.kind == "Tee"));
    }

    #[test]
    fn types_are_offered_by_medium() {
        let gas: Vec<_> = for_medium(Medium::Plyn).map(|t| t.id.as_str()).collect();
        assert!(gas.contains(&"HERZG") && gas.contains(&"PE100") && gas.contains(&"Cu"));
        assert!(!gas.contains(&"PPR"));
        assert_eq!(for_medium(Medium::Kanal).next().unwrap().id, "PPHT");
        assert!(for_medium(Medium::Vzt).all(|t| t.is_duct()));
    }

    #[test]
    fn sizes_are_labelled_as_drawings_write_them() {
        // PEX-AL-PEX, like copper, is chosen by its inner diameter
        assert_eq!(by_id("PEXAL").unwrap().size_label(12.0, None), "16x2");
        assert_eq!(
            by_id("OCEL").unwrap().size_label(15.0, None),
            "DN 15 (21,3)"
        );
        assert_eq!(by_id("VZK").unwrap().size_label(160.0, None), "Ø160");
        assert_eq!(
            by_id("VZH").unwrap().size_label(400.0, Some(200.0)),
            "400×200"
        );
        let cu = by_id("Cu").unwrap();
        // copper is chosen by its inner diameter
        assert_eq!(cu.size(13.0).unwrap().outer_mm, 15.0);
        assert_eq!(cu.size_label(13.0, None), "15x1");
        assert!(by_id("DYM")
            .unwrap()
            .size_label(100.0, None)
            .contains("60/100"));
    }

    #[test]
    fn sizes_feed_the_calculation() {
        let t = by_id("PPR").unwrap();
        let s = t.calc_sizes();
        assert_eq!(s.len(), t.sizes.len());
        assert_eq!(s[1].inner_mm, 13.2);
        assert!((t.roughness_m() - 7e-6).abs() < 1e-12);
        assert!(t.kg_per_m(20.0).unwrap() > 0.0);
    }
}
