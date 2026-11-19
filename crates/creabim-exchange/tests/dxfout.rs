//! A drawing written as DXF reads back with the program's own reader: the
//! same lines, at true size, in every version.

use creabim_core::{ElementKind, ViewKind};
use creabim_exchange::dxf;
use creabim_geom::dxfout::{view_dxf, DxfVersion};

#[test]
fn a_plan_written_as_dxf_reads_back_in_every_version() {
    let m = creabim_core::starter::starter_house();
    let (id, v) = m
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::View(v) if matches!(v.kind, ViewKind::Plan { .. }) => Some((e.id, v)),
            _ => None,
        })
        .unwrap();
    let d = creabim_geom::section::view_drawing(&m, id).unwrap();
    let looks = creabim_geom::looks::Looks::for_view(&m, id, v);
    let (lo, hi) = d.bounds().unwrap();
    let mut counts = vec![];
    for version in DxfVersion::ALL {
        let text = view_dxf(&m, &d, &looks, v.scale, version);
        let back = dxf::read(&text, None);
        assert!(back.polys.len() > 100, "{version:?}: {}", back.polys.len());
        assert!(!back.texts.is_empty(), "{version:?}");
        assert!(back.layers.len() > 3, "{version:?}: {:?}", back.layers);
        // millimetres, and the house as large as it is
        assert!((back.unit - 0.001).abs() < 1e-12 || version == DxfVersion::R12);
        let [a, b] = back.bounds.unwrap();
        let k = if version == DxfVersion::R12 {
            0.001 / back.unit
        } else {
            1.0
        };
        assert!(((b.x - a.x) * k - (hi.x - lo.x)).abs() < 2.0, "{version:?}");
        counts.push(back.polys.len());
    }
    // the same drawing in each
    assert!(counts.iter().all(|c| *c == counts[0]), "{counts:?}");
}
