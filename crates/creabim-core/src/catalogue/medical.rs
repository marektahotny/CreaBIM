//! Medical supply units, as PilonDesigner's `medunit.ts` (the Pilon
//! Hydraulika plugin's MedUnitConfig and MedUnitLayout): the wall rail
//! NR-H3 and the fixed ceiling bridge SM-P 5.4, with their medical gas
//! outlets (STN EN ISO 9170-1), plain and backed-up sockets, data, USB,
//! nurse call and light switch (STN EN ISO 11197). The outlets are laid
//! out along the profile, the unit gets its code, its 3D model and a
//! connection for each gas where the pipes join it.

use super::{cuboid, look, Build, Look};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::Hatch;
use glam::DVec3;

/// Which unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedUnitKind {
    /// Wall rail NR-H3.
    Rail,
    /// Fixed ceiling bridge SM-P 5.4.
    Bridge,
}

impl MedUnitKind {
    pub fn name(self) -> &'static str {
        match self {
            MedUnitKind::Rail => "Nástenná rampa NR-H3",
            MedUnitKind::Bridge => "Stropný most pevný SM-P 5.4",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            MedUnitKind::Rail => "NR-H3",
            MedUnitKind::Bridge => "SM-P 5.4",
        }
    }
}

/// The gases a unit can carry, in the order they stand on it.
pub const MED_GASES: [&str; 5] = ["O2", "AIR", "VAC", "N2O", "AGSS"];

/// Which of a unit's parts it is made with: Flídr medical builds the
/// rail and the bridge to the room's needs, so each part can be left out
/// (its count is kept, to come back when the part is switched on again).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MedParts {
    /// The outlets of each of [`MED_GASES`].
    pub gases: [bool; 5],
    pub sockets: bool,
    pub sockets_backed: bool,
    /// Weak current: data RJ45.
    pub data: bool,
    pub usb: bool,
    /// The indirect light (up to the ceiling).
    pub light_indirect: bool,
    /// The direct light (down to the bed).
    pub light_direct: bool,
    /// The night orientation light.
    pub light_night: bool,
    /// The stainless medical rail 25 × 10 (medilišta) for accessories.
    pub med_rail: bool,
}

impl MedParts {
    /// Every part on.
    pub fn all() -> Self {
        Self {
            gases: [true; 5],
            sockets: true,
            sockets_backed: true,
            data: true,
            usb: true,
            light_indirect: true,
            light_direct: true,
            light_night: true,
            med_rail: true,
        }
    }
}

impl Default for MedParts {
    fn default() -> Self {
        Self::all()
    }
}

/// A unit's catalogue set by the Flídr medical catalogue
/// (flidr-medical-katalog-sortimentu.pdf, "Nástěnná rampa NR-H3" and
/// "Stropní mosty – pevné SM-P 5.4").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MedCatalogue {
    pub length_mm: f64,
    /// The rail's height or the bridge's depth, mm.
    pub size_mm: f64,
    pub gas_outlets: u32,
    pub agss: u32,
    pub sockets: u32,
    pub weak: u32,
    pub usb: u32,
    /// Indirect, direct and night light, W (LED); 0 = none.
    pub light_w: [f64; 3],
    /// Load on the medical rail, kg.
    pub rail_load_kg: f64,
}

impl MedUnitKind {
    /// Its catalogue set.
    pub fn catalogue(self) -> MedCatalogue {
        match self {
            MedUnitKind::Rail => MedCatalogue {
                length_mm: 1500.0,
                size_mm: 233.0,
                gas_outlets: 6,
                agss: 0,
                sockets: 6,
                weak: 4,
                usb: 2,
                light_w: [27.0, 9.0, 2.0],
                rail_load_kg: 50.0,
            },
            MedUnitKind::Bridge => MedCatalogue {
                length_mm: 2000.0,
                size_mm: 310.0,
                gas_outlets: 8,
                agss: 1,
                sockets: 24,
                weak: 2,
                usb: 2,
                light_w: [24.0, 10.0, 2.0],
                rail_load_kg: 300.0,
            },
        }
    }

    /// Whether it can carry the gas `MED_GASES[i]`: the NR-H3 has no
    /// AGSS outlet in the catalogue.
    pub fn carries(self, i: usize) -> bool {
        !(self == MedUnitKind::Rail && i == 4)
    }
}

/// What a unit is fitted with.
#[derive(Clone, Debug, PartialEq)]
pub struct MedUnitConfig {
    pub kind: MedUnitKind,
    pub length_mm: f64,
    /// Outlets of each of [`MED_GASES`].
    pub gases: [u32; 5],
    pub sockets: u32,
    /// Sockets on the backed-up supply.
    pub sockets_backed: u32,
    pub data: u32,
    pub usb: u32,
    pub nurse: bool,
    pub light: bool,
    /// The gases on the left, the electrics on the right.
    pub gases_left: bool,
    /// A bridge with outlets on both sides.
    pub two_sided: bool,
    /// The rail's axis, or the bridge's underside, above the floor, m.
    pub height: f64,
    /// The ceiling a bridge hangs from, above the floor, m.
    pub ceiling: f64,
    /// Which parts it is made with.
    pub on: MedParts,
}

impl MedUnitConfig {
    /// The usual unit of `kind`.
    pub fn usual(kind: MedUnitKind) -> Self {
        match kind {
            MedUnitKind::Bridge => Self {
                kind,
                length_mm: 2000.0,
                gases: [3, 2, 3, 0, 1],
                sockets: 12,
                sockets_backed: 12,
                data: 2,
                usb: 2,
                nurse: false,
                light: true,
                gases_left: true,
                two_sided: false,
                height: 1.85,
                ceiling: 2.8,
                on: MedParts::all(),
            },
            MedUnitKind::Rail => Self {
                kind,
                length_mm: 1500.0,
                gases: [1, 1, 1, 0, 0],
                sockets: 4,
                sockets_backed: 2,
                data: 2,
                usb: 2,
                nurse: true,
                light: true,
                gases_left: true,
                two_sided: false,
                height: 1.45,
                ceiling: 2.8,
                on: MedParts::all(),
            },
        }
    }

    /// The unit as made: the counts of the parts switched off are 0.
    pub fn fitted(&self) -> Self {
        let mut c = self.clone();
        let on = self.on;
        for i in 0..5 {
            if !on.gases[i] || !self.kind.carries(i) {
                c.gases[i] = 0;
            }
        }
        let off = |n: &mut u32, b: bool| {
            if !b {
                *n = 0
            }
        };
        off(&mut c.sockets, on.sockets);
        off(&mut c.sockets_backed, on.sockets_backed);
        off(&mut c.data, on.data);
        off(&mut c.usb, on.usb);
        c
    }

    /// Its gases and how many outlets of each.
    pub fn gas_counts(&self) -> Vec<(&'static str, u32)> {
        let f = self.fitted();
        MED_GASES
            .iter()
            .zip(f.gases)
            .filter(|(_, n)| *n > 0)
            .map(|(g, n)| (*g, n))
            .collect()
    }

    /// Its lights and medical rail left out, by their marks in the code:
    /// SN indirect, SP direct, SO night light, ML medical rail.
    pub fn left_out(&self) -> Vec<&'static str> {
        let on = self.on;
        [
            (on.light_indirect, "SN"),
            (on.light_direct, "SP"),
            (on.light_night, "SO"),
            (on.med_rail, "ML"),
        ]
        .iter()
        .filter(|(b, _)| !*b)
        .map(|(_, m)| *m)
        .collect()
    }

    /// The unit's code as the plugin writes it:
    /// `NR-H3 1500 O2-1 AIR-1 VAC-1 Z4 ZZ2 D2 U2 S`.
    pub fn code(&self) -> String {
        let f = self.fitted();
        let bridge = self.kind == MedUnitKind::Bridge;
        let gases: Vec<String> = self
            .gas_counts()
            .iter()
            .map(|(g, n)| format!("{g}-{n}"))
            .collect();
        let mut s = format!(
            "{} {:.0} {} Z{} ZZ{} D{}",
            self.kind.short(),
            layout(self).length,
            gases.join(" "),
            f.sockets,
            f.sockets_backed,
            f.data
        );
        if f.usb > 0 {
            s += &format!(" U{}", f.usb);
        }
        if self.nurse {
            s += " S";
        }
        if bridge && self.two_sided {
            s += " 2S";
        }
        if !self.gases_left {
            s += " P";
        }
        if bridge {
            s += &format!(" H{:.0}", self.ceiling * 1000.0);
        }
        let out = self.left_out();
        if !out.is_empty() {
            s += &format!(" bez {}", out.join(" "));
        }
        s
    }

    /// The height of its underside above the floor, m.
    pub fn underside(&self) -> f64 {
        match self.kind {
            MedUnitKind::Bridge => self.height,
            MedUnitKind::Rail => (self.height - layout(self).height / 2000.0).max(0.0),
        }
    }
}

/// What stands on the profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedItemKind {
    /// An outlet of the gas `MED_GASES[i]`.
    Gas(usize),
    Socket,
    SocketBacked,
    Data,
    Usb,
    Nurse,
    Ground,
    Switch,
}

impl MedItemKind {
    /// Its width, height and the pitch it takes in its row, mm.
    pub fn size(self) -> (f64, f64, f64) {
        match self {
            MedItemKind::Gas(_) => (52.0, 52.0, 76.0),
            MedItemKind::Socket | MedItemKind::SocketBacked => (46.0, 46.0, 50.0),
            MedItemKind::Data | MedItemKind::Usb => (22.0, 22.0, 26.0),
            MedItemKind::Nurse => (50.0, 50.0, 56.0),
            MedItemKind::Ground => (12.0, 12.0, 18.0),
            MedItemKind::Switch => (24.0, 24.0, 30.0),
        }
    }

    /// Its colour: the gases' by STN EN ISO 9170-1 and EN 1089-3.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            MedItemKind::Gas(0) => [255, 255, 255],
            MedItemKind::Gas(1) => [31, 41, 55],
            MedItemKind::Gas(2) => [250, 204, 21],
            MedItemKind::Gas(3) => [37, 99, 235],
            MedItemKind::Gas(_) => [162, 28, 175],
            MedItemKind::Socket => [243, 244, 246],
            MedItemKind::SocketBacked => [220, 38, 38],
            MedItemKind::Data => [51, 65, 85],
            MedItemKind::Usb => [71, 85, 105],
            MedItemKind::Nurse => [249, 115, 22],
            MedItemKind::Ground => [22, 163, 74],
            MedItemKind::Switch => [229, 231, 235],
        }
    }

    fn name(self) -> String {
        match self {
            MedItemKind::Gas(i) => format!("Outlet {}", MED_GASES[i.min(4)]),
            MedItemKind::Socket => "Socket".into(),
            MedItemKind::SocketBacked => "Socket, backed up".into(),
            MedItemKind::Data => "Data".into(),
            MedItemKind::Usb => "USB".into(),
            MedItemKind::Nurse => "Nurse call".into(),
            MedItemKind::Ground => "Equipotential pin".into(),
            MedItemKind::Switch => "Light switch".into(),
        }
    }
}

/// One thing on the profile: `x` from the middle of its length, `z` from
/// its underside, mm; `back` on a two-sided bridge's far side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MedItem {
    pub kind: MedItemKind,
    pub x: f64,
    pub z: f64,
    pub back: bool,
}

/// A unit laid out, mm.
#[derive(Clone, Debug, PartialEq)]
pub struct MedLayout {
    pub length: f64,
    pub height: f64,
    pub depth: f64,
    pub items: Vec<MedItem>,
    /// Whether it was made longer than asked, to hold its outlets.
    pub lengthened: bool,
}

const MARGIN: f64 = 90.0;
const GROUP_GAP: f64 = 60.0;

/// Where everything stands on the profile (the plugin's
/// `MedUnitLayout.Compute`).
pub fn layout(c: &MedUnitConfig) -> MedLayout {
    let c = &c.fitted();
    let bridge = c.kind == MedUnitKind::Bridge;
    let (height, depth) = if bridge {
        (230.0, 310.0)
    } else {
        (233.0, 105.0)
    };
    let (z_gas, z_el) = if bridge { (80.0, 165.0) } else { (95.0, 168.0) };
    let sides = if bridge && c.two_sided { 2 } else { 1 };
    let group = |k: MedItemKind, n: u32, z: f64| -> Vec<MedItem> {
        (0..n)
            .map(|_| MedItem {
                kind: k,
                x: 0.0,
                z,
                back: false,
            })
            .collect()
    };
    let width = |gs: &[Vec<MedItem>]| -> f64 {
        gs.iter()
            .flat_map(|g| g.iter().map(|i| i.kind.size().2))
            .sum::<f64>()
            + gs.len().saturating_sub(1) as f64 * GROUP_GAP
    };
    // rows: their groups, whether they start from the left, their side
    let mut rows: Vec<(Vec<Vec<MedItem>>, bool, bool)> = vec![];
    for s in 0..sides {
        let part = |n: u32| match (sides, s) {
            (1, _) => n,
            (_, 0) => n.div_ceil(2),
            _ => n / 2,
        };
        let mut gas = vec![];
        for (i, n) in c.gases.iter().enumerate() {
            gas.extend(group(MedItemKind::Gas(i), part(*n), z_gas));
        }
        let mut el: Vec<Vec<MedItem>> = vec![];
        if part(c.sockets) > 0 {
            let mut g = group(MedItemKind::Socket, part(c.sockets), z_el);
            g.extend(group(MedItemKind::Ground, 2, z_el));
            el.push(g);
        }
        let mut low = group(MedItemKind::Data, part(c.data), z_el);
        low.extend(group(MedItemKind::Usb, part(c.usb), z_el));
        if c.nurse && s == 0 {
            low.extend(group(MedItemKind::Nurse, 1, z_el));
        }
        if c.light && s == 0 {
            low.extend(group(MedItemKind::Switch, 1, z_el));
        }
        if !low.is_empty() {
            el.push(low);
        }
        if part(c.sockets_backed) > 0 {
            let mut g = group(MedItemKind::SocketBacked, part(c.sockets_backed), z_el);
            g.extend(group(MedItemKind::Ground, 2, z_el));
            el.push(g);
        }
        if !gas.is_empty() {
            rows.push((vec![gas], c.gases_left, s == 1));
        }
        if !el.is_empty() {
            rows.push((el, !c.gases_left, s == 1));
        }
    }
    let need = 2.0 * MARGIN + rows.iter().map(|r| width(&r.0)).fold(0.0, f64::max);
    let mut length = c.length_mm.max(if bridge { 700.0 } else { 600.0 });
    let lengthened = need > length;
    if lengthened {
        length = (need / 100.0).ceil() * 100.0;
    }
    let mut items = vec![];
    for (groups, from_left, back) in rows {
        let w = width(&groups);
        let mut x = if from_left {
            -length / 2.0 + MARGIN
        } else {
            length / 2.0 - MARGIN - w
        };
        for g in groups {
            for mut it in g {
                let p = it.kind.size().2;
                it.x = x + p / 2.0;
                it.back = back;
                if it.kind == MedItemKind::Ground {
                    it.z -= 36.0;
                }
                items.push(it);
                x += p;
            }
            x += GROUP_GAP;
        }
    }
    MedLayout {
        length,
        height,
        depth,
        items,
        lengthened,
    }
}

fn plastic(name: &str, rgb: [u8; 3]) -> Look {
    look(name, "plastic", Hatch::Empty, rgb, 0.5)
}

/// The catalogue key of the unit `c`: the same unit is the same type.
pub fn key(c: &MedUnitConfig) -> String {
    let code: String = c
        .code()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    format!("med-{}-{:.0}", code.to_lowercase(), c.height * 100.0)
}

/// The unit as a catalogue object: the rail with its back on the wall,
/// the bridge hung from the ceiling on two hangers; a `Connection <gas>`
/// stands on top over each gas's outlets, where its pipe joins.
pub fn unit(c: &MedUnitConfig) -> CatalogueObject {
    let l = layout(c);
    let bridge = c.kind == MedUnitKind::Bridge;
    let m = |v: f64| v / 1000.0;
    let (len, d, h) = (m(l.length), m(l.depth), m(l.height));
    // the profile's front and back faces: its back at y = 0, out is −y;
    // a bridge's hangers stand on its middle
    let (front, back) = (-d, 0.0);
    let mid = -d / 2.0;
    let top = if bridge {
        (h + 0.1).max(c.ceiling - c.height)
    } else {
        h
    };
    let mut b = Build::new();
    if bridge {
        // from the ceiling down to its top
        b.at(0.0);
    } else {
        b.at(c.underside());
    }
    b.add(
        "Profile",
        look(
            "Aluminium, anodised",
            "metal",
            Hatch::Solid,
            [170, 180, 192],
            0.4,
        ),
        cuboid(
            DVec3::new(-len / 2.0, front, 0.0),
            DVec3::new(len / 2.0, back, h),
        ),
        true,
    );
    if bridge {
        for x in [-len / 2.0 + 0.12, len / 2.0 - 0.12] {
            b.add(
                "Hanger",
                look(
                    "Steel, painted grey",
                    "metal",
                    Hatch::Solid,
                    [148, 163, 184],
                    0.5,
                ),
                cuboid(
                    DVec3::new(x - 0.04, mid - 0.04, h),
                    DVec3::new(x + 0.04, mid + 0.04, top),
                ),
                true,
            );
        }
    }
    // the lights by the catalogue: the indirect one on top, up to the
    // ceiling; the direct one on the underside, down to the bed; the
    // night light low on the front
    let on = c.on;
    let diffuser = plastic("Diffuser, opal PMMA", [250, 250, 236]);
    let e = 0.006;
    if on.light_indirect {
        b.add(
            "Indirect light",
            diffuser.clone(),
            cuboid(
                DVec3::new(-len / 2.0 + 0.06, front + 0.02, h),
                DVec3::new(len / 2.0 - 0.06, back - 0.02, h + e),
            ),
            false,
        );
    }
    if on.light_direct {
        let (y0, y1) = if bridge {
            (mid - 0.03, mid + 0.03)
        } else {
            (front + 0.015, front + 0.05)
        };
        b.add(
            "Direct light",
            diffuser.clone(),
            cuboid(
                DVec3::new(-len / 6.0, y0, 0.0),
                DVec3::new(len / 6.0, y1, e),
            ),
            false,
        );
    }
    if on.light_night {
        b.add(
            "Night light",
            diffuser,
            cuboid(
                DVec3::new(len / 2.0 - 0.13, front - 0.004, 0.012),
                DVec3::new(len / 2.0 - 0.07, front, 0.032),
            ),
            false,
        );
    }
    // the stainless medical rail 25 × 10 along the front's top, on a
    // two-sided bridge on its back too
    if on.med_rail {
        let steel = look(
            "Stainless steel",
            "metal",
            Hatch::Solid,
            [200, 204, 208],
            0.3,
        );
        let mut faces = vec![(front - 0.010, front)];
        if bridge && c.two_sided {
            faces.push((back, back + 0.010));
        }
        for (y0, y1) in faces {
            b.add(
                "Medical rail 25 × 10",
                steel.clone(),
                cuboid(
                    DVec3::new(-len / 2.0 + 0.02, y0, h - 0.035),
                    DVec3::new(len / 2.0 - 0.02, y1, h - 0.010),
                ),
                false,
            );
        }
    }
    for it in &l.items {
        let (w, ih, _) = it.kind.size();
        let (y0, y1) = if it.back {
            (back, back + 0.008)
        } else {
            (front - 0.008, front)
        };
        let name = it.kind.name();
        b.add(
            &name,
            plastic(
                &format!("Medical unit, {}", name.to_lowercase()),
                it.kind.rgb(),
            ),
            cuboid(
                DVec3::new(m(it.x - w / 2.0), y0, m(it.z - ih / 2.0)),
                DVec3::new(m(it.x + w / 2.0), y1, m(it.z + ih / 2.0)),
            ),
            false,
        );
    }
    // a connection over each gas's outlets: at the wall on a rail, on
    // the hangers' line on a bridge
    for (i, gas) in MED_GASES.iter().enumerate() {
        let xs: Vec<f64> = l
            .items
            .iter()
            .filter(|it| it.kind == MedItemKind::Gas(i) && !it.back)
            .map(|it| it.x)
            .collect();
        if xs.is_empty() {
            continue;
        }
        let x = m(xs.iter().sum::<f64>() / xs.len() as f64);
        // a bridge's stand beside its hangers and end at the ceiling
        let (y, z0, z1) = if bridge {
            (mid + 0.06, top - 0.03, top)
        } else {
            (-0.025, top, top + 0.03)
        };
        b.add(
            &format!("Connection {gas}"),
            look("Copper", "metal", Hatch::Solid, [184, 115, 51], 0.3),
            cuboid(
                DVec3::new(x - 0.006, y - 0.006, z0),
                DVec3::new(x + 0.006, y + 0.006, z1),
            ),
            false,
        );
    }
    let name = format!("{} {:.0} mm", c.kind.name(), l.length);
    b.finish(
        &key(c),
        &name,
        &format!(
            "{}: medical gas outlets (STN EN ISO 9170-1), sockets, data and nurse call on one profile (STN EN ISO 11197){}; each gas joins it on top, Cu 12 × 1; Flídr medical",
            c.code(),
            lights(c)
        ),
        ObjectCategory::Medical,
        ("IfcMedicalDevice", "USERDEFINED"),
        if bridge { Mount::Ceiling } else { Mount::Wall },
    )
}

/// Its lights and medical rail as the description names them, with the
/// catalogue's power.
fn lights(c: &MedUnitConfig) -> String {
    let w = c.kind.catalogue().light_w;
    let on = c.on;
    let mut v = vec![];
    for (b, name, p) in [
        (on.light_indirect, "indirect light", w[0]),
        (on.light_direct, "direct light", w[1]),
        (on.light_night, "night light", w[2]),
    ] {
        if b && p > 0.0 {
            v.push(format!("{name} {p:.0} W LED"));
        }
    }
    if on.med_rail {
        v.push(format!(
            "stainless medical rail 25 × 10 ({:.0} kg)",
            c.kind.catalogue().rail_load_kg
        ));
    }
    if v.is_empty() {
        String::new()
    } else {
        format!(", {}", v.join(", "))
    }
}

/// The usual rail and the usual bridge.
pub fn starters() -> Vec<CatalogueObject> {
    vec![
        unit(&MedUnitConfig::usual(MedUnitKind::Rail)),
        unit(&MedUnitConfig::usual(MedUnitKind::Bridge)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usual_units_are_the_plugins() {
        let r = MedUnitConfig::usual(MedUnitKind::Rail);
        assert_eq!(r.code(), "NR-H3 1500 O2-1 AIR-1 VAC-1 Z4 ZZ2 D2 U2 S");
        let l = layout(&r);
        assert_eq!((l.length, l.height, l.depth), (1500.0, 233.0, 105.0));
        // gases from the left, electrics up to the right margin
        let gas: Vec<f64> = l
            .items
            .iter()
            .filter(|i| matches!(i.kind, MedItemKind::Gas(_)))
            .map(|i| i.x)
            .collect();
        assert_eq!(gas, vec![-622.0, -546.0, -470.0]);
        let right = l.items.iter().map(|i| i.x).fold(f64::MIN, f64::max);
        assert!((right - (750.0 - 90.0 - 9.0)).abs() < 1e-9, "{right}");
        assert!((r.underside() - (1.45 - 0.1165)).abs() < 1e-9);
        let b = MedUnitConfig::usual(MedUnitKind::Bridge);
        assert_eq!(
            b.code(),
            "SM-P 5.4 2000 O2-3 AIR-2 VAC-3 AGSS-1 Z12 ZZ12 D2 U2 H2800"
        );
        assert!(!layout(&b).lengthened);
    }

    #[test]
    fn a_unit_too_short_for_its_outlets_is_made_longer() {
        let mut c = MedUnitConfig::usual(MedUnitKind::Rail);
        c.length_mm = 600.0;
        c.sockets = 10;
        let l = layout(&c);
        assert!(l.lengthened && l.length > 600.0 && l.length % 100.0 == 0.0);
        let half = l.length / 2.0;
        assert!(l.items.iter().all(|i| i.x.abs() < half - 80.0));
    }

    #[test]
    fn the_units_are_catalogue_objects_with_a_connection_for_each_gas() {
        for c in starters() {
            c.object.validate().unwrap();
            assert_eq!(c.object.category, ObjectCategory::Medical);
            assert!(c.key.starts_with("starter/med-"), "{}", c.key);
        }
        let rail = unit(&MedUnitConfig::usual(MedUnitKind::Rail));
        let conns: Vec<&str> = rail
            .object
            .parts
            .iter()
            .filter_map(|p| p.name.strip_prefix("Connection "))
            .collect();
        assert_eq!(conns, vec!["O2", "AIR", "VAC"]);
        assert_eq!(rail.object.mount, Mount::Wall);
        // its back on the wall, 105 mm out
        let (lo, hi) = rail.object.bounds();
        // its back on the wall, the medical rail 10 mm on its 105 mm front
        assert!(hi.y.abs() < 1e-9 && (lo.y + 0.115).abs() < 1e-9, "{lo}");
        let bridge = unit(&MedUnitConfig::usual(MedUnitKind::Bridge));
        assert_eq!(bridge.object.mount, Mount::Ceiling);
        // hung 0.95 m: from the ceiling at 2.8 m to its underside at 1.85
        assert!((bridge.object.bounds().1.z - 0.95).abs() < 1e-9);
    }

    #[test]
    fn parts_switched_off_are_left_out_and_their_counts_kept() {
        let mut c = MedUnitConfig::usual(MedUnitKind::Bridge);
        c.on.gases[4] = false;
        c.on.sockets_backed = false;
        c.on.light_indirect = false;
        c.on.med_rail = false;
        assert_eq!(
            c.code(),
            "SM-P 5.4 2000 O2-3 AIR-2 VAC-3 Z12 ZZ0 D2 U2 H2800 bez SN ML"
        );
        // the counts come back when switched on again
        assert_eq!((c.gases[4], c.sockets_backed), (1, 12));
        let names = |c: &MedUnitConfig| -> Vec<String> {
            unit(c)
                .object
                .parts
                .iter()
                .map(|p| p.name.clone())
                .collect()
        };
        let n = names(&c);
        assert!(!n
            .iter()
            .any(|p| p == "Indirect light" || p.starts_with("Medical rail")));
        assert!(n.iter().any(|p| p == "Direct light") && n.iter().any(|p| p == "Night light"));
        assert!(!n
            .iter()
            .any(|p| p == "Outlet AGSS" || p == "Socket, backed up"));
        let all = names(&MedUnitConfig::usual(MedUnitKind::Bridge));
        assert!(
            all.iter().any(|p| p == "Indirect light") && all.iter().any(|p| p == "Outlet AGSS")
        );
        // the NR-H3 has no AGSS in the catalogue
        let mut r = MedUnitConfig::usual(MedUnitKind::Rail);
        r.gases[4] = 2;
        assert!(!r.code().contains("AGSS"));
        assert_eq!(MedUnitKind::Rail.catalogue().light_w, [27.0, 9.0, 2.0]);
    }
}
