//! Forced extraction of single rooms by their own fans (a bathroom run
//! now and then, a windowless plant room run all the time): the fans
//! makers sell with their characteristic curves, and the check of every
//! fan drawn on an exhaust duct: the air its room needs, the pressure
//! its ducts, bends, valve and outlet take, the point the fan really
//! runs at, and the undercut of the door the air comes in by.
//!
//! STN 73 4301:2005 čl. 4.2.2.8 recommends forced extraction for rooms
//! with a WC and bathrooms; it gives no flow, so the room's own need is
//! taken (its type's, [`creabim_core::roomtype`], or the flow typed on
//! the valve).

use crate::{airbalance, ductsizing, heatloss};
use creabim_core::mep::{Pipe, SystemKind, Valve};
use creabim_core::{ElementId, ElementKind, Model, OpeningKind};
use glam::{DVec2, DVec3};

const RHO: f64 = 1.2;
/// How near two ends must be to join, m.
const JOIN: f64 = 0.02;

/// One speed of a fan: its maker's data and its curve, static pressure
/// (Pa) against flow (m³/h), flow rising.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Speed {
    pub name: &'static str,
    pub rpm: f64,
    /// Flow blowing free (0 Pa), m³/h.
    pub q_free: f64,
    pub watts: f64,
    pub curve: &'static [(f64, f64)],
}

impl Speed {
    /// The static pressure the fan gives at `q` m³/h, Pa; 0 past its
    /// curve. Below the curve's first point it holds that point's
    /// pressure (the maker draws nothing lower).
    pub fn pressure(&self, q: f64) -> f64 {
        let c = self.curve;
        if q <= c[0].0 {
            return c[0].1;
        }
        for w in c.windows(2) {
            let ((q0, p0), (q1, p1)) = (w[0], w[1]);
            if q <= q1 {
                return p0 + (p1 - p0) * (q - q0) / (q1 - q0);
            }
        }
        0.0
    }
}

/// A fan a maker sells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fan {
    /// Its id, as a valve keeps it in `product`.
    pub id: &'static str,
    pub maker: &'static str,
    pub name: &'static str,
    /// In the ceiling or a wall of the room (it is the room's grille
    /// too), or in the duct.
    pub in_room: bool,
    pub dn: f64,
    pub speeds: &'static [Speed],
    /// What it is, as the maker says: controls, protection, noise.
    pub about: &'static str,
    /// Price without VAT and its currency, where a shop shows it.
    pub price: Option<(f64, &'static str)>,
    pub source: &'static str,
}

impl Fan {
    pub fn speed(&self, name: Option<&str>) -> &'static Speed {
        name.and_then(|n| self.speeds.iter().find(|s| s.name == n))
            .unwrap_or(&self.speeds[0])
    }
}

/// The fans, Soler & Palau (sold in Slovakia and Czechia). Curves read
/// off the makers' charts, every 4 m³/h.
pub const FANS: &[Fan] = &[
    Fan {
        id: "SP-TD160-100-SILENT",
        maker: "Soler & Palau",
        name: "TD-160/100 N SILENT",
        in_room: false,
        dn: 100.0,
        speeds: &[
            Speed {
                name: "VO",
                rpm: 2400.0,
                q_free: 180.0,
                watts: 29.0,
                curve: &[
                    (4.0, 108.0),
                    (8.0, 103.7),
                    (12.0, 99.0),
                    (16.0, 94.5),
                    (20.0, 89.3),
                    (24.0, 84.2),
                    (32.0, 73.8),
                    (40.0, 66.1),
                    (48.0, 62.4),
                    (56.0, 59.9),
                    (64.0, 58.9),
                    (72.0, 58.9),
                    (80.0, 58.7),
                    (88.0, 57.2),
                    (96.0, 55.2),
                    (104.0, 53.7),
                    (112.0, 52.6),
                    (120.0, 50.6),
                    (128.0, 46.3),
                    (136.0, 39.6),
                    (144.0, 32.5),
                    (152.0, 25.7),
                    (160.0, 18.1),
                    (168.0, 11.3),
                    (176.0, 3.0),
                    (180.0, 0.0),
                ],
            },
            Speed {
                name: "NO",
                rpm: 2220.0,
                q_free: 150.0,
                watts: 18.0,
                curve: &[
                    (4.0, 88.0),
                    (8.0, 84.2),
                    (12.0, 79.7),
                    (16.0, 75.4),
                    (20.0, 70.7),
                    (24.0, 66.4),
                    (32.0, 58.0),
                    (40.0, 52.1),
                    (48.0, 49.2),
                    (56.0, 47.9),
                    (64.0, 47.2),
                    (72.0, 46.1),
                    (80.0, 43.8),
                    (88.0, 41.8),
                    (96.0, 39.6),
                    (104.0, 36.7),
                    (112.0, 32.7),
                    (120.0, 26.3),
                    (128.0, 19.2),
                    (136.0, 11.2),
                    (144.0, 5.3),
                    (152.0, 0.0),
                ],
            },
        ],
        about: "Potrubný diagonálny ventilátor Ø100, 230 V, 2 otáčky (VO 0,17 A, NO 0,11 A), tepelná ochrana, IP44, 1,4 kg, -20 až +40 °C; ovládanie spínačom s dobehom alebo hygrostatom (príslušenstvo)",
        price: Some((1721.0, "Kč")),
        source: "S&P katalóg MIXVENT-TD Silent 160–1000 (elektrodesign.cz), krivka TD-160/100 N SILENT; cena czvzduchotechnika.cz 11/2026",
    },
    Fan {
        id: "SP-SILENT100-CHZ",
        maker: "Soler & Palau",
        name: "SILENT-100 CHZ",
        in_room: true,
        dn: 100.0,
        speeds: &[Speed {
            name: "",
            rpm: 2400.0,
            q_free: 95.0,
            watts: 8.0,
            curve: &[
                (20.0, 39.7),
                (24.0, 38.0),
                (28.0, 35.8),
                (32.0, 33.5),
                (36.0, 30.4),
                (38.0, 26.8),
                (40.0, 24.9),
                (48.0, 24.1),
                (56.0, 24.7),
                (60.0, 24.4),
                (64.0, 23.8),
                (68.0, 22.2),
                (72.0, 19.9),
                (76.0, 16.7),
                (80.0, 13.3),
                (84.0, 9.5),
                (88.0, 5.4),
                (92.0, 0.8),
                (93.0, 0.0),
            ],
        }],
        about: "Stropný / stenový axiálny ventilátor Ø100 s hygrostatom a dobehom 1–30 min, spätná klapka, guľôčkové ložiská, 230 V, IP45, 0,57 kg; 26,5 dB(A) v 3 m na strane sania",
        price: Some((197.52, "€")),
        source: "S&P list SILENT 100 (kvelektro.cz), krivka SILENT-100 (czvzduchotechnika.cz); cena czvzduchotechnika.cz 11/2026",
    },
];

/// The fan a shared branch of several rooms is designed with.
pub const CENTRAL_FAN: &str = "SP-TD160-100-SILENT";

pub fn fan(id: &str) -> Option<&'static Fan> {
    FANS.iter().find(|f| f.id == id)
}

/// The fan a valve is: its product, else a fan in the room where it
/// stands at a duct's free end, else one in the duct.
pub fn fan_of(v: &Valve, free_end: bool) -> &'static Fan {
    v.product
        .as_deref()
        .and_then(fan)
        .unwrap_or(if free_end { &FANS[1] } else { &FANS[0] })
}

/// How the check is made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    /// The loss of one bend, ζ (designer's: the makers' fittings data
    /// for Spiro bends are not in the app yet).
    pub zeta_bend: f64,
    /// The loss of the extract valve, ζ on the duct's velocity.
    pub zeta_valve: f64,
    /// The outlet's loss, ζ on the duct's velocity (1 is the air leaving
    /// at the duct's speed, nothing for the hood).
    pub zeta_outlet: f64,
    /// Air velocity under the door it comes in by, m/s.
    pub transfer_v: f64,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            zeta_bend: 0.5,
            zeta_valve: 0.0,
            zeta_outlet: 1.0,
            transfer_v: 1.0,
        }
    }
}

/// Where the air of a fan comes in: an extract valve or the fan itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Inlet {
    pub id: ElementId,
    pub room: Option<(ElementId, String, String)>,
    /// What its room needs from it, m³/h.
    pub need: f64,
    /// What it gets at the fan's working point, m³/h.
    pub flow: f64,
    pub length: f64,
    pub bends: u32,
    /// The door undercut its room needs, mm, and the door's width, m.
    pub undercut: Option<(f64, f64)>,
    /// What its valve must throttle beyond its own way's loss so the
    /// fan's air shares as the rooms need, Pa (0 on the worst way).
    pub throttle: f64,
}

/// One fan checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: ElementId,
    pub fan: &'static Fan,
    pub speed: &'static Speed,
    pub inlets: Vec<Inlet>,
    pub need: f64,
    /// The ducts' loss at the need, Pa, and what the fan gives there.
    pub dp_need: f64,
    pub p_need: f64,
    /// Where the fan's curve meets the ducts': flow, m³/h, and pressure, Pa.
    pub q: f64,
    pub p: f64,
    /// An outlet (a hood or a free end) was found.
    pub outlet: bool,
    pub notes: Vec<String>,
}

impl Row {
    pub fn ok(&self) -> bool {
        self.outlet && self.need > 0.0 && self.q + 0.5 >= self.need
    }
}

struct Seg {
    a: usize,
    b: usize,
    d_mm: f64,
    len: f64,
}

/// A round duct's diameter for the friction, mm: a rectangular one's
/// equivalent.
fn diameter(p: &Pipe) -> f64 {
    match p.height_mm {
        Some(h) => {
            let w = p.nominal_mm;
            1.3 * (w * h).powf(0.625) / (w + h).powf(0.25)
        }
        None => {
            if p.inner_mm > 1.0 {
                p.inner_mm
            } else {
                p.nominal_mm
            }
        }
    }
}

/// The exhaust ducts as a graph: points, segments, what stands at each
/// point (the valves on them).
struct Graph {
    points: Vec<DVec3>,
    segs: Vec<Seg>,
    adj: Vec<Vec<usize>>,
    /// Valves on exhaust ducts: id, valve, its point, level.
    valves: Vec<(ElementId, Valve, usize, ElementId)>,
}

fn graph(model: &Model) -> Graph {
    let pipes: Vec<(ElementId, &Pipe, DVec3, DVec3)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) if p.kind() == SystemKind::AirExhaust && p.length() > 1e-6 => {
                let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
                Some((e.id, p, p.start + z, p.end + z))
            }
            _ => None,
        })
        .collect();
    let valves: Vec<(ElementId, Valve, DVec3, ElementId)> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Valve(v) = &e.kind else {
                return None;
            };
            let (_, p, a, b) = pipes.iter().find(|p| p.0 == v.pipe)?;
            Some((e.id, v.clone(), a.lerp(*b, v.t.clamp(0.0, 1.0)), p.level))
        })
        .collect();
    let mut cuts: Vec<DVec3> = pipes.iter().flat_map(|p| [p.2, p.3]).collect();
    cuts.extend(valves.iter().map(|v| v.2));
    let mut points: Vec<DVec3> = vec![];
    let mut point = |p: DVec3| -> usize {
        if let Some(i) = points.iter().position(|q| q.distance(p) <= JOIN) {
            return i;
        }
        points.push(p);
        points.len() - 1
    };
    let mut segs = vec![];
    for (_, p, a, b) in &pipes {
        let d = *b - *a;
        let len = d.length();
        let mut ts: Vec<f64> = cuts
            .iter()
            .filter_map(|q| {
                let t = (*q - *a).dot(d) / (len * len);
                (t * len > JOIN && (1.0 - t) * len > JOIN && (*a + d * t).distance(*q) <= JOIN)
                    .then_some(t)
            })
            .collect();
        ts.sort_by(f64::total_cmp);
        let mut at = *a;
        for t in ts.into_iter().chain([1.0]) {
            let to = a.lerp(*b, t);
            if at.distance(to) > JOIN {
                segs.push(Seg {
                    a: point(at),
                    b: point(to),
                    d_mm: diameter(p),
                    len: at.distance(to),
                });
            }
            at = to;
        }
    }
    let valves = valves
        .into_iter()
        .map(|(id, v, at, level)| (id, v, point(at), level))
        .collect();
    let mut adj = vec![vec![]; points.len()];
    for (k, s) in segs.iter().enumerate() {
        adj[s.a].push(k);
        adj[s.b].push(k);
    }
    Graph {
        points,
        segs,
        adj,
        valves,
    }
}

impl Graph {
    /// The segments from `from` to `to` in order, by breadth first.
    fn path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let mut prev: Vec<Option<usize>> = vec![None; self.points.len()];
        let mut seen = vec![false; self.points.len()];
        let mut queue = std::collections::VecDeque::from([from]);
        seen[from] = true;
        while let Some(i) = queue.pop_front() {
            if i == to {
                let mut out = vec![];
                let mut at = to;
                while let Some(k) = prev[at] {
                    out.push(k);
                    let s = &self.segs[k];
                    at = if s.a == at { s.b } else { s.a };
                }
                out.reverse();
                return Some(out);
            }
            for &k in &self.adj[i] {
                let s = &self.segs[k];
                let j = if s.a == i { s.b } else { s.a };
                if !seen[j] {
                    seen[j] = true;
                    prev[j] = Some(k);
                    queue.push_back(j);
                }
            }
        }
        None
    }

    fn reach(&self, from: usize) -> Vec<bool> {
        let mut seen = vec![false; self.points.len()];
        let mut stack = vec![from];
        seen[from] = true;
        while let Some(i) = stack.pop() {
            for &k in &self.adj[i] {
                let s = &self.segs[k];
                let j = if s.a == i { s.b } else { s.a };
                if !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        seen
    }

    /// The bends along a path: where the way turns more than 30°.
    fn bends(&self, from: usize, path: &[usize]) -> u32 {
        let mut at = from;
        let mut dirs = vec![];
        for &k in path {
            let s = &self.segs[k];
            let to = if s.a == at { s.b } else { s.a };
            dirs.push((self.points[to] - self.points[at]).normalize_or_zero());
            at = to;
        }
        dirs.windows(2)
            .filter(|w| w[0].dot(w[1]) < 30f64.to_radians().cos())
            .count() as u32
    }
}

fn dynamic(q: f64, d_mm: f64) -> f64 {
    let v = ductsizing::velocity(q, d_mm.max(1.0));
    RHO * v * v / 2.0
}

/// The room a point of level `level` stands in.
fn room_at(model: &Model, level: ElementId, at: DVec2) -> Option<(ElementId, String, String)> {
    model.iter().find_map(|e| {
        let ElementKind::Room(r) = &e.kind else {
            return None;
        };
        if r.level != level {
            return None;
        }
        let shape = creabim_core::room::shape_of(model, r)?;
        creabim_core::polygon::contains(&shape.outline, at)
            .then(|| (e.id, r.number.clone(), e.name.clone()))
    })
}

/// The doors of a room: their widths, m.
fn doors_of(model: &Model, room: ElementId) -> Vec<f64> {
    let Some(ElementKind::Room(r)) = model.get(room).map(|e| &e.kind) else {
        return vec![];
    };
    let Some(shape) = creabim_core::room::shape_of(model, r) else {
        return vec![];
    };
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind != OpeningKind::Door {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        if w.level != r.level {
            continue;
        }
        let along = (w.end - w.start).normalize_or_zero();
        let mid = w.start + along * (o.offset + o.width / 2.0);
        let d = w.thickness / 2.0 + 0.15;
        let n = along.perp();
        if creabim_core::polygon::contains(&shape.outline, mid + n * d)
            || creabim_core::polygon::contains(&shape.outline, mid - n * d)
        {
            out.push(o.width);
        }
    }
    out
}

/// What a room needs taken out, m³/h: its type's flow, else its air
/// change (the heat loss's least air).
fn room_need(model: &Model, ho: &heatloss::Opts, room: ElementId) -> f64 {
    heatloss::building(model, ho)
        .rooms
        .iter()
        .find(|r| r.id == room)
        .map_or(0.0, |r| r.v_min.round())
}

/// Every fan on the exhaust ducts, checked.
pub fn check(model: &Model, ho: &heatloss::Opts, o: &Opts) -> Vec<Row> {
    let g = graph(model);
    let mut out = vec![];
    for (fid, fv, fat, flevel) in g.valves.iter().filter(|v| v.1.kind == "ventilator") {
        let free_end = g.adj[*fat].len() == 1;
        let fan = fan_of(fv, free_end);
        let speed = fan.speed(fv.preset.as_deref());
        let reach = g.reach(*fat);
        let mut notes = vec![];
        // the outlet: a hood, else the highest free end with nothing on it
        let hood_v = g
            .valves
            .iter()
            .find(|v| v.1.kind == "vyfukVzt" && reach[v.2]);
        let hood = hood_v.map(|v| v.2);
        let hood_acc = hood_v.and_then(|v| v.1.product.as_deref()).and_then(valve);
        let taken: Vec<usize> = g.valves.iter().map(|v| v.2).collect();
        let outlet = hood.or_else(|| {
            (0..g.points.len())
                .filter(|&i| reach[i] && g.adj[i].len() == 1 && !taken.contains(&i))
                .max_by(|&i, &j| g.points[i].z.total_cmp(&g.points[j].z))
        });
        if let Some(a) = hood_acc.filter(|a| a.loss.is_empty()) {
            notes.push(format!(
                "{}: výrobca nedáva tlakovú stratu v číslach, počítaná ako výtok z potrubia (ζ {}).",
                a.name, o.zeta_outlet
            ));
        }
        if hood.is_none() && outlet.is_some() {
            notes.push("Výfuk: voľný koniec potrubia (bez hlavice).".to_string());
        }
        // the inlets: extract valves, else the fan in the room
        let mut ins: Vec<(ElementId, usize, ElementId, Option<f64>)> = g
            .valves
            .iter()
            .filter(|v| v.1.kind == "vyustO" && reach[v.2])
            .map(|v| (v.0, v.2, v.3, v.1.flow_m3h))
            .collect();
        if ins.is_empty() && fan.in_room {
            ins.push((*fid, *fat, *flevel, fv.flow_m3h));
        }
        if ins.is_empty() {
            notes.push("Na potrubí ventilátora nie je odvodný ventil ani mriežka.".to_string());
        }
        if fan.in_room && !free_end {
            notes.push(format!(
                "{} sa osádza do stropu alebo steny miestnosti: má stáť na konci potrubia.",
                fan.name
            ));
        }
        let mut inlets = vec![];
        let mut paths = vec![];
        for (id, at, level, given) in &ins {
            let p = g.points[*at];
            let room = room_at(model, *level, p.truncate());
            let need = given.unwrap_or_else(|| {
                if *id == *fid {
                    room.as_ref().map_or(0.0, |r| room_need(model, ho, r.0))
                } else {
                    airbalance::terminal_flow(model, ho, *id)
                }
            });
            let path = outlet.and_then(|to| g.path(*at, to)).unwrap_or_default();
            let length: f64 = path.iter().map(|&k| g.segs[k].len).sum();
            let bends = g.bends(*at, &path);
            let undercut = room.as_ref().and_then(|r| {
                let doors = doors_of(model, r.0);
                let w: f64 = doors.iter().sum();
                (w > 0.0 && need > 0.0).then(|| (need / 3600.0 / o.transfer_v / w * 1000.0, w))
            });
            if room.is_none() {
                notes.push("Odvodné miesto nestojí v žiadnej miestnosti.".to_string());
            } else if need <= 0.0 {
                notes.push(format!(
                    "Miestnosť {} nemá potrebu odvodu: nastavte jej typ (napr. Práčovňa) alebo prietok na ventile.",
                    room.as_ref().map_or(String::new(), |r| r.1.clone())
                ));
            }
            if room.is_some() && undercut.is_none() && need > 0.0 {
                notes
                    .push("Miestnosť nemá dvere: náhradný vzduch treba priviesť inak.".to_string());
            }
            inlets.push(Inlet {
                id: *id,
                room,
                need,
                flow: 0.0,
                length,
                bends,
                undercut,
                throttle: 0.0,
            });
            paths.push((*at, path));
        }
        let need: f64 = inlets.iter().map(|i| i.need).sum();
        let n = inlets.len().max(1) as f64;
        let share: Vec<f64> = inlets
            .iter()
            .map(|i| if need > 0.0 { i.need / need } else { 1.0 / n })
            .collect();
        // how much of the fan's air each segment carries
        let mut part = vec![0.0; g.segs.len()];
        for (k, (_, path)) in paths.iter().enumerate() {
            for &s in path {
                part[s] += share[k];
            }
        }
        let is_fan: Vec<bool> = inlets.iter().map(|i| i.id == *fid).collect();
        let accs: Vec<Option<&Accessory>> = ins
            .iter()
            .map(|i| {
                g.valves
                    .iter()
                    .find(|v| v.0 == i.0)
                    .and_then(|v| v.1.product.as_deref())
                    .and_then(valve)
            })
            .collect();
        for a in accs.iter().flatten().filter(|a| a.loss.is_empty()) {
            let n = format!(
                "{}: výrobca dáva tlakovú stratu len grafom, počítaná ζ {} (zadajte v okne).",
                a.name, o.zeta_valve
            );
            if !notes.contains(&n) {
                notes.push(n);
            }
        }
        let each = |q: f64| -> Vec<f64> {
            paths
                .iter()
                .enumerate()
                .map(|(k, (from, path))| {
                    let mut dp = 0.0;
                    let mut at = *from;
                    let mut last: Option<DVec3> = None;
                    for &s in path {
                        let seg = &g.segs[s];
                        let qs = q * part[s];
                        dp += ductsizing::friction(qs, seg.d_mm) * seg.len;
                        let to = if seg.a == at { seg.b } else { seg.a };
                        let dir = (g.points[to] - g.points[at]).normalize_or_zero();
                        if last.is_some_and(|l| l.dot(dir) < 30f64.to_radians().cos()) {
                            dp += o.zeta_bend * dynamic(qs, seg.d_mm);
                        }
                        last = Some(dir);
                        at = to;
                    }
                    let first = path.first().map(|&s| (q * part[s], g.segs[s].d_mm));
                    let lastseg = path.last().map(|&s| (q * part[s], g.segs[s].d_mm));
                    if let Some((_, d)) = first {
                        if !is_fan[k] {
                            let qi = q * share[k];
                            dp += match accs[k].filter(|a| !a.loss.is_empty()) {
                                Some(a) => a.dp(qi),
                                None => o.zeta_valve * dynamic(qi, d),
                            };
                        }
                    }
                    if let Some((qs, d)) = lastseg {
                        dp += match hood_acc.filter(|a| !a.loss.is_empty()) {
                            Some(a) => a.dp(qs),
                            None => o.zeta_outlet * dynamic(qs, d),
                        };
                    }
                    dp
                })
                .collect()
        };
        let system = |q: f64| -> f64 { each(q).into_iter().fold(0.0, f64::max) };
        // where the curves meet: the fan falls, the ducts rise
        let (mut lo, mut hi) = (0.0, speed.q_free);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if speed.pressure(mid) > system(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let q = if outlet.is_some() && !ins.is_empty() {
            lo
        } else {
            0.0
        };
        let (dp_need, p_need) = (system(need), speed.pressure(need));
        // what each valve must take up so the air shares as the rooms
        // need: the worst way's loss less its own, at the need
        let at_need = each(need);
        for (k, i) in inlets.iter_mut().enumerate() {
            i.flow = (q * share[k]).round();
            i.throttle = at_need.get(k).map_or(0.0, |own| dp_need - own);
        }
        if outlet.is_none() {
            notes.push("Potrubie nemá výfuk von (hlavicu ani voľný koniec).".to_string());
        }
        if need > 0.0 && q > need * 1.3 {
            notes.push(format!(
                "{} {}: dáva {:.0} m³/h pri potrebe {:.0} m³/h; priškrťte tanierovým ventilom (menšie x) alebo zvoľte nižšie otáčky.",
                fan.name, speed.name, q, need
            ));
        }
        out.push(Row {
            id: *fid,
            fan,
            speed,
            dp_need,
            p_need,
            need,
            q,
            p: speed.pressure(q),
            outlet: outlet.is_some(),
            inlets,
            notes,
        });
    }
    out
}

/// An extract valve or a roof outlet a maker sells, with its pressure
/// loss against flow (Pa, m³/h, flow rising).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accessory {
    pub id: &'static str,
    pub maker: &'static str,
    pub name: &'static str,
    pub dn: f64,
    pub loss: &'static [(f64, f64)],
    pub about: &'static str,
    pub price: Option<(f64, &'static str)>,
    pub source: &'static str,
}

impl Accessory {
    /// Its loss at `q` m³/h, Pa: between its points straight, past them
    /// with the square of the flow.
    pub fn dp(&self, q: f64) -> f64 {
        let c = self.loss;
        if c.is_empty() || q <= 0.0 {
            return 0.0;
        }
        if q <= c[0].0 {
            return c[0].1 * (q / c[0].0).powi(2);
        }
        for w in c.windows(2) {
            let ((q0, p0), (q1, p1)) = (w[0], w[1]);
            if q <= q1 {
                return p0 + (p1 - p0) * (q - q0) / (q1 - q0);
            }
        }
        let (ql, pl) = c[c.len() - 1];
        pl * (q / ql).powi(2)
    }
}

/// Extract valves (ceiling disc valves).
pub const VALVES: &[Accessory] = &[
    Accessory {
        id: "SA-EFF-100-SW",
        maker: "Systemair",
        name: "EFF-100-SW tanierový ventil odvodný",
        dn: 100.0,
        // read off the maker's chart "EFF-100-SW Pressure drop" (vector
        // curve of the PDF), setting +10 mm = fully open
        loss: &[
            (47.1, 10.7),
            (55.0, 14.6),
            (62.5, 18.7),
            (70.3, 23.4),
            (77.9, 28.8),
            (84.6, 33.6),
            (90.4, 38.4),
            (96.2, 43.4),
            (103.9, 50.3),
            (110.1, 56.6),
            (117.4, 63.9),
            (124.0, 71.2),
            (130.5, 78.9),
        ],
        about: "Kovový odvodný tanierový ventil (pozinkovaný plech, RAL 9003), Ø A 100, Ø B 135 mm, 0,2 kg; montážny rámček RF-BOR-C-100; strata pri nastavení +10 mm (úplne otvorený), pri 0 mm napr. 80 m³/h ≈ 76 Pa; LwA 25/30/35 dB pri 52/73/90 m³/h",
        price: None,
        source: "Systemair, EFF handbook 2023-08 (A-EFF_DATASHEET_ENG_GLO_HANDBOOK_EFF_202308.pdf), str. 9 graf EFF-100-SW",
    },
    Accessory {
    id: "ED-IT-PRO-100",
    maker: "Elektrodesign",
    name: "IT-PRO 100 tanierový ventil",
    dn: 100.0,
    // the maker gives the loss only as a chart (settings x = 6–17 mm)
    loss: &[],
    about: "Univerzálny plastový tanierový ventil (biele ABS) pre prívod aj odvod, s montážnym krúžkom do stropu alebo steny; Ø B 96, Ø C 152, Ø D 99, Ø G 146 mm; nastavenie vzdialenosti tanierika x = 6–17 mm",
    price: Some((147.0, "Kč")),
    source: "Elektrodesign katalóg IT-PRO (it-pro-en.pdf), e-shop elektrodesign.cz, kód AS100100010, 11/2026",
    },
];

/// Roof outlets.
pub const OUTLETS: &[Accessory] = &[Accessory {
    id: "KLOBER-ROVIA-KE0520",
    maker: "Klöber",
    name: "Rovia univerzálny strešný prestup DN100 (KE 0520)",
    dn: 100.0,
    // no loss data from the maker
    loss: &[],
    about: "Strešný odvetrávací prestup DN100 s hlavicou pre bežné betónové a pálené škridly, sklon strechy cca 20–50°",
    price: None,
    source: "Klöber Rovia, technický list (izomat.cz)",
}];

pub fn valve(id: &str) -> Option<&'static Accessory> {
    VALVES.iter().chain(OUTLETS).find(|a| a.id == id)
}

/// One line of the bill.
#[derive(Clone, Debug, PartialEq)]
pub struct BillLine {
    pub name: String,
    pub qty: f64,
    pub unit: &'static str,
    pub price: Option<(f64, &'static str)>,
    pub source: String,
}

/// The bill of the extracts, read from the model: fans, valves and
/// outlets by their product, and the ducts the fans are on.
pub fn bill(model: &Model) -> Vec<BillLine> {
    let g = graph(model);
    let mut lines: Vec<BillLine> = vec![];
    let mut add = |name: String, unit: &'static str, qty: f64, price, source: &str| {
        if let Some(l) = lines.iter_mut().find(|l| l.name == name) {
            l.qty += qty;
        } else {
            lines.push(BillLine {
                name,
                qty,
                unit,
                price,
                source: source.to_string(),
            });
        }
    };
    let mut on = vec![false; g.points.len()];
    for (_, v, at, _) in &g.valves {
        match v.kind.as_str() {
            "ventilator" => {
                let f = fan_of(v, g.adj[*at].len() == 1);
                add(
                    format!("Ventilátor {} {}", f.maker, f.name),
                    "ks",
                    1.0,
                    f.price,
                    f.source,
                );
                for (i, r) in g.reach(*at).into_iter().enumerate() {
                    on[i] |= r;
                }
            }
            "vyustO" | "vyfukVzt" => {
                if let Some(a) = v.product.as_deref().and_then(valve) {
                    add(
                        format!("{} {}", a.maker, a.name),
                        "ks",
                        1.0,
                        a.price,
                        a.source,
                    );
                }
            }
            _ => {}
        }
    }
    let mut by_dn: Vec<(f64, f64)> = vec![];
    for s in &g.segs {
        if on[s.a] {
            match by_dn.iter_mut().find(|x| (x.0 - s.d_mm).abs() < 0.5) {
                Some(x) => x.1 += s.len,
                None => by_dn.push((s.d_mm, s.len)),
            }
        }
    }
    by_dn.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (d, l) in by_dn {
        add(
            format!("Potrubie Spiro Ø{d:.0} (pozink, STN EN 1506) vrátane tvaroviek"),
            "m",
            l,
            None,
            "dĺžka z modelu",
        );
    }
    lines
}

// ---------------------------------------------------------------- design

/// What the design puts in its elements' names, so a new design
/// replaces the old.
pub const DESIGN_TAG: &str = "Odvetranie";

/// The rooms that can have their own extract fan: those air is taken
/// from (bathrooms, WCs, laundries, plant rooms, stores…).
pub fn candidates(model: &Model) -> Vec<(ElementId, String, String)> {
    let mut out: Vec<(ElementId, String, String)> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Room(r) = &e.kind else {
                return None;
            };
            let t = creabim_core::roomtype::RoomType::of(&r.occupancy, &e.name);
            (t.needs().exhaust && t != creabim_core::roomtype::RoomType::Kitchen)
                .then(|| (e.id, r.number.clone(), e.name.clone()))
        })
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// The top of the roof over `p`, absolute, m.
fn roof_top(model: &Model, p: DVec2) -> Option<f64> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Roof(r) => {
                let base = r.base(model);
                r.faces()
                    .into_iter()
                    .filter(|f| creabim_core::polygon::contains(&f.outline, p))
                    .map(|f| base + f.z(p) + r.vertical_thickness())
                    .fold(None, |m: Option<f64>, z| Some(m.map_or(z, |m| m.max(z))))
            }
            _ => None,
        })
        .fold(None, |m: Option<f64>, z| Some(m.map_or(z, |m| m.max(z))))
}

/// Where in a room the valve goes: the middle of its outline, pulled
/// inside where the middle falls out.
fn middle(outline: &[DVec2]) -> DVec2 {
    let n = outline.len().max(1) as f64;
    let c = outline.iter().copied().sum::<DVec2>() / n;
    if creabim_core::polygon::contains(outline, c) {
        return c;
    }
    outline
        .windows(3)
        .map(|w| (w[0] + w[1] + w[2]) / 3.0)
        .find(|p| creabim_core::polygon::contains(outline, *p))
        .unwrap_or(c)
}

/// The elements of an extract for each of `rooms`, straight up through
/// the roof: an extract valve in the ceiling, a Spiro duct Ø100 up into
/// the attic and 1.5 m on a level run towards the ridge (VZO) with the
/// fan `fan_id` on it, the duct on up through the
/// roof (VZE) and the roof outlet on its end. Elements of an earlier
/// design for the same rooms are given back to be removed.
pub fn propose(
    model: &Model,
    rooms: &[ElementId],
    fan_id: &str,
    speed: Option<&str>,
) -> (Vec<creabim_core::Element>, Vec<ElementId>, Vec<String>) {
    use creabim_core::Element;
    let fan = fan(fan_id).unwrap_or(&FANS[0]);
    let mut add = vec![];
    let mut warn = vec![];
    let mut tags = vec![];
    for id in rooms {
        let Some(e) = model.get(*id) else { continue };
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let Some(shape) = creabim_core::room::shape_of(model, r) else {
            warn.push(format!("{} {}: miestnosť nemá obrys.", r.number, e.name));
            continue;
        };
        let elev = model.level(r.level).map_or(0.0, |l| l.elevation);
        let at = middle(&shape.outline);
        let h = creabim_core::room::height(model, r);
        let ceiling_top = creabim_core::room::ceiling_over(model, r.level, at, elev + h + 0.01)
            .and_then(|(cid, z)| match model.get(cid).map(|c| &c.kind) {
                Some(ElementKind::Ceiling(c)) => Some(z + c.thickness - elev),
                _ => None,
            })
            .unwrap_or(h + 0.3);
        let Some(top) = roof_top(model, at).map(|z| z - elev) else {
            warn.push(format!(
                "{} {}: nad miestnosťou nie je strecha.",
                r.number, e.name
            ));
            continue;
        };
        let tag = format!("{DESIGN_TAG} {}", r.number);
        // in the attic towards where the roof rises most, so the fan sits
        // on a level run and the outlet comes out higher up the roof
        let run = 1.5;
        let dir = [DVec2::X, -DVec2::X, DVec2::Y, -DVec2::Y]
            .into_iter()
            .filter_map(|d| roof_top(model, at + d * run).map(|z| (d, z)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(DVec2::X, |x| x.0);
        let m = at + dir * run;
        let top_m = roof_top(model, m).map_or(top, |z| z - elev);
        let z_run = ceiling_top + 0.35;
        let a = DVec3::new(at.x, at.y, h);
        let b = DVec3::new(at.x, at.y, z_run);
        let c = DVec3::new(m.x, m.y, z_run);
        let d = DVec3::new(m.x, m.y, top_m.max(z_run + 0.5));
        let mut duct = |system: &str, from: DVec3, to: DVec3| {
            let p = Pipe {
                level: r.level,
                system: system.into(),
                start: from,
                end: to,
                pipe_type: "VZK".into(),
                nominal_mm: fan.dn,
                outer_mm: fan.dn + 1.0,
                inner_mm: fan.dn,
                height_mm: None,
                // through the cold attic: insulated against condensation
                insulation_mm: 30.0,
                insulation_set: true,
                mark: None,
            };
            let el = Element::new(
                format!("{tag} – potrubie Ø{:.0}", fan.dn),
                ElementKind::Pipe(p),
            );
            let pid = el.id;
            add.push(el);
            pid
        };
        let up = duct("VZO", a, b);
        let level_run = duct("VZO", b, c);
        let out = duct("VZE", c, d);
        // a fan in the ceiling is the room's grille too; one in the
        // duct takes the air from an extract valve
        let mut fv = if fan.in_room {
            Valve::new(up, 0.0, "ventilator")
        } else {
            Valve::new(level_run, 0.5, "ventilator")
        };
        fv.product = Some(fan.id.into());
        fv.preset = speed.filter(|s| !s.is_empty()).map(str::to_string);
        if !fan.in_room {
            let mut valve = Valve::new(up, 0.0, "vyustO");
            valve.product = Some(VALVES[0].id.into());
            add.push(Element::new(
                format!("{tag} – odvodný ventil"),
                ElementKind::Valve(valve),
            ));
        }
        add.push(Element::new(
            format!("{tag} – ventilátor {}", fan.name),
            ElementKind::Valve(fv),
        ));
        let mut hood = Valve::new(out, 1.0, "vyfukVzt");
        hood.product = Some(OUTLETS[0].id.into());
        add.push(Element::new(
            format!("{tag} – strešná hlavica"),
            ElementKind::Valve(hood),
        ));
        tags.push(tag);
    }
    let numbers: Vec<String> = tags
        .iter()
        .map(|t| t.trim_start_matches(DESIGN_TAG).trim().to_string())
        .collect();
    let remove = model
        .iter()
        .filter(|e| of_design(&e.name, &numbers))
        .map(|e| e.id)
        .collect();
    (add, remove, warn)
}

/// Whether an element belongs to an earlier design for one of the rooms
/// numbered `numbers`: its name "Odvetranie 1.07 – …" or, of a shared
/// branch, "Odvetranie 1.07+1.08+1.12 – …".
fn of_design(name: &str, numbers: &[String]) -> bool {
    let Some(rest) = name.strip_prefix(&format!("{DESIGN_TAG} ")) else {
        return false;
    };
    let Some((head, _)) = rest.split_once(" – ") else {
        return false;
    };
    head.split('+')
        .any(|n| numbers.iter().any(|m| m == n.trim()))
}

/// The elements of one shared extract for all of `rooms` (George
/// 2026-11-25): an extract valve in the ceiling of each, a Spiro Ø100 up
/// into the attic and square across to one collecting duct, the
/// collecting duct sized by the air it carries (STN EN 1506 sizes, at
/// most [`ductsizing::Limits`]' main velocity), on past the last branch
/// towards where the roof rises most with the fan `fan_id` on it, then up
/// through the roof (VZE) to one roof outlet. The valves are set to share
/// the air by each room's need. Elements of earlier designs for the same
/// rooms are given back to be removed.
pub fn propose_central(
    model: &Model,
    ho: &heatloss::Opts,
    rooms: &[ElementId],
    fan_id: &str,
    speed: Option<&str>,
) -> (Vec<creabim_core::Element>, Vec<ElementId>, Vec<String>) {
    use creabim_core::Element;
    let fan = fan(fan_id).unwrap_or(&FANS[0]);
    let mut warn = vec![];
    // each room: its number, the valve's place, its height, the attic
    // run's height, its need, its level
    struct Point {
        number: String,
        at: DVec2,
        h: f64,
        z_run: f64,
        need: f64,
        level: ElementId,
        elev: f64,
    }
    let mut pts: Vec<Point> = vec![];
    for id in rooms {
        let Some(e) = model.get(*id) else { continue };
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let Some(shape) = creabim_core::room::shape_of(model, r) else {
            warn.push(format!("{} {}: miestnosť nemá obrys.", r.number, e.name));
            continue;
        };
        let elev = model.level(r.level).map_or(0.0, |l| l.elevation);
        let at = middle(&shape.outline);
        let h = creabim_core::room::height(model, r);
        let ceiling_top = creabim_core::room::ceiling_over(model, r.level, at, elev + h + 0.01)
            .and_then(|(cid, z)| match model.get(cid).map(|c| &c.kind) {
                Some(ElementKind::Ceiling(c)) => Some(z + c.thickness - elev),
                _ => None,
            })
            .unwrap_or(h + 0.3);
        pts.push(Point {
            number: r.number.clone(),
            at,
            h,
            z_run: ceiling_top + 0.35,
            need: room_need(model, ho, *id),
            level: r.level,
            elev,
        });
    }
    let numbers: Vec<String> = pts.iter().map(|p| p.number.clone()).collect();
    let remove: Vec<ElementId> = model
        .iter()
        .filter(|e| of_design(&e.name, &numbers))
        .map(|e| e.id)
        .collect();
    let Some(first) = pts.first() else {
        return (vec![], remove, warn);
    };
    if pts.iter().any(|p| p.level != first.level) {
        warn.push("Spoločná vetva: miestnosti musia byť na jednom podlaží.".to_string());
        return (vec![], remove, warn);
    }
    let (level, elev) = (first.level, first.elev);
    let z_run = pts.iter().map(|p| p.z_run).fold(f64::MIN, f64::max);
    // the collecting duct along the way the rooms spread most, at their
    // middle across
    let (lo, hi) = pts.iter().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(p.at), hi.max(p.at)),
    );
    let along = if hi.x - lo.x >= hi.y - lo.y {
        DVec2::X
    } else {
        DVec2::Y
    };
    let across = along.perp();
    let mid = pts.iter().map(|p| p.at).sum::<DVec2>() / pts.len() as f64;
    let c = mid.dot(across);
    let foot = |p: DVec2| along * p.dot(along) + across * c;
    // the fan's end: the end of the collecting duct where the roof is
    // higher
    let s_lo = pts.iter().map(|p| p.at.dot(along)).fold(f64::MAX, f64::min);
    let s_hi = pts.iter().map(|p| p.at.dot(along)).fold(f64::MIN, f64::max);
    let run = 1.5;
    let end_at = |s: f64, out: f64| along * (s + out * run) + across * c;
    let roof_at = |q: DVec2| roof_top(model, q).unwrap_or(f64::MIN);
    let out = if roof_at(end_at(s_hi, 1.0)) >= roof_at(end_at(s_lo, -1.0)) {
        1.0
    } else {
        -1.0
    };
    let s_fan = if out > 0.0 { s_hi } else { s_lo };
    let m = end_at(s_fan, out);
    let Some(top) = roof_top(model, m).map(|z| z - elev) else {
        warn.push("Nad koncom zberného potrubia nie je strecha.".to_string());
        return (vec![], remove, warn);
    };
    let tag = format!("{DESIGN_TAG} {}", numbers.join("+"));
    let mut add = vec![];
    let duct = |add: &mut Vec<Element>, system: &str, d: f64, from: DVec3, to: DVec3| {
        let p = Pipe {
            level,
            system: system.into(),
            start: from,
            end: to,
            pipe_type: "VZK".into(),
            nominal_mm: d,
            outer_mm: d + 1.0,
            inner_mm: d,
            height_mm: None,
            // through the cold attic: insulated against condensation
            insulation_mm: 30.0,
            insulation_set: true,
            mark: None,
        };
        let el = Element::new(format!("{tag} – potrubie Ø{d:.0}"), ElementKind::Pipe(p));
        let id = el.id;
        add.push(el);
        id
    };
    let at3 = |q: DVec2, z: f64| DVec3::new(q.x, q.y, z);
    // the branches: up, then square across to the collecting duct
    for p in &pts {
        let up = duct(&mut add, "VZO", 100.0, at3(p.at, p.h), at3(p.at, z_run));
        let f = foot(p.at);
        if f.distance(p.at) > JOIN {
            duct(&mut add, "VZO", 100.0, at3(p.at, z_run), at3(f, z_run));
        }
        let mut v = Valve::new(up, 0.0, "vyustO");
        v.product = Some(VALVES[0].id.into());
        add.push(Element::new(
            format!("{tag} – odvodný ventil {}", p.number),
            ElementKind::Valve(v),
        ));
    }
    // the collecting duct from the far end to the fan's end, each piece
    // as wide as the air it carries
    let mut stops: Vec<f64> = pts.iter().map(|p| p.at.dot(along)).collect();
    stops.sort_by(|a, b| (a * out).total_cmp(&(b * out)));
    stops.dedup_by(|a, b| (*a - *b).abs() < JOIN);
    let need_upto = |s: f64| -> f64 {
        pts.iter()
            .filter(|p| p.at.dot(along) * out <= s * out + JOIN)
            .map(|p| p.need)
            .sum()
    };
    let main_v = ductsizing::Limits::default().main;
    for w in stops.windows(2) {
        let q = need_upto(w[0]);
        let d = ductsizing::round_for(q, main_v).max(100.0);
        duct(
            &mut add,
            "VZO",
            d,
            at3(along * w[0] + across * c, z_run),
            at3(along * w[1] + across * c, z_run),
        );
    }
    let total: f64 = pts.iter().map(|p| p.need).sum();
    let d_fan = ductsizing::round_for(total, main_v).max(fan.dn);
    let to_fan = duct(
        &mut add,
        "VZO",
        d_fan,
        at3(along * s_fan + across * c, z_run),
        at3(m, z_run),
    );
    let outlet = duct(
        &mut add,
        "VZE",
        d_fan,
        at3(m, z_run),
        at3(m, top.max(z_run + 0.5)),
    );
    let mut fv = Valve::new(to_fan, 0.5, "ventilator");
    fv.product = Some(fan.id.into());
    fv.preset = speed.filter(|s| !s.is_empty()).map(str::to_string);
    add.push(Element::new(
        format!("{tag} – ventilátor {}", fan.name),
        ElementKind::Valve(fv),
    ));
    let mut hood = Valve::new(outlet, 1.0, "vyfukVzt");
    hood.product = OUTLETS
        .iter()
        .find(|o| (o.dn - d_fan).abs() < 1.0)
        .map(|o| o.id.to_string());
    if hood.product.is_none() {
        warn.push(format!(
            "Strešný výfuk Ø{d_fan:.0}: v aplikácii nie je výrobok tejto veľkosti, doplňte ho."
        ));
    }
    add.push(Element::new(
        format!("{tag} – strešná hlavica"),
        ElementKind::Valve(hood),
    ));
    if fan.in_room {
        warn.push(format!(
            "{} sa osádza do stropu miestnosti, na spoločnú vetvu sa nehodí.",
            fan.name
        ));
    }
    (add, remove, warn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Element, Room, Transaction};

    fn duct(level: ElementId, system: &str, a: DVec3, b: DVec3) -> Pipe {
        Pipe {
            level,
            system: system.into(),
            start: a,
            end: b,
            pipe_type: "VZK".into(),
            nominal_mm: 100.0,
            outer_mm: 100.0,
            inner_mm: 99.0,
            height_mm: None,
            insulation_mm: 0.0,
            insulation_set: false,
            mark: None,
        }
    }

    #[test]
    fn curves_fall_as_the_flow_rises() {
        for f in FANS {
            for s in f.speeds {
                assert!(s.curve.windows(2).all(|w| w[1].0 > w[0].0), "{}", f.name);
                assert_eq!(s.curve.last().unwrap().1, 0.0);
                assert!(
                    (s.curve.last().unwrap().0 - s.q_free).abs() <= 2.0,
                    "{}",
                    f.name
                );
            }
        }
        let td = fan("SP-TD160-100-SILENT").unwrap();
        assert_eq!(td.speed(Some("NO")).q_free, 150.0);
        assert!((td.speeds[0].pressure(100.0) - 54.45).abs() < 0.01);
    }

    #[test]
    fn a_bathroom_fan_runs_where_its_curve_meets_the_ducts() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        doc.commit(Transaction::new("room").insert(Element::new(
            "Kúpeľňa",
            ElementKind::Room(Room::new(level, DVec2::new(2.5, 3.0))),
        )))
        .unwrap();
        // valve in the ceiling, 3 m across the attic, up 2 m to the roof
        let a = DVec3::new(2.5, 3.0, 2.6);
        let b = DVec3::new(2.5, 3.0, 3.0);
        let c = DVec3::new(5.5, 3.0, 3.0);
        let d = DVec3::new(5.5, 3.0, 5.0);
        let mut ids = vec![];
        for (x, y) in [(a, b), (b, c), (c, d)] {
            let e = Element::new("Duct", ElementKind::Pipe(duct(level, "VZO", x, y)));
            ids.push(e.id);
            doc.commit(Transaction::new("duct").insert(e)).unwrap();
        }
        let valve = Element::new(
            "Valve",
            ElementKind::Valve(Valve::new(ids[0], 0.0, "vyustO")),
        );
        let fanv = Element::new(
            "Fan",
            ElementKind::Valve(Valve::new(ids[1], 0.5, "ventilator")),
        );
        let hood = Element::new(
            "Hood",
            ElementKind::Valve(Valve::new(ids[2], 1.0, "vyfukVzt")),
        );
        doc.commit(
            Transaction::new("v")
                .insert(valve)
                .insert(fanv)
                .insert(hood),
        )
        .unwrap();
        let ho = heatloss::Opts::default();
        let rows = check(doc.model(), &ho, &Opts::default());
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.fan.name, "TD-160/100 N SILENT");
        assert!(r.outlet && r.inlets.len() == 1);
        assert_eq!(r.inlets[0].bends, 2);
        assert!((r.inlets[0].length - 5.4).abs() < 1e-6);
        assert_eq!(r.need, 90.0);
        // the fan gives more than the ducts take at the need
        assert!(r.p_need > r.dp_need && r.ok(), "{r:?}");
        assert!(r.q > 90.0 && r.q < 180.0);
        assert!((r.p - r.fan.speeds[0].pressure(r.q)).abs() < 1e-9);

        // a ceiling fan on the same ducts does not reach 90 m³/h
        let before = doc.model().get(r.id).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::Valve(v) = &mut after.kind {
            v.product = Some("SP-SILENT100-CHZ".into());
        }
        doc.commit(Transaction::new("fan").replace(before, after))
            .unwrap();
        let m = doc.model();
        let rows = check(m, &ho, &Opts::default());
        assert!(!rows[0].ok() && rows[0].q < 90.0, "{:?}", rows[0]);
    }

    #[test]
    fn a_new_design_replaces_the_old_one_of_its_rooms() {
        let n = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(of_design("Odvetranie 1.08 – potrubie Ø100", &n(&["1.08"])));
        assert!(of_design(
            "Odvetranie 1.07+1.08+1.12 – ventilátor TD",
            &n(&["1.12"])
        ));
        assert!(!of_design("Odvetranie 1.08 – potrubie Ø100", &n(&["1.07"])));
        assert!(!of_design("Rekuperácia – VZO 1.08", &n(&["1.08"])));
    }
}
