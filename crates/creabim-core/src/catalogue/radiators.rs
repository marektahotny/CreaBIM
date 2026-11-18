//! Steel panel radiators (STN EN 442) by type, height and length: the
//! types 11, 21, 22 and 33 in the heights and lengths every maker's
//! range has. The outputs are the values usual for such radiators at
//! 75/65/20 °C: values to design with, to be replaced by the maker's own
//! once the product is chosen.
//!
//! A radiator is a catalogue object whose key says its size
//! (`radiator-22-600-1000`), so one standing in a model can be read back
//! ([`Panel::of_key`]).

use super::{appliance_white, chrome, cuboid, cylinder, rrect, soft_box, v2, v3, Build};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};

/// The panels and convector fins a radiator has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PanelKind {
    /// One panel, one row of fins.
    K11,
    /// Two panels, one row of fins.
    K21,
    /// Two panels, two rows of fins.
    K22,
    /// Three panels, three rows of fins.
    K33,
}

/// The heights of the range, mm.
pub const HEIGHTS: [u32; 5] = [300, 400, 500, 600, 900];

/// Bottom of a radiator above the floor, m.
pub const OFF_FLOOR: f64 = 0.15;

impl PanelKind {
    pub const ALL: [PanelKind; 4] = [
        PanelKind::K11,
        PanelKind::K21,
        PanelKind::K22,
        PanelKind::K33,
    ];

    /// Its number: 11, 21, 22 or 33.
    pub fn number(self) -> u32 {
        match self {
            PanelKind::K11 => 11,
            PanelKind::K21 => 21,
            PanelKind::K22 => 22,
            PanelKind::K33 => 33,
        }
    }

    pub fn of_number(n: u32) -> Option<PanelKind> {
        PanelKind::ALL.into_iter().find(|k| k.number() == n)
    }

    /// How many panels.
    pub fn panels(self) -> u32 {
        self.number() / 10
    }

    /// Depth off the wall's face to its front, m.
    pub fn depth(self) -> f64 {
        match self {
            PanelKind::K11 => 0.061,
            PanelKind::K21 => 0.077,
            PanelKind::K22 => 0.100,
            PanelKind::K33 => 0.155,
        }
    }

    /// The exponent of its characteristic, Φ = Φn · (Δθ / 50)ⁿ.
    pub fn exponent(self) -> f64 {
        match self {
            PanelKind::K11 => 1.30,
            PanelKind::K21 => 1.31,
            PanelKind::K22 => 1.33,
            PanelKind::K33 => 1.34,
        }
    }

    /// Output of a metre of length at 75/65/20 °C (Δθ = 50 K), W, for
    /// each of [`HEIGHTS`].
    fn per_metre(self) -> [f64; 5] {
        match self {
            PanelKind::K11 => [577.0, 733.0, 880.0, 1020.0, 1410.0],
            PanelKind::K21 => [762.0, 962.0, 1150.0, 1329.0, 1826.0],
            PanelKind::K22 => [1004.0, 1263.0, 1489.0, 1700.0, 2300.0],
            PanelKind::K33 => [1430.0, 1790.0, 2110.0, 2400.0, 3240.0],
        }
    }
}

/// The lengths of the range, mm: by 100 to 1200, by 200 to 3000.
pub fn lengths() -> Vec<u32> {
    (4..=12)
        .map(|i| i * 100)
        .chain((7..=15).map(|i| i * 200))
        .collect()
}

/// One radiator of the range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub kind: PanelKind,
    /// mm.
    pub height: u32,
    /// mm.
    pub length: u32,
}

impl Panel {
    /// Its output at 75/65/20 °C, W. The height nearest in the range
    /// stands for one outside it.
    pub fn nominal_w(&self) -> f64 {
        let i = (0..HEIGHTS.len())
            .min_by_key(|i| HEIGHTS[*i].abs_diff(self.height))
            .unwrap_or(3);
        self.kind.per_metre()[i] * self.length as f64 / 1000.0
    }

    /// Its output at the mean temperature difference `dt` K to the
    /// room, W.
    pub fn output_w(&self, dt: f64) -> f64 {
        if dt <= 0.0 {
            return 0.0;
        }
        self.nominal_w() * (dt / 50.0).powf(self.kind.exponent())
    }

    /// As makers write it: 22-600×1000.
    pub fn label(&self) -> String {
        format!("{}-{}×{}", self.kind.number(), self.height, self.length)
    }

    /// Its catalogue key.
    pub fn key(&self) -> String {
        format!(
            "radiator-{}-{}-{}",
            self.kind.number(),
            self.height,
            self.length
        )
    }

    /// The radiator a catalogue key names; the starter catalogue's plain
    /// `radiator` is a 22-600×800.
    pub fn of_key(key: &str) -> Option<Panel> {
        let key = key.rsplit('/').next().unwrap_or(key);
        if key == "radiator" {
            return Some(Panel {
                kind: PanelKind::K22,
                height: 600,
                length: 800,
            });
        }
        let mut it = key.strip_prefix("radiator-")?.split('-');
        let kind = PanelKind::of_number(it.next()?.parse().ok()?)?;
        let height = it.next()?.parse().ok()?;
        let length = it.next()?.parse().ok()?;
        (it.next().is_none() && height > 0 && length > 0).then_some(Panel {
            kind,
            height,
            length,
        })
    }

    /// As a catalogue object: its back to the wall on y = 0, its bottom
    /// 150 mm off the floor, a thermostatic valve on its left and the
    /// return valve on its right.
    pub fn object(&self) -> CatalogueObject {
        let (w, h) = (self.length as f64 / 1000.0, self.height as f64 / 1000.0);
        let d = self.kind.depth();
        let n = self.kind.panels();
        // the panels spread through the depth, 30 mm clear of the wall
        let gap = 0.03;
        let t = 0.022_f64.min((d - gap) / n as f64);
        let step = if n > 1 {
            (d - gap - t) / (n - 1) as f64
        } else {
            0.0
        };
        let mut b = Build::new();
        for i in 0..n {
            let y1 = -gap - step * i as f64;
            b.add(
                "Panel",
                appliance_white(),
                soft_box(v3(-w / 2.0, y1 - t, 0.0), v3(w / 2.0, y1, h), 0.005, 0.004),
                false,
            );
        }
        let bx = (w / 2.0 - 0.1).max(0.05);
        for x in [-bx, bx] {
            b.add(
                "Bracket",
                appliance_white(),
                cuboid(v3(x - 0.02, -gap, h - 0.1), v3(x + 0.02, 0.0, h - 0.06)),
                false,
            );
        }
        let y = -(gap + d) / 2.0;
        b.add(
            "Thermostatic valve",
            appliance_white(),
            cylinder(v2(-w / 2.0 - 0.03, y), 0.017, 0.0, 0.1),
            false,
        );
        b.add(
            "Valve",
            chrome(),
            cylinder(v2(w / 2.0 + 0.03, y), 0.012, 0.0, 0.06),
            false,
        );
        // its outline, hatched across
        b.outline(rrect(v2(0.0, -(gap + d) / 2.0), w, d - gap + 0.01, 0.0, 0));
        let lines = ((w / 0.1).round() as usize).max(4);
        for i in 0..lines {
            let x = -w / 2.0 + w * (i as f64 + 0.5) / lines as f64;
            b.line(v2(x - 0.03, -d), v2(x + 0.03, -gap));
        }
        b.at(OFF_FLOOR);
        b.finish(
            &self.key(),
            &format!("Radiator {}", self.label()),
            &format!(
                "Steel panel radiator type {}, {} mm high and {} mm long, {} W at 75/65/20 °C, with a thermostatic valve",
                self.kind.number(),
                self.height,
                self.length,
                self.nominal_w().round()
            ),
            ObjectCategory::Hvac,
            ("IfcSpaceHeater", "RADIATOR"),
            Mount::Wall,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_radiator_reads_back_from_its_key() {
        let p = Panel {
            kind: PanelKind::K22,
            height: 600,
            length: 1000,
        };
        assert_eq!(p.key(), "radiator-22-600-1000");
        assert_eq!(Panel::of_key(&p.key()), Some(p));
        assert_eq!(Panel::of_key("starter/radiator").unwrap().length, 800);
        assert_eq!(Panel::of_key("radiator-12-600-1000"), None);
        assert_eq!(Panel::of_key("towel-radiator"), None);
        assert_eq!(p.label(), "22-600×1000");
    }

    #[test]
    fn output_follows_the_characteristic() {
        let p = Panel {
            kind: PanelKind::K22,
            height: 600,
            length: 1000,
        };
        assert_eq!(p.nominal_w(), 1700.0);
        assert!((p.output_w(50.0) - 1700.0).abs() < 1e-6);
        // 55/45/20: Δθ 30 K gives about half
        let low = p.output_w(30.0);
        assert!(low > 820.0 && low < 880.0, "{low}");
        assert_eq!(p.output_w(-1.0), 0.0);
        // more panels and more height give more
        for h in HEIGHTS {
            let w: Vec<f64> = PanelKind::ALL
                .iter()
                .map(|k| {
                    Panel {
                        kind: *k,
                        height: h,
                        length: 1000,
                    }
                    .nominal_w()
                })
                .collect();
            assert!(w.windows(2).all(|x| x[0] < x[1]), "{w:?}");
        }
        assert_eq!(lengths().first(), Some(&400));
        assert_eq!(lengths().last(), Some(&3000));
    }

    #[test]
    fn the_object_is_a_heater_as_long_as_its_panels() {
        let p = Panel {
            kind: PanelKind::K33,
            height: 500,
            length: 1400,
        };
        let o = p.object();
        assert_eq!(Panel::of_key(&o.key), Some(p));
        let panels = o.object.parts.iter().filter(|x| x.name == "Panel").count();
        assert_eq!(panels, 3);
    }
}
