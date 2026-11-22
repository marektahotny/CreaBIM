//! The fixtures PilonDesigner connects that the starter catalogue lacks:
//! a washing machine, a dishwasher, a bidet, a urinal, a towel radiator,
//! an underfloor heating manifold and three fan coils. Their sizes and
//! plan symbols are PilonDesigner's ([`crate::fixture::shape`]), turned
//! into the object's own axes, so their connectors
//! ([`crate::fixture::connectors`]) sit where the model shows them.

use super::{
    appliance_white, ceramic, chrome, cuboid, cylinder, look, metal, soft_box, squircle, stack,
    stainless, v2, v3, Build,
};
use crate::fixture::{shape, slug, to_object_axes, Shape};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::Hatch;
use glam::DVec3;

/// The plan symbol of `s`, in the object's axes.
fn symbol(b: &mut Build, s: &Shape) {
    let to = |p: glam::DVec2| to_object_axes(p.extend(0.0)).truncate();
    for c in &s.curves {
        let closed = c.len() > 2 && c.first() == c.last();
        if closed {
            b.outline(c[..c.len() - 1].iter().map(|&p| to(p)).collect());
        } else {
            for w in c.windows(2) {
                b.line(to(w[0]), to(w[1]));
            }
        }
    }
}

/// A box of `s`'s width and depth, `y0` to `y1` out from the wall
/// (PilonDesigner's sense), `z0` to `z1` high.
fn body(s: &Shape, y0: f64, y1: f64, z0: f64, z1: f64) -> (DVec3, DVec3) {
    (v3(-s.width / 2.0, -y1, z0), v3(s.width / 2.0, -y0, z1))
}

fn finish(
    mut b: Build,
    family: &str,
    name: &str,
    description: &str,
    category: ObjectCategory,
    ifc: (&str, &str),
    mount: Mount,
) -> CatalogueObject {
    let s = shape(family, None);
    symbol(&mut b, &s);
    b.finish(
        &format!("pd-{}", slug(family)),
        name,
        description,
        category,
        ifc,
        mount,
    )
}

fn washing_machine() -> CatalogueObject {
    let f = "Práčka";
    let s = shape(f, None);
    let mut b = Build::new();
    let (lo, hi) = body(&s, 0.0, s.depth, 0.0, s.height);
    b.add(
        "Body",
        appliance_white(),
        soft_box(lo, hi, 0.02, 0.008),
        false,
    );
    b.add(
        "Controls",
        look(
            "Plastic, charcoal",
            "plastic",
            Hatch::Empty,
            [46, 48, 52],
            0.5,
        ),
        cuboid(
            v3(lo.x + 0.02, lo.y - 0.006, hi.z - 0.1),
            v3(hi.x - 0.02, lo.y + 0.01, hi.z - 0.03),
        ),
        false,
    );
    finish(
        b,
        f,
        "Washing machine",
        "Front-loading washing machine 600 × 600 with its cold water and waste",
        ObjectCategory::Plumbing,
        ("IfcElectricAppliance", "WASHINGMACHINE"),
        Mount::Wall,
    )
}

fn dishwasher() -> CatalogueObject {
    let f = "Umývačka riadu";
    let s = shape(f, None);
    let mut b = Build::new();
    let (lo, hi) = body(&s, 0.0, s.depth - 0.02, 0.0, s.height);
    b.add("Body", appliance_white(), cuboid(lo, hi), false);
    b.add(
        "Door",
        stainless(),
        soft_box(
            v3(lo.x + 0.003, lo.y - 0.02, 0.1),
            v3(hi.x - 0.003, lo.y, hi.z - 0.003),
            0.004,
            0.002,
        ),
        false,
    );
    finish(
        b,
        f,
        "Dishwasher",
        "Dishwasher 600 wide with its cold water and waste",
        ObjectCategory::Plumbing,
        ("IfcElectricAppliance", "DISHWASHER"),
        Mount::Wall,
    )
}

fn bidet() -> CatalogueObject {
    let f = "Bidet";
    let s = shape(f, None);
    let n = 32;
    let cy = -0.3;
    let mut b = Build::new();
    b.add(
        "Bowl",
        ceramic(),
        stack(&[
            (squircle(v2(0.0, cy + 0.02), 0.12, 0.17, 2.6, n), 0.0),
            (squircle(v2(0.0, cy + 0.01), 0.13, 0.2, 2.6, n), 0.22),
            (squircle(v2(0.0, cy), 0.18, 0.25, 2.4, n), s.height - 0.01),
            (squircle(v2(0.0, cy), 0.17, 0.24, 2.4, n), s.height),
            (squircle(v2(0.0, cy), 0.1, 0.16, 2.4, n), s.height - 0.005),
            (squircle(v2(0.0, cy), 0.06, 0.1, 2.4, n), 0.25),
        ]),
        false,
    );
    b.add(
        "Mixer",
        chrome(),
        cylinder(v2(0.0, -0.1), 0.022, s.height, s.height + 0.12),
        false,
    );
    finish(
        b,
        f,
        "Bidet",
        "Floor-standing bidet with a mixer, its hot and cold water and waste",
        ObjectCategory::Bathroom,
        ("IfcSanitaryTerminal", "BIDET"),
        Mount::Wall,
    )
}

fn wc_wall_hung() -> CatalogueObject {
    let f = "Závesné WC s podomietkovou nádržkou";
    let s = shape(f, None);
    let n = 32;
    // the pan's middle, out from the wall
    let cy = -0.3;
    let rim = s.height - 0.02;
    let mut b = Build::new();
    b.add(
        "Pan",
        ceramic(),
        stack(&[
            (squircle(v2(0.0, -0.12), 0.1, 0.1, 2.6, n), s.bottom),
            (squircle(v2(0.0, -0.17), 0.15, 0.15, 2.6, n), s.bottom + 0.1),
            (
                squircle(v2(0.0, cy + 0.035), 0.18, 0.265, 2.4, n),
                rim - 0.02,
            ),
            (squircle(v2(0.0, cy + 0.035), 0.18, 0.265, 2.4, n), rim),
            (squircle(v2(0.0, cy), 0.13, 0.2, 2.4, n), rim - 0.01),
            (squircle(v2(0.0, cy), 0.07, 0.1, 2.4, n), s.bottom + 0.16),
        ]),
        false,
    );
    b.add(
        "Seat and lid",
        ceramic(),
        stack(&[
            (squircle(v2(0.0, cy + 0.03), 0.18, 0.26, 2.4, n), rim),
            (
                squircle(v2(0.0, cy + 0.03), 0.182, 0.262, 2.4, n),
                rim + 0.012,
            ),
            (squircle(v2(0.0, cy + 0.03), 0.172, 0.25, 2.4, n), s.height),
        ]),
        false,
    );
    // the flush plate 246 × 164, its middle 1 m above the floor
    b.add(
        "Flush plate",
        chrome(),
        soft_box(
            v3(-0.123, -0.012, 0.918),
            v3(0.123, 0.0, 1.082),
            0.004,
            0.002,
        ),
        false,
    );
    finish(
        b,
        f,
        "WC, wall-hung with concealed cistern",
        "Wall-hung pan 360 × 530, rim 400 mm, on an installation frame in the wall (500 wide, 1120 high, 120 deep or more) with its concealed cistern and flush plate; water DN 15 in the frame, waste DN 100",
        ObjectCategory::Toilet,
        ("IfcSanitaryTerminal", "TOILETPAN"),
        Mount::Wall,
    )
}

fn urinal() -> CatalogueObject {
    let f = "Pisoár";
    let s = shape(f, None);
    let n = 28;
    let cy = -0.17;
    let mut b = Build::new();
    b.add(
        "Bowl",
        ceramic(),
        stack(&[
            (squircle(v2(0.0, -0.1), 0.1, 0.1, 2.4, n), s.bottom),
            (squircle(v2(0.0, cy), 0.17, 0.15, 2.4, n), s.bottom + 0.1),
            (squircle(v2(0.0, cy + 0.02), 0.17, 0.13, 2.4, n), s.height),
            (
                squircle(v2(0.0, -0.08), 0.15, 0.07, 2.4, n),
                s.height + 0.05,
            ),
        ]),
        false,
    );
    b.add(
        "Flush valve",
        chrome(),
        cylinder(v2(0.0, -0.03), 0.02, s.height + 0.05, 1.2),
        false,
    );
    finish(
        b,
        f,
        "Urinal",
        "Wall-hung urinal with a flush valve, rim at 650 mm",
        ObjectCategory::Toilet,
        ("IfcSanitaryTerminal", "URINAL"),
        Mount::Wall,
    )
}

/// The towel radiator: a KORADO KORALUX LINEAR CLASSIC-M 1220.500 under
/// PilonDesigner's family key, so the drawings and connections know it.
fn towel_radiator() -> CatalogueObject {
    let mut c = super::towels::TowelRail {
        height: 1220,
        length: 500,
    }
    .object();
    c.key = format!("pd-{}", slug("Kúpeľňový rebrík"));
    c
}

fn manifold() -> CatalogueObject {
    let f = "Rozdeľovač podlahového vykurovania";
    let s = shape(f, None);
    let mut b = Build::new();
    let (lo, hi) = body(&s, 0.01, 0.01 + s.depth, s.bottom, s.height);
    b.add("Cabinet", appliance_white(), cuboid(lo, hi), false);
    let brass = metal("Brass", [181, 148, 82], 0.3);
    for (i, z) in [s.bottom + 0.15, s.bottom + 0.35].into_iter().enumerate() {
        b.add(
            if i == 0 { "Return bar" } else { "Supply bar" },
            brass.clone(),
            cuboid(
                v3(lo.x + 0.06, lo.y - 0.02, z - 0.015),
                v3(hi.x - 0.06, lo.y, z + 0.015),
            ),
            false,
        );
    }
    finish(
        b,
        f,
        "Underfloor heating manifold",
        "Manifold for underfloor heating loops in its cabinet, with its heating supply and return",
        ObjectCategory::Hvac,
        ("IfcSpaceHeater", "USERDEFINED"),
        Mount::Wall,
    )
}

fn fan_coil(f: &'static str, name: &str, description: &str) -> CatalogueObject {
    let s = shape(f, None);
    let mut b = Build::new();
    let grille = look(
        "Plastic, charcoal",
        "plastic",
        Hatch::Empty,
        [46, 48, 52],
        0.5,
    );
    if f.contains("kazet") {
        // hung from the ceiling: its top at the ceiling, z down from there
        let h = s.height - s.bottom;
        let c = v2(0.0, -s.depth / 2.0);
        b.add(
            "Unit",
            metal("Galvanised steel", [176, 180, 184], 0.45),
            cuboid(
                v3(c.x - 0.285, c.y - 0.285, 0.02),
                v3(c.x + 0.285, c.y + 0.285, h),
            ),
            false,
        );
        b.add(
            "Face plate",
            appliance_white(),
            cuboid(
                v3(c.x - 0.3, c.y - 0.3, 0.002),
                v3(c.x + 0.3, c.y + 0.3, 0.02),
            ),
            false,
        );
        b.add(
            "Grille",
            grille,
            cuboid(
                v3(c.x - 0.2, c.y - 0.2, 0.0),
                v3(c.x + 0.2, c.y + 0.2, 0.002),
            ),
            false,
        );
        return finish(
            b,
            f,
            name,
            description,
            ObjectCategory::Hvac,
            ("IfcUnitaryEquipment", "USERDEFINED"),
            Mount::Ceiling,
        );
    }
    let (lo, hi) = body(&s, 0.01, 0.01 + s.depth, s.bottom, s.height);
    b.add(
        "Casing",
        appliance_white(),
        soft_box(lo, hi, 0.02, 0.008),
        false,
    );
    let (z0, z1) = if f.contains("parapet") {
        (hi.z - 0.002, hi.z)
    } else {
        (lo.z, lo.z + 0.002)
    };
    b.add(
        "Grille",
        grille,
        cuboid(
            v3(lo.x + 0.05, lo.y + 0.03, z0 - 0.002),
            v3(hi.x - 0.05, hi.y - 0.04, z1 + 0.002),
        ),
        false,
    );
    finish(
        b,
        f,
        name,
        description,
        ObjectCategory::Hvac,
        ("IfcUnitaryEquipment", "USERDEFINED"),
        Mount::Wall,
    )
}

/// PilonDesigner's fixtures CreaBIM has no model of.
pub fn starters() -> Vec<CatalogueObject> {
    vec![
        washing_machine(),
        dishwasher(),
        bidet(),
        wc_wall_hung(),
        urinal(),
        towel_radiator(),
        manifold(),
        fan_coil(
            "Fan-coil nástenný",
            "Fan coil, wall",
            "Wall-mounted fan coil unit with its chilled water and condensate",
        ),
        fan_coil(
            "Fan-coil parapetný",
            "Fan coil, under window",
            "Under-window fan coil unit with its chilled water and condensate",
        ),
        fan_coil(
            "Fan-coil kazetový (strop)",
            "Fan coil, ceiling cassette",
            "Four-way ceiling cassette fan coil with its chilled water and condensate",
        ),
    ]
}
