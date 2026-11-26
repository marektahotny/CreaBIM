//! The Statika tab's timber roof and steel members (George 2026-11-26):
//! the bar's section, grade, slope and turnbuckle for the member the
//! Beam or Column tool draws, and setting a new beam or column up as it.

use super::CreaBimApp;
use crate::memberstool::MemberSettings;
use crate::ui_model::Item;
use creabim_core::framing::{self, Framing, Role};
use creabim_core::{Beam, Column};
use glam::DVec3;
use serde_json::json;

/// Timber strength classes (STN EN 338, STN EN 14080) and steel grades
/// (STN EN 10025-2) to pick by name.
pub const TIMBER_GRADES: [&str; 8] = ["C16", "C20", "C22", "C24", "C27", "C30", "GL24h", "GL28h"];
pub const STEEL_GRADES: [&str; 3] = ["S235", "S275", "S355"];

/// The bar while a member is drawn.
pub fn tool_options(m: &MemberSettings) -> Vec<serde_json::Value> {
    let Some(f) = &m.framing else {
        return vec![];
    };
    let mut v = vec![
        json!({
            "kind": "menu", "key": "framing.role", "label": "Member",
            "value": f.role.label(), "icon": f.role.icon(),
            "hint": "What is drawn: the roof's timber (rafter, purlin, wall plate, post…) or a steel column, beam or bracing rod",
        }),
        json!({
            "kind": "menu", "key": "framing.section", "label": "Cross-section",
            "value": f.section, "icon": "profile",
            "hint": "Its section from the makers' tables: KVH (DEKWOOD), IPE, HEA, HEB, UPN (STAD), hollow sections and angles (Ferona), round bars; or a timber b/h typed below",
        }),
    ];
    if f.role.is_timber() {
        let s = framing::section(&f.section);
        let (b, h) = s.map_or((0.1, 0.16), |s| (s.b, s.h));
        v.push(json!({
            "kind": "number", "key": "member.framing_b", "label": "b", "value": b,
            "unit": "m", "step": 0.01, "min": 0.02, "max": 0.5,
            "hint": "Width of the timber section as the structural drawing gives it",
        }));
        v.push(json!({
            "kind": "number", "key": "member.framing_h", "label": "h", "value": h,
            "unit": "m", "step": 0.01, "min": 0.02, "max": 1.0,
            "hint": "Height of the timber section as the structural drawing gives it",
        }));
    }
    v.push(json!({
        "kind": "menu", "key": "framing.grade", "label": "Grade",
        "value": f.grade,
        "hint": "Strength class (C24, GL24h) or steel grade (S235, S355): kept with the member for a later calculation",
    }));
    if !f.role.is_column() {
        v.push(json!({
            "kind": "number", "key": "member.framing_angle", "label": "Pitch", "value": m.framing_angle,
            "unit": "°", "step": 1.0, "min": -80.0, "max": 80.0,
            "hint": "How steeply it climbs from the first point clicked to the second: 0 level, a rafter at the roof's pitch",
        }));
    }
    if f.role == Role::BracingRod {
        v.push(json!({
            "kind": "toggle", "id": "member:framing.turnbuckle", "label": "Turnbuckle",
            "on": f.turnbuckle, "icon": "link",
            "hint": "A turnbuckle at the rod's middle: Rothoblaas TENDITORE for its thread (M12–M30)",
        }));
    }
    v
}

/// A number typed in the bar; false when it isn't a member's.
pub fn set_option(m: &mut MemberSettings, key: &str, v: f64) -> bool {
    match key {
        "framing_angle" => m.framing_angle = v.clamp(-80.0, 80.0),
        "framing_b" | "framing_h" => {
            let Some(f) = &mut m.framing else { return true };
            let s = framing::section(&f.section);
            let (mut b, mut h) = s.map_or((0.1, 0.16), |s| (s.b, s.h));
            if key == "framing_b" {
                b = v.clamp(0.02, 0.5);
            } else {
                h = v.clamp(0.02, 1.0);
            }
            f.section = format!("{:.0}/{:.0}", b * 1000.0, h * 1000.0);
            // a maker's size typed again takes its name back
            if let Some(k) = framing::timber_sections()
                .into_iter()
                .find(|k| (k.b - b).abs() < 5e-4 && (k.h - h).abs() < 5e-4)
            {
                f.section = k.name;
            }
        }
        _ => return false,
    }
    true
}

/// A toggle in the bar; false when it isn't a member's.
pub fn toggle_option(m: &mut MemberSettings, what: &str) -> bool {
    if what != "framing.turnbuckle" {
        return false;
    }
    if let Some(f) = &mut m.framing {
        f.turnbuckle = !f.turnbuckle;
    }
    true
}

/// The bar's menus.
pub fn menu_items(m: &MemberSettings, key: &str) -> Option<Vec<Item>> {
    let f = m.framing.as_ref();
    Some(match key {
        "framing.role" => Role::ALL
            .iter()
            .enumerate()
            .flat_map(|(i, r)| {
                let item = Item::new(format!("framing.role:{i}"), r.label())
                    .icon(r.icon())
                    .checked(f.map(|f| f.role) == Some(*r));
                if i == Role::TIMBER.len() {
                    vec![Item::Separator, item]
                } else {
                    vec![item]
                }
            })
            .collect(),
        "framing.section" => {
            let now = f.map(|f| f.section.clone()).unwrap_or_default();
            let item =
                |n: String| Item::new(format!("framing.section:{n}"), n.clone()).checked(n == now);
            if f.is_some_and(|f| f.role.is_timber()) {
                framing::timber_sections()
                    .into_iter()
                    .map(|s| item(s.name))
                    .collect()
            } else {
                let all = framing::steel_sections();
                let family = |p: &str| -> Vec<Item> {
                    all.iter()
                        .filter(|s| s.name.starts_with(p))
                        .map(|s| item(s.name.clone()))
                        .collect()
                };
                let rod = f.is_some_and(|f| f.role == Role::BracingRod);
                if rod {
                    family("Ø")
                } else {
                    vec![
                        Item::sub("IPE", family("IPE ")),
                        Item::sub("HEA", family("HEA ")),
                        Item::sub("HEB", family("HEB ")),
                        Item::sub("UPN", family("UPN ")),
                        Item::sub("SHS", family("SHS ")),
                        Item::sub("RHS", family("RHS ")),
                        Item::sub("L", family("L ")),
                        Item::sub("Round bars", family("Ø")),
                    ]
                }
            }
        }
        "framing.grade" => {
            let timber = f.is_none_or(|f| f.role.is_timber());
            let grades: &[&str] = if timber {
                &TIMBER_GRADES
            } else {
                &STEEL_GRADES
            };
            grades
                .iter()
                .map(|g| {
                    Item::new(format!("framing.grade:{g}"), *g)
                        .checked(f.is_some_and(|f| f.grade == *g))
                })
                .collect()
        }
        _ => return None,
    })
}

impl CreaBimApp {
    /// `framing.section:` and `framing.grade:` from the bar.
    pub(super) fn framing_action(&mut self, head: &str, arg: &str) -> bool {
        let Some(f) = &mut self.plan.members.framing else {
            return matches!(head, "framing.section" | "framing.grade");
        };
        match head {
            "framing.section" => {
                if framing::section(arg).is_some() {
                    f.section = arg.to_string();
                    if f.role == Role::BracingRod {
                        f.turnbuckle &= framing::section(arg)
                            .and_then(|s| framing::turnbuckle_for(s.b))
                            .is_some();
                    }
                }
            }
            "framing.grade" => f.grade = arg.to_string(),
            _ => return false,
        }
        true
    }
}

/// Make `b`, drawn from its start to its end, the member `f`: its
/// section, the slope it climbs at from its start, and a sloped one's
/// section square to its axis.
pub fn make_beam(b: &mut Beam, f: &Framing, angle: f64) {
    let mut e = creabim_core::Element::new("", creabim_core::ElementKind::Beam(b.clone()));
    framing::apply(&mut e, f.clone());
    if let creabim_core::ElementKind::Beam(x) = e.kind {
        *b = x;
    }
    if f.role.slopes() && angle.abs() > 1e-6 {
        let rise = framing::rise_for(b.start, b.end, angle);
        b.rise = [0.0, rise];
        b.up = up_of(b);
    }
}

/// The section's up for a sloped beam: square to its axis, in the upright
/// plane through it.
pub fn up_of(b: &Beam) -> Option<DVec3> {
    let d = (b.end - b.start)
        .extend(b.rise[1] - b.rise[0])
        .normalize_or_zero();
    let side = DVec3::Z.cross(d).normalize_or_zero();
    if side == DVec3::ZERO {
        return None;
    }
    let up = d.cross(side).normalize_or_zero();
    Some(if up.z < 0.0 { -up } else { up })
}

/// Make `c` the member `f`.
pub fn make_column(c: &mut Column, f: &Framing) {
    let mut e = creabim_core::Element::new("", creabim_core::ElementKind::Column(c.clone()));
    framing::apply(&mut e, f.clone());
    if let creabim_core::ElementKind::Column(x) = e.kind {
        *c = x;
    }
}

/// The name a new member takes.
/// The member's name in the app's language (Krokva, Oceľový stĺp…).
pub fn name_of(f: &Framing) -> String {
    crate::i18n::tr(f.role.label())
}

/// The Properties rows of a member: what it is, its section, grade,
/// slope, how its ends are held, its turnbuckle and mark, and the
/// section's properties a calculation takes. The undo label when
/// something changed.
pub fn properties(
    ui: &mut eframe::egui::Ui,
    f: &mut Framing,
    slope: Option<&mut f64>,
) -> &'static str {
    use creabim_core::framing::EndFix;
    let mut label = "";
    let tr = crate::i18n::tr;
    crate::props::Combo::new("Member")
        .selected_text(tr(f.role.label()))
        .show_ui(ui, |ui| {
            let roles: &[Role] = if f.role.is_timber() {
                &Role::TIMBER
            } else {
                &Role::STEEL
            };
            for r in roles {
                if ui.selectable_label(f.role == *r, tr(r.label())).clicked() && f.role != *r {
                    f.role = *r;
                    label = "Member";
                }
            }
        });
    crate::props::Combo::new("Cross-section")
        .selected_text(f.section.clone())
        .show_ui(ui, |ui| {
            for n in framing::section_names(f.role) {
                if ui.selectable_label(f.section == n, &n).clicked() && f.section != n {
                    f.section = n;
                    label = "Member section";
                }
            }
        });
    if f.role.is_timber() {
        // b/h as the structural drawing gives it
        let (mut b, mut h) = framing::section(&f.section).map_or((0.1, 0.16), |s| (s.b, s.h));
        let row = |ui: &mut eframe::egui::Ui, t: &str, v: &mut f64| {
            crate::props::row(ui, t, |ui| {
                crate::props::fill(ui, crate::units::drag(v).range(0.02..=1.0)).changed()
            })
        };
        if row(ui, "b", &mut b) | row(ui, "h", &mut h) {
            f.section = framing::timber_sections()
                .into_iter()
                .find(|k| (k.b - b).abs() < 5e-4 && (k.h - h).abs() < 5e-4)
                .map_or_else(
                    || format!("{:.0}/{:.0}", b * 1000.0, h * 1000.0),
                    |k| k.name,
                );
            label = "Member section";
        }
    }
    crate::props::Combo::new("Grade")
        .selected_text(f.grade.clone())
        .show_ui(ui, |ui| {
            let grades: &[&str] = if f.role.is_timber() {
                &TIMBER_GRADES
            } else {
                &STEEL_GRADES
            };
            for g in grades {
                if ui.selectable_label(f.grade == *g, *g).clicked() && f.grade != *g {
                    f.grade = g.to_string();
                    label = "Member grade";
                }
            }
        });
    if let Some(a) = slope {
        if crate::props::row(ui, "Pitch", |ui| {
            crate::props::fill(
                ui,
                eframe::egui::DragValue::new(a)
                    .speed(0.5)
                    .range(-80.0..=80.0)
                    .suffix("°"),
            )
            .changed()
        }) {
            label = "Member slope";
        }
    }
    let ends = [
        (EndFix::Pinned, "Pinned", "A hinge: held, free to turn"),
        (EndFix::Fixed, "Fixed", "Held against turning too"),
        (EndFix::Free, "Free", "A cantilever's free end"),
    ];
    if crate::props::choice(ui, "Start support", &mut f.ends[0], &ends)
        | crate::props::choice(ui, "End support", &mut f.ends[1], &ends)
    {
        label = "Member supports";
    }
    if f.role == Role::BracingRod
        && crate::props::check(
            ui,
            "Turnbuckle",
            &mut f.turnbuckle,
            tr("Rothoblaas TENDITORE at its middle"),
        )
        .changed()
    {
        label = "Turnbuckle";
    }
    let mut mark = f.mark.clone().unwrap_or_default();
    if crate::props::row(ui, "Position mark", |ui| {
        crate::props::fill(
            ui,
            eframe::egui::TextEdit::singleline(&mut mark).hint_text(tr("numbered")),
        )
        .changed()
    }) {
        f.mark = (!mark.trim().is_empty()).then(|| mark.trim().to_string());
        label = "Member mark";
    }
    if let Some(s) = framing::section(&f.section) {
        let cm = |v: f64, k: f64| format!("{:.2}", v * k).replace('.', ",");
        let mut t = format!(
            "A = {} cm², Iy = {} cm⁴, Iz = {} cm⁴, Wy = {} cm³, Wz = {} cm³",
            cm(s.area, 1e4),
            cm(s.iy, 1e8),
            cm(s.iz, 1e8),
            cm(s.wy, 1e6),
            cm(s.wz, 1e6)
        );
        if let Some(m) = s.mass {
            t += &format!(", {} kg/m", cm(m, 1.0));
        }
        t += &format!(" ({})", s.source);
        crate::props::note(ui, t);
    }
    label
}

#[cfg(test)]
mod tests {
    /// George's project with a timber roof and a steel carport drawn in,
    /// for a look in the app: `CREABIM_FRAMING_IN=<.creabim>
    /// CREABIM_FRAMING_OUT=<.creabim> cargo test -p creabim-app
    /// framing_shot -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn framing_shot() {
        use creabim_core::{ElementKind, ViewKind};
        let (Ok(i), Ok(o)) = (
            std::env::var("CREABIM_FRAMING_IN"),
            std::env::var("CREABIM_FRAMING_OUT"),
        ) else {
            return;
        };
        crate::i18n::set_lang(crate::i18n::Lang::Sk);
        let mut doc = creabim_store::load(&i).unwrap();
        let m = doc.model();
        for (e, l) in m.levels() {
            println!(
                "level {} {:.3} h {:.3} {:?}",
                e.name, l.elevation, l.height, e.id
            );
        }
        for e in m.iter() {
            if let ElementKind::View(v) = &e.kind {
                if let ViewKind::Plan { level, .. } = v.kind {
                    println!(
                        "plan {:?} on {:?} structural {}",
                        e.name, level, v.structural
                    );
                }
            }
            if let ElementKind::Roof(r) = &e.kind {
                println!(
                    "roof {:?} level {:?} pitch {} base {:.3} peak {:.3} bbox {:?}",
                    e.name,
                    r.level,
                    r.pitch,
                    r.base(m),
                    r.peak(),
                    r.eave_outline()
                );
            }
        }
        let mut lo = glam::DVec2::splat(1e9);
        let mut hi = glam::DVec2::splat(-1e9);
        for e in m.iter() {
            if let ElementKind::Wall(w) = &e.kind {
                if m.get(w.level).is_some_and(|l| l.name == "1. NP") {
                    for p in [w.start, w.end] {
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                    println!(
                        "wall t {:.3} top {:?}",
                        w.thickness,
                        crate::app::framingui::tests::wall_top(m, w)
                    );
                }
            }
            if let ElementKind::Roof(r) = &e.kind {
                println!(
                    "shape {:?} slopes {:?}",
                    r.effective_shape(),
                    (0..r.eave_outline().len())
                        .map(|i| r.slope(i))
                        .collect::<Vec<_>>()
                );
            }
        }
        println!("walls {lo:?} {hi:?}");
        let roof = m
            .levels()
            .find(|(e, _)| e.name == "Strecha")
            .map(|(e, _)| e.id)
            .unwrap();
        let ground = m
            .levels()
            .find(|(e, _)| e.name == "1. NP")
            .map(|(e, _)| e.id)
            .unwrap();
        let mut ids = build(doc.model(), roof, ground);
        // the statik's krov drawing: the roof level seen from over the
        // ridge, the roof covering left out
        let mut kv = creabim_core::View::plan(roof);
        kv.kind = ViewKind::Plan {
            level: roof,
            cut_height: 3.5,
        };
        kv.scale = 50.0;
        kv.structural = true;
        kv.hidden.set_category(creabim_core::Category::Roofs, true);
        ids.insert(
            0,
            creabim_core::Element::new("Výkres krovu", ElementKind::View(kv)),
        );
        // the timber list grouped by cross-section, for the budget
        let mut sv = creabim_core::View::plan(roof);
        sv.kind = ViewKind::Schedule {
            category: creabim_core::ScheduleCategory::TimberList,
            by_type: true,
        };
        ids.push(creabim_core::Element::new(
            "Výkaz reziva podľa prierezu",
            ElementKind::View(sv),
        ));
        let mut tx = creabim_core::Transaction::new("framing");
        let mut probe = doc.model().clone();
        let mut ok = vec![];
        for e in ids {
            ok.push(e.id);
            tx = tx.insert(e);
        }
        tx.clone().apply(&mut probe).unwrap();
        for t in crate::planview::framing_tags(&probe, &ok) {
            tx = tx.insert(t);
        }
        // the 3D view's crop box off, the roof framing seen whole
        for e in doc.model().iter() {
            if let ElementKind::CropBox(_) = &e.kind {
                let mut after = e.clone();
                if let ElementKind::CropBox(a) = &mut after.kind {
                    a.enabled = false;
                }
                tx = tx.replace(e.clone(), after);
            }
        }
        doc.commit(tx).unwrap();
        creabim_store::save(&o, &doc).unwrap();
        let t = creabim_geom::framing::timber_schedule(doc.model());
        for r in &t.rows {
            println!("{:?}", r.cells);
        }
        println!("{:?}", t.totals);
        let s = creabim_geom::framing::steel_schedule(doc.model());
        for r in &s.rows {
            println!("{:?}", r.cells);
        }
        println!("{:?}", s.totals);
    }

    /// A timber hip roof's members over George's house (walls 14,675 ×
    /// 11,850, tops at +2,800, pitch 25°) and a steel carport beside it.
    fn build(
        m: &creabim_core::Model,
        roof: creabim_core::ElementId,
        ground: creabim_core::ElementId,
    ) -> Vec<creabim_core::Element> {
        use creabim_core::framing::{self, Framing, Role};
        use creabim_core::members::{elevation, storey_height};
        use creabim_core::{Beam, Column, Element, ElementKind};
        use glam::{DVec2, DVec3};
        let mut out = vec![];
        let beam = |level, a: DVec3, b: DVec3, role: Role, sec: &str| {
            let top0 = elevation(m, level) + storey_height(m, level);
            let mut x = Beam::new(level, a.truncate(), b.truncate());
            x.top_offset = top0 - a.z;
            x.rise = [0.0, b.z - a.z];
            if (b.z - a.z).abs() > 1e-6 {
                x.up = super::up_of(&x);
            }
            let mut f = Framing::new(role);
            f.section = sec.into();
            let mut e = Element::new(crate::i18n::tr(role.label()), ElementKind::Beam(x));
            framing::apply(&mut e, f);
            e
        };
        let column = |level, p: DVec2, h: f64, base: f64, role: Role, sec: &str| {
            let mut c = Column::new(
                level,
                p,
                creabim_core::members::Profile::Rectangle {
                    width: 0.1,
                    depth: 0.1,
                },
            );
            c.height = Some(h);
            c.base_offset = base;
            let mut f = Framing::new(role);
            f.section = sec.into();
            let mut e = Element::new(crate::i18n::tr(role.label()), ElementKind::Column(c));
            framing::apply(&mut e, f);
            e
        };
        let v = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
        let t = 25f64.to_radians().tan();
        // wall plates on the walls' cores, tops at +2,950
        let (x0, y0, x1, y1) = (0.23, 0.23, 14.445, 11.62);
        let zp = 2.95;
        for (a, b) in [
            (v(x0, y0, zp), v(x1, y0, zp)),
            (v(x0, y1, zp), v(x1, y1, zp)),
            (v(x0, y0, zp), v(x0, y1, zp)),
            (v(x1, y0, zp), v(x1, y1, zp)),
        ] {
            out.push(beam(roof, a, b, Role::WallPlate, "150/150"));
        }
        // rafters 80/200 on the plates, the ridge over the middle
        let ym = (y0 + y1) / 2.0;
        let z0 = zp + 0.2 / 25f64.to_radians().cos();
        let half = ym - y0;
        let zr = z0 + half * t;
        let (rx0, rx1) = (x0 + half, x1 - half);
        let mut x = rx0;
        while x <= rx1 + 1e-6 {
            out.push(beam(
                roof,
                v(x, y0, z0),
                v(x, ym, zr),
                Role::Rafter,
                "80/200",
            ));
            out.push(beam(
                roof,
                v(x, y1, z0),
                v(x, ym, zr),
                Role::Rafter,
                "80/200",
            ));
            x += 0.9;
        }
        // jack rafters up to the hips, both ends
        let mut d = 0.9;
        while d < half - 0.3 {
            for (xa, s) in [(x0 + d, 1.0), (x1 - d, -1.0)] {
                let _ = s;
                out.push(beam(
                    roof,
                    v(xa, y0, z0),
                    v(xa, y0 + d, z0 + d * t),
                    Role::Rafter,
                    "80/200",
                ));
                out.push(beam(
                    roof,
                    v(xa, y1, z0),
                    v(xa, y1 - d, z0 + d * t),
                    Role::Rafter,
                    "80/200",
                ));
            }
            for (ya, _) in [(y0 + d, 1.0), (y1 - d, -1.0)] {
                out.push(beam(
                    roof,
                    v(x0, ya, z0),
                    v(
                        x0 + d.min(ya - y0).min(y1 - ya),
                        ya,
                        z0 + d.min(ya - y0).min(y1 - ya) * t,
                    ),
                    Role::Rafter,
                    "80/200",
                ));
                out.push(beam(
                    roof,
                    v(x1, ya, z0),
                    v(
                        x1 - d.min(ya - y0).min(y1 - ya),
                        ya,
                        z0 + d.min(ya - y0).min(y1 - ya) * t,
                    ),
                    Role::Rafter,
                    "80/200",
                ));
            }
            d += 0.9;
        }
        // hip rafters from the corners to the ridge's ends, 45° in plan
        let th = 25f64.to_radians().tan() / 2f64.sqrt();
        let hz = |a: DVec2, b: DVec2| z0 + a.distance(b) * th;
        for (c, r) in [
            (DVec2::new(x0, y0), DVec2::new(rx0, ym)),
            (DVec2::new(x0, y1), DVec2::new(rx0, ym)),
            (DVec2::new(x1, y0), DVec2::new(rx1, ym)),
            (DVec2::new(x1, y1), DVec2::new(rx1, ym)),
        ] {
            out.push(beam(
                roof,
                c.extend(z0),
                r.extend(hz(c, r)),
                Role::HipRafter,
                "120/200",
            ));
        }
        // ridge purlin 150/250 under the rafters' tops
        out.push(beam(
            roof,
            v(rx0, ym, zr - 0.22),
            v(rx1, ym, zr - 0.22),
            Role::RidgePurlin,
            "150/250",
        ));
        // middle purlins 160/240 on posts 150/150, the rafters on them
        for yp in [y0 + 2.6, y1 - 2.6] {
            let zt = z0 + 2.6 * t - 0.2 / 25f64.to_radians().cos();
            out.push(beam(
                roof,
                v(x0 + 2.6, yp, zt),
                v(x1 - 2.6, yp, zt),
                Role::Purlin,
                "160/240",
            ));
            for xp in [x0 + 2.6, 7.34, x1 - 2.6] {
                let base = 0.0;
                out.push(column(
                    roof,
                    DVec2::new(xp, yp),
                    zt - 0.24 - 2.8 - base,
                    base,
                    Role::Post,
                    "150/150",
                ));
            }
        }
        // collar ties 50/200 between the purlins at each post
        for xp in [x0 + 2.6, 7.34, x1 - 2.6] {
            let zc = z0 + 2.6 * t + 0.1;
            out.push(beam(
                roof,
                v(xp + 0.075, y0 + 2.3, zc),
                v(xp + 0.075, y1 - 2.3, zc),
                Role::Collar,
                "50/200",
            ));
            out.push(beam(
                roof,
                v(xp - 0.075, y0 + 2.3, zc),
                v(xp - 0.075, y1 - 2.3, zc),
                Role::Collar,
                "50/200",
            ));
        }
        // a steel carport beside the house: RHS columns, sloped beams at
        // 4°, SHS purlins, crossed rods Ø 12 with turnbuckles
        let (cx0, cx1, cy0, cy1) = (17.0, 22.0, 0.0, 5.0);
        let (h0, s) = (2.6, 4f64.to_radians().tan());
        for xc in [cx0, (cx0 + cx1) / 2.0, cx1] {
            for (yc, h) in [(cy0, h0), (cy1, h0 - (cy1 - cy0) * s)] {
                out.push(column(
                    ground,
                    DVec2::new(xc, yc),
                    h - 0.14,
                    0.0,
                    Role::SteelColumn,
                    "RHS 80×50×3",
                ));
            }
            out.push(beam(
                ground,
                v(xc, cy0 - 0.3, h0 + 0.3 * s),
                v(xc, cy1 + 0.3, h0 - (cy1 - cy0 + 0.3) * s),
                Role::SteelBeam,
                "RHS 140×80×3",
            ));
        }
        let mut y = cy0;
        while y <= cy1 + 1e-6 {
            let z = h0 - (y - cy0) * s + 0.05;
            out.push(beam(
                ground,
                v(cx0 - 0.2, y, z),
                v(cx1 + 0.2, y, z),
                Role::SteelBeam,
                "SHS 50×50×3",
            ));
            y += 1.25;
        }
        for (a, b) in [(cx0, (cx0 + cx1) / 2.0), ((cx0 + cx1) / 2.0, cx1)] {
            for (p, q) in [(a, b), (b, a)] {
                out.push(beam(
                    ground,
                    v(p, cy1, 0.15),
                    v(q, cy1, h0 - (cy1 - cy0) * s - 0.2),
                    Role::BracingRod,
                    "Ø 12",
                ));
            }
        }
        out
    }

    pub fn wall_top(m: &creabim_core::Model, w: &creabim_core::Wall) -> f64 {
        m.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset + w.height
    }
}
