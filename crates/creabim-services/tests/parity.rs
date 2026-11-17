//! The water calculation against PilonHydraulika's: each network in
//! `tests/parity` is run here and compared with what the C# core found
//! (`<name>.expected.json`, written by `tools/pilon-parity`).

use creabim_services::fixtures::match_fixture;
use creabim_services::network::{AccessoryRole, FittingKind};
use creabim_services::{Network, Node, NodeKind, PipeSize, Settings, SystemKind};
use serde_json::Value;
use std::path::Path;

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

fn enum_of<T: serde::de::DeserializeOwned + Default>(v: &Value) -> T {
    if v.is_null() {
        T::default()
    } else {
        serde_json::from_value(v.clone()).unwrap()
    }
}

fn size(v: &Value) -> Option<PipeSize> {
    v.as_array().map(|a| PipeSize {
        nominal_mm: f(&a[0]),
        inner_mm: f(&a[1]),
    })
}

fn point(v: &Value) -> [f64; 3] {
    [f(&v[0]), f(&v[1]), f(&v[2])]
}

fn network(case: &Value, s: &Settings) -> Network {
    let mut net = Network::default();
    for j in case["nodes"].as_array().unwrap() {
        let kind: NodeKind = enum_of(&j["kind"]);
        let p1 = point(&j["p1"]);
        let p2 = point(&j["p2"]);
        let mut n = if kind == NodeKind::Pipe {
            let mut n = Node::pipe(
                j["id"].as_str().unwrap(),
                p1,
                p2,
                PipeSize {
                    nominal_mm: f(&j["nominal"]),
                    inner_mm: f(&j["inner"]),
                },
                j["sizes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(size)
                    .collect(),
            );
            n.roughness_m = f(&j["roughness"]);
            n.material = j["material"].as_str().unwrap().into();
            n.manual_size = size(&j["manual"]);
            n
        } else {
            Node {
                id: j["id"].as_str().unwrap().into(),
                kind,
                p1,
                p2,
                elbow_angle_deg: 90.0,
                ..Default::default()
            }
        };
        n.fitting = enum_of::<FittingKind>(&j["fitting"]);
        n.role = enum_of::<AccessoryRole>(&j["role"]);
        n.is_branch = j["branch"].as_bool().unwrap_or(false);
        n.is_linked_equipment = j["linked"].as_bool().unwrap_or(false);
        n.leaf_required_pa = f(&j["leaf_required_pa"]);
        n.zeta_override = j["zeta"].as_f64();
        if let Some(a) = j["angle"].as_f64() {
            n.elbow_angle_deg = a;
        }
        if let Some(name) = j["fixture"].as_str() {
            n.name = name.into();
            let mut fx = match_fixture(&s.fixtures, &s.unknown_fixture, name, "");
            fx.connector_nominal_mm = f(&j["connector_dn"]);
            n.fixture = Some(fx);
        }
        let parent = j["parent"].as_u64().map(|p| p as usize);
        net.add(parent, n);
    }
    net
}

/// Equal to a relative 1e-9, or within 1e-9 near zero.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn compare(name: &str, case: &Value, want: &Value) -> Vec<String> {
    let s: Settings = serde_json::from_value(case["settings"].clone()).unwrap();
    let mut net = network(case, &s);
    let kind = if case["kind"] == "hot" {
        SystemKind::HotWater
    } else {
        SystemKind::ColdWater
    };
    let r = creabim_services::water(
        &s,
        &mut net,
        kind,
        case["root_is_connection"].as_bool().unwrap(),
        case["available_kpa"].as_f64(),
    );
    let mut bad = vec![];
    let num = |bad: &mut Vec<String>, what: String, got: f64, want: &Value| {
        if !near(got, f(want)) {
            bad.push(format!("{name}: {what} {got} ≠ {}", f(want)));
        }
    };
    num(
        &mut bad,
        "root flow".into(),
        r.root_flow_ls,
        &want["root_flow_ls"],
    );
    num(
        &mut bad,
        "required".into(),
        r.required_pa,
        &want["required_pa"],
    );
    num(
        &mut bad,
        "required as is".into(),
        r.required_pa_current,
        &want["required_pa_current"],
    );
    num(
        &mut bad,
        "available".into(),
        r.available_pa,
        &want["available_pa"],
    );
    for (n, w) in net.nodes.iter().zip(want["nodes"].as_array().unwrap()) {
        let id = &n.id;
        num(&mut bad, format!("{id} flow"), n.flow_ls, &w["flow_ls"]);
        num(&mut bad, format!("{id} ΣLU"), n.sum_lu, &w["sum_lu"]);
        num(
            &mut bad,
            format!("{id} least DN"),
            n.min_nominal_mm,
            &w["min_nominal_mm"],
        );
        num(
            &mut bad,
            format!("{id} v"),
            n.velocity_ms,
            &w["velocity_ms"],
        );
        num(&mut bad, format!("{id} R"), n.r_pa_per_m, &w["r_pa_per_m"]);
        num(
            &mut bad,
            format!("{id} linear loss"),
            n.linear_loss_pa,
            &w["linear_loss_pa"],
        );
        num(
            &mut bad,
            format!("{id} local loss"),
            n.local_loss_pa,
            &w["local_loss_pa"],
        );
        num(
            &mut bad,
            format!("{id} fixed loss"),
            n.fixed_loss_pa,
            &w["fixed_loss_pa"],
        );
        num(&mut bad, format!("{id} ζ"), n.zeta, &w["zeta"]);
        if n.proposed != size(&w["proposed"]) {
            bad.push(format!(
                "{name}: {id} size {:?} ≠ {}",
                n.proposed, w["proposed"]
            ));
        }
        if n.on_critical_path != w["on_critical_path"].as_bool().unwrap() {
            bad.push(format!(
                "{name}: {id} on the critical path {}",
                n.on_critical_path
            ));
        }
    }
    let leaf = r.critical_leaf.map(|l| net.nodes[l].id.clone());
    if leaf.as_deref() != want["critical_leaf"].as_str() {
        bad.push(format!(
            "{name}: critical leaf {leaf:?} ≠ {}",
            want["critical_leaf"]
        ));
    }
    bad
}

#[test]
fn water_matches_pilon_hydraulika() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/parity");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter(|p| !p.to_string_lossy().ends_with(".expected.json"))
        .collect();
    files.sort();
    assert!(!files.is_empty());
    let mut bad = vec![];
    for p in &files {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let case: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let want: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join(format!("{name}.expected.json"))).unwrap(),
        )
        .unwrap();
        bad.extend(compare(&name, &case, &want));
    }
    let cases: std::collections::BTreeSet<_> =
        bad.iter().map(|b| b.split(':').next().unwrap()).collect();
    assert!(
        bad.is_empty(),
        "{} of {} networks differ:\n{}",
        cases.len(),
        files.len(),
        bad.iter().take(60).cloned().collect::<Vec<_>>().join("\n")
    );
}
