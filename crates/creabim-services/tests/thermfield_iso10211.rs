//! The 2D temperature field solver against the reference cases 1 and 2 of
//! STN EN ISO 10211, annex C (a method passing both is "very accurate").
//!
//! The norm's values live only in the user's private parameter file
//! `stn-en-iso-10211-overovanie*.tsv` (the newest one is taken) in the
//! directory named by `CREABIM_NORMS`; without it the tests say so and
//! pass. Tolerances 0,1 °C and 0,1 W/m are the annex's own.

use creabim_core::fireparams::{self, number, ParamTable};
use creabim_services::thermfield::{Detail, Edge, Env, Material, Mesh, Region};

fn tables() -> Option<Vec<ParamTable>> {
    let Ok(dir) = std::env::var("CREABIM_NORMS") else {
        eprintln!("CREABIM_NORMS not set: ISO 10211 annex C cases skipped");
        return None;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("stn-en-iso-10211-overovanie")
        })
        .collect();
    files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    let file = files.last()?;
    Some(fireparams::parse(
        &std::fs::read_to_string(file.path()).ok()?,
    ))
}

fn table<'a>(t: &'a [ParamTable], id: &str) -> &'a ParamTable {
    t.iter()
        .find(|x| x.id == id)
        .unwrap_or_else(|| panic!("table {id} missing"))
}

fn val(t: &ParamTable, key: &str, col: &str) -> f64 {
    number(t.cell(key, col)).unwrap_or_else(|| panic!("{} {key}", t.id))
}

#[test]
fn case_1_half_column() {
    let Some(t) = tables() else { return };
    let ref1 = table(&t, "pripad1");
    let a = 1.0;
    let d = Detail {
        materials: vec![Material {
            name: "m".into(),
            lambda: 1.0,
        }],
        regions: vec![Region::rect(0, 0.0, 0.0, a / 2.0, a)],
        envs: vec![
            Env {
                name: "AB".into(),
                theta: 20.0,
            },
            Env {
                name: "BC, CD".into(),
                theta: 0.0,
            },
        ],
        edges: vec![
            Edge {
                a: [0.0, 0.0],
                b: [a / 2.0, 0.0],
                env: 0,
                rs: 0.0,
            },
            Edge {
                a: [0.0, 0.0],
                b: [0.0, a],
                env: 1,
                rs: 0.0,
            },
            Edge {
                a: [0.0, a],
                b: [a / 2.0, a],
                env: 1,
                rs: 0.0,
            },
        ],
    };
    let f = d.solve(&Mesh {
        h_max: a / 64.0,
        h_min: a / 64.0,
        ratio: 1.0,
    });
    let mut n = 0;
    for row in &ref1.rows {
        let y = number(&row[0]).unwrap() * a / 8.0;
        for (c, cell) in row.iter().enumerate().skip(1) {
            let want = number(cell).unwrap();
            let got = f.at([c as f64 * a / 8.0, y]).unwrap();
            assert!(
                (got - want).abs() <= 0.1 + 1e-9,
                "row {} x{c}: {got:.2} vs {want}",
                row[0]
            );
            n += 1;
        }
    }
    assert_eq!(n, 28);
    assert!(f.balance.abs() < 1e-4);
}

#[test]
fn case_2_frame() {
    let Some(t) = tables() else { return };
    let g = table(&t, "pripad2_geometria");
    let m = table(&t, "pripad2_materialy");
    let res = table(&t, "pripad2_vysledky");
    let k = |key: &str| val(g, key, "mm") * 0.001;
    let (ab, ac, cd, cf, em, gj, im, leg) = (
        k("AB"),
        k("AC"),
        k("CD"),
        k("CF"),
        k("EM"),
        k("GJ"),
        k("IM"),
        k("FG–KJ"),
    );
    let h = ac + em + im;
    let lam = |key: &str| val(m, key, "value");
    let d = Detail {
        materials: vec![
            Material {
                name: "1".into(),
                lambda: lam("lambda_1_beton"),
            },
            Material {
                name: "2".into(),
                lambda: lam("lambda_2_drevo"),
            },
            Material {
                name: "3".into(),
                lambda: lam("lambda_3_izolacia"),
            },
            Material {
                name: "4".into(),
                lambda: lam("lambda_4_hlinik"),
            },
        ],
        regions: vec![
            Region::rect(2, 0.0, ac, ab, h),
            Region::rect(0, 0.0, 0.0, ab, ac),
            Region::rect(1, 0.0, ac, cd, ac + cf),
            Region::rect(3, 0.0, ac + cf, cd, ac + cf + gj),
            Region::rect(3, 0.0, ac + cf, leg, h),
            Region::rect(3, 0.0, ac + em, ab, h),
        ],
        envs: vec![
            Env {
                name: "HI".into(),
                theta: lam("theta_HI"),
            },
            Env {
                name: "AB".into(),
                theta: lam("theta_AB"),
            },
        ],
        edges: vec![
            Edge {
                a: [0.0, 0.0],
                b: [ab, 0.0],
                env: 1,
                rs: lam("Rse_AB"),
            },
            Edge {
                a: [0.0, h],
                b: [ab, h],
                env: 0,
                rs: lam("Rsi_HI"),
            },
        ],
    };
    let f = d.solve(&Mesh {
        h_max: 0.002,
        h_min: 0.00025,
        ratio: 1.2,
    });
    let pts = [
        ("A", [0.0, 0.0]),
        ("B", [ab, 0.0]),
        ("C", [0.0, ac]),
        ("D", [cd, ac]),
        ("E", [ab, ac]),
        ("F", [0.0, ac + cf]),
        ("G", [cd, ac + cf]),
        ("H", [0.0, h]),
        ("I", [ab, h]),
    ];
    for (name, p) in pts {
        let want = val(res, name, "theta_C");
        let got = f.at(p).unwrap();
        eprintln!("{name}: {got:.2} °C (norma {want})");
        assert!(
            (got - want).abs() <= 0.1 + 1e-9,
            "{name}: {got:.3} vs {want}"
        );
    }
    let want = val(res, "celkova_hustota_toku_W_m", "theta_C");
    eprintln!(
        "tok: {:.3} W/m (norma {want}), uzlov {}",
        f.flows[0],
        f.nodes()
    );
    assert!(
        (f.flows[0] - want).abs() <= 0.1 + 1e-9,
        "flow {:.3} vs {want}",
        f.flows[0]
    );
    assert!(f.balance.abs() < 1e-4);
}
