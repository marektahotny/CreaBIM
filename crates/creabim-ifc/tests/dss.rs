//! Every element goes to IFC with its class of the data standard (DSS):
//! a classification reference and its values in the `DSS` set, in Slovak.

use creabim_core::mep::Pipe;
use creabim_core::{Document, Element, ElementKind, Transaction};
use creabim_ifc::{export_ifc4, ExportOptions};
use glam::DVec3;

#[test]
fn elements_systems_and_the_building_are_classified() {
    let mut doc = Document::from_model(creabim_core::starter::starter_house());
    let level = doc.model().levels().next().unwrap().0.id;
    let pipe = Element::new(
        "pipe",
        ElementKind::Pipe(Pipe {
            level,
            system: "SV".into(),
            start: DVec3::new(1.0, 1.0, 0.5),
            end: DVec3::new(3.0, 1.0, 0.5),
            pipe_type: "PPR".into(),
            nominal_mm: 20.0,
            outer_mm: 25.0,
            inner_mm: 18.0,
            height_mm: None,
            insulation_mm: 9.0,
            insulation_set: true,
            mark: None,
        }),
    );
    doc.commit(Transaction::new("pipe").insert(pipe)).unwrap();
    let ifc = export_ifc4(doc.model(), &ExportOptions::for_model(doc.model(), "Test"));
    for want in [
        "IFCCLASSIFICATION(",
        "'DSS'",
        "IFCCLASSIFICATIONREFERENCE($,'WPA001'",
        "IFCCLASSIFICATIONREFERENCE($,'JB01'",
        "IFCCLASSIFICATIONREFERENCE($,'AA01'",
        "IFCCLASSIFICATIONREFERENCE($,'QQC001'",
        "IFCRELASSOCIATESCLASSIFICATION(",
        // DN 20 on the pipe, in millimetres
        "'menovit\\X2\\00FD\\X0\\ priemer','mm',IFCREAL(20.0)",
        "'ozna\\X2\\010D\\X0\\enie syst\\X2\\00E9\\X0\\mu',$,IFCLABEL('SV')",
    ] {
        assert!(ifc.contains(want), "missing {want}");
    }
}
