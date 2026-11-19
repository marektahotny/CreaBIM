//! Performance budgets (Addendum A6-7). Timing on shared machines is
//! noisy, so these are opt-in, and only mean something in an optimised
//! build:
//!
//! ```text
//! cargo test --release -p creabim-geom --test perf -- --ignored --nocapture
//! ```
//!
//! Each budget is the time the starter house took on the owner's machine
//! (16 cores) when it was set, with room to spare; a test fails when a
//! change makes a drawing clearly slower. The targets A6 aims for are in
//! `docs/PERFORMANCE.md`.

use creabim_core::{ElementKind, Model, ViewKind};
use std::time::Instant;

/// The best of a few runs, in ms: the least disturbed by other work.
fn best_ms(mut f: impl FnMut()) -> f64 {
    f(); // warm up (the shared scene, allocations)
    (0..3)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::INFINITY, f64::min)
}

fn views(m: &Model, kind: fn(&ViewKind) -> bool) -> Vec<(String, creabim_core::ElementId)> {
    m.iter()
        .filter_map(|e| match &e.kind {
            ElementKind::View(v) if kind(&v.kind) && v.callout.is_none() => {
                Some((e.name.clone(), e.id))
            }
            _ => None,
        })
        .collect()
}

fn check(what: &str, ms: f64, budget: f64, failures: &mut Vec<String>) {
    let flag = if ms > budget { "  OVER" } else { "" };
    println!("{what:<32} {ms:>7.1} ms  (budget {budget:.0}){flag}");
    if ms > budget {
        failures.push(format!("{what}: {ms:.1} ms > {budget:.0} ms"));
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored"]
fn starter_house_within_budgets() {
    let m = creabim_core::starter::starter_house();
    let mut failures = vec![];

    // the 3D scene is cached per revision, so time a fresh build
    let ms = best_ms(|| {
        let _ = creabim_geom::build_scene(&m);
    });
    check("3D scene", ms, 70.0, &mut failures);

    for (name, id) in views(&m, |k| matches!(k, ViewKind::Plan { .. })) {
        let ms = best_ms(|| {
            let _ = creabim_geom::section::view_drawing_with(&m, id, false);
        });
        check(&name, ms, 25.0, &mut failures);
    }
    for (name, id) in views(&m, |k| matches!(k, ViewKind::Section { .. })) {
        let ms = best_ms(|| {
            let _ = creabim_geom::section::view_drawing_with(&m, id, false);
        });
        check(&name, ms, 160.0, &mut failures);
    }
    for (name, id) in views(&m, |k| matches!(k, ViewKind::Axonometric { .. })) {
        let ms = best_ms(|| {
            let _ = creabim_geom::section::view_drawing_with(&m, id, false);
        });
        check(&name, ms, 300.0, &mut failures);
    }
    assert!(failures.is_empty(), "over budget:\n{}", failures.join("\n"));
}

/// A 13-storey block: what a real project asks of the drawings. Prints
/// the times (no budgets yet: they are set from these runs).
#[test]
#[ignore = "timing: run with --release -- --ignored"]
fn tower_timings() {
    let storeys: usize = std::env::var("CREABIM_STOREYS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(13);
    let t = Instant::now();
    let m = creabim_core::starter::starter_tower(storeys);
    println!(
        "{storeys} storeys: {} elements, built in {:.0} ms",
        m.len(),
        t.elapsed().as_secs_f64() * 1000.0
    );
    let mut failures = vec![];
    let ms = best_ms(|| {
        let _ = creabim_geom::build_scene(&m);
    });
    check("3D scene", ms, f64::INFINITY, &mut failures);
    let scene = creabim_geom::build_scene(&m);
    let tris: usize = scene.iter().map(|i| i.mesh.indices.len() / 3).sum();
    println!("{tris} triangles");
    for (kind, test) in [
        (
            "plan",
            (|k: &ViewKind| matches!(k, ViewKind::Plan { .. })) as fn(&ViewKind) -> bool,
        ),
        ("section", |k| matches!(k, ViewKind::Section { .. })),
        ("3D", |k| matches!(k, ViewKind::Axonometric { .. })),
    ] {
        for (name, id) in views(&m, test) {
            let ms = best_ms(|| {
                let _ = creabim_geom::section::view_drawing_with(&m, id, false);
            });
            check(&format!("{kind}: {name}"), ms, f64::INFINITY, &mut failures);
        }
    }
}

#[test]
#[ignore = "diagnosis"]
fn tower_slow_elements() {
    let m = creabim_core::starter::starter_tower(3);
    let mut kinds: std::collections::BTreeMap<
        String,
        std::collections::HashSet<creabim_core::ElementId>,
    > = Default::default();
    for e in m.iter() {
        let k = format!("{:?}", e.kind)
            .split('(')
            .next()
            .unwrap()
            .to_string();
        kinds.entry(k).or_default().insert(e.id);
    }
    let name = |id: &creabim_core::ElementId| m.get(*id).unwrap().name.clone();
    use std::io::Write;
    for sl in &kinds["Slab"] {
        let mut set = kinds["Wall"].clone();
        set.insert(*sl);
        eprint!("all walls + {} {:?}... ", name(sl), sl);
        std::io::stderr().flush().unwrap();
        let t = Instant::now();
        let _ = creabim_geom::scene::build_scene_of(&m, Some(&set));
        eprintln!("{:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    }
    let geo = [
        "Wall", "Slab", "Opening", "Object", "Roof", "Column", "Beam", "Stair", "Footing",
        "Ceiling", "Shaft", "Railing",
    ];
    for (i, a) in geo.iter().enumerate() {
        for b in &geo[i + 1..] {
            let set: std::collections::HashSet<_> = kinds[*a].union(&kinds[*b]).copied().collect();
            let t = Instant::now();
            let _ = creabim_geom::scene::build_scene_of(&m, Some(&set));
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            if ms > 50.0 {
                println!("{a}+{b}: {ms:.0} ms");
            }
        }
    }
    for (k, set) in &kinds {
        let t = Instant::now();
        let s = creabim_geom::scene::build_scene_of(&m, Some(set));
        let tris: usize = s.iter().map(|i| i.mesh.indices.len() / 3).sum();
        println!(
            "{k:<16} {:>4} el {:>8.0} ms {tris} tris",
            set.len(),
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
}
