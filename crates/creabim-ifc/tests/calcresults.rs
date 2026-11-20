//! What the calculations wrote on the model goes to IFC where viewers look
//! for it: a room's heat loss in Pset_SpaceThermalDesign, a pipe's
//! diameters and length in the standard sets, the pipes grouped in their
//! system, and every value in the calculation's own CreaBIM_ set.

use creabim_core::mep::{Pipe, Valve};
use creabim_core::{results, Document, Element, ElementKind, Transaction};
use creabim_ifc::{export_ifc4, ExportOptions};
use creabim_services::{calcresults, firewater, heatloss};
use glam::DVec3;

fn fire_main(doc: &mut Document) {
    let level = doc.model().levels().next().unwrap().0.id;
    let sys = creabim_core::mep::system("PV").unwrap();
    let ty = creabim_services::pipe_types::by_id(sys.pipe_type).unwrap();
    let z = &ty.sizes[0];
    let pipe = |a: [f64; 3], b: [f64; 3]| {
        Element::new(
            "pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: "PV".into(),
                start: DVec3::from(a),
                end: DVec3::from(b),
                pipe_type: ty.id.clone(),
                nominal_mm: z.nominal_mm,
                outer_mm: z.outer_mm,
                inner_mm: z.inner_mm,
                height_mm: None,
                insulation_mm: 0.0,
                insulation_set: false,
                mark: None,
            }),
        )
    };
    let main = pipe([1.0, 1.0, -1.0], [1.0, 1.0, 0.5]);
    let branch = pipe([1.0, 1.0, 0.5], [3.0, 1.0, 1.2]);
    let id = branch.id;
    let tx = Transaction::new("fire")
        .insert(main)
        .insert(branch)
        .insert(Element::new(
            "H1",
            ElementKind::Valve(Valve::new(id, 1.0, "hydrant")),
        ));
    doc.commit(tx).unwrap();
}

#[test]
fn results_go_to_the_rooms_pipes_and_systems_in_ifc() {
    let mut doc = Document::from_model(creabim_core::starter::starter_house());
    fire_main(&mut doc);
    let o = heatloss::Opts::default();
    let b = heatloss::building(doc.model(), &o);
    assert!(!b.rooms.is_empty());
    doc.commit(results::write(
        doc.model(),
        calcresults::heat_loss(&b, &o),
        "2026-11-19",
    ))
    .unwrap();
    let f = firewater::run(doc.model(), "PV").expect("fire main");
    doc.commit(results::write(
        doc.model(),
        calcresults::fire(&f),
        "2026-11-19",
    ))
    .unwrap();
    // the room keeps its loss
    let room = b.rooms[0].id;
    let got = results::of(doc.model(), room);
    assert_eq!(got[0].0.title, "Tepelné straty");
    assert!(got[0].1.iter().any(|v| v.key == "phi_hl"));

    // the materials' λ, for the walls' U
    let mut tx = creabim_core::Transaction::new("λ");
    let mats: Vec<creabim_core::Element> = doc
        .model()
        .iter()
        .filter(|e| matches!(e.kind, creabim_core::ElementKind::Material(_)))
        .cloned()
        .collect();
    for before in mats {
        let mut after = before.clone();
        if let creabim_core::ElementKind::Material(m) = &mut after.kind {
            m.lambda = Some(0.5);
        }
        tx = tx.replace(before, after);
    }
    doc.commit(tx).unwrap();
    let ifc = export_ifc4(doc.model(), &ExportOptions::for_model(doc.model(), "Test"));
    if let Ok(path) = std::env::var("CREABIM_IFC_OUT") {
        std::fs::write(path, &ifc).unwrap();
    }
    for want in [
        "'Pset_SpaceThermalDesign'",
        "'Pset_WallCommon'",
        "'ThermalTransmittance',$,IFCTHERMALTRANSMITTANCEMEASURE(",
        ".THERMALTRANSMITTANCEUNIT.",
        "'TotalHeatLoss',$,IFCPOWERMEASURE(",
        "'HeatingDryBulb',$,IFCTHERMODYNAMICTEMPERATUREMEASURE(",
        "'CreaBIM_TepelneStraty'",
        "'Celkov",
        "'CreaBIM_PoziarnyVodovod'",
        "IFCDISTRIBUTIONSYSTEM(",
        ".FIREPROTECTION.",
        "'Pset_PipeSegmentTypeCommon'",
        "'Qto_PipeSegmentBaseQuantities'",
        ".POWERUNIT.,$,.WATT.",
        ".VOLUMETRICFLOWRATEUNIT.",
    ] {
        assert!(ifc.contains(want), "{want} missing");
    }
    // still reads back
    creabim_ifc::import_ifc(&ifc).expect("reads back");
}
