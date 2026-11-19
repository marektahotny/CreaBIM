//! The office's title block on a sheet with no template of its own, as
//! PilonDesigner drew it (after the office's `RohoveRazitko.dxf`) and
//! as its printed drawings carry it: 160.6 × 54.2 mm in the lower right
//! corner, 5 mm from the paper's edge and 2 mm up; the three names, the
//! client, the profession, the building and its place, the drawing's
//! name; to the right the format (`60,5 x 42,0 cm (6 x A4)`), the date,
//! the stage, the job number, the scale and the drawing's number; over
//! it the project's height above the sea. Everything is filled from the
//! project's information and the sheet. No border is drawn: thin marks
//! at the edges show where a sheet larger than A4 folds to A4 (STN EN
//! ISO 5457, DIN 824), and corner marks where the plot is trimmed.

use crate::paper::{Anchor, Canvas, Stroke};
use crate::sheet::SheetInfo;
use creabim_core::project::ProjectInfo;
use creabim_core::words::tr;
use creabim_core::Sheet;
use glam::DVec2;

/// The drawing's edge inside the paper's.
pub const EDGE: f64 = 5.0;
/// The block stands this far above the drawing's lower edge.
pub const LIFT: f64 = 2.0;
pub const W: f64 = 160.6;
pub const H: f64 = 54.2;

/// The block's lines from its lower right corner, x to the left
/// negative, y up, and their widths.
const LINES: [[f64; 5]; 18] = [
    [0.0, 0.0, -160.6, 0.0, 0.5],
    [0.0, 9.2, -160.6, 9.2, 0.13],
    [-50.1, 29.6, -160.6, 29.6, 0.13],
    [0.0, 54.2, -160.6, 54.2, 0.5],
    [-50.1, 49.1, -160.6, 49.1, 0.13],
    [-50.1, 44.0, -160.6, 44.0, 0.13],
    [-50.1, 39.9, -160.6, 39.9, 0.5],
    [-50.1, 34.8, -160.6, 34.8, 0.13],
    [-160.6, 0.0, -160.6, 54.2, 0.5],
    [0.0, 0.0, 0.0, 54.2, 0.5],
    [-25.6, 0.0, -25.6, 29.6, 0.13],
    [-50.1, 0.0, -50.1, 54.2, 0.5],
    [0.0, 14.3, -50.1, 14.3, 0.13],
    [0.0, 19.4, -50.1, 19.4, 0.13],
    [0.0, 24.5, -50.1, 24.5, 0.13],
    [-50.1, 29.6, 0.0, 29.6, 0.5],
    [-84.9, 39.9, -84.9, 54.2, 0.13],
    [-119.7, 39.9, -119.7, 54.2, 0.13],
];

/// How a text sits on its point: along (start, middle) and up (0 its
/// foot, 1 its middle, 2 its head).
type Sit = (Anchor, u8);
const MID: Sit = (Anchor::Middle, 1);
const LEFT: Sit = (Anchor::Start, 1);
const FOOT: Sit = (Anchor::Start, 0);
const FOOT_MID: Sit = (Anchor::Middle, 0);
const HEAD: Sit = (Anchor::Start, 2);
const HEAD_MID: Sit = (Anchor::Middle, 2);

/// Where folds to A4 cross a sheet `w` × `h`: upright ones 210 mm from
/// the right edge and every 190 mm on (the strip left over keeps the
/// binding edge), level ones every 297 mm up from the lower edge (the
/// title block's corner stays on top).
pub fn fold_marks(w: f64, h: f64) -> (Vec<f64>, Vec<f64>) {
    let (mut xs, mut ys) = (vec![], vec![]);
    // an A4 is not folded
    if w * h <= 297.0 * 210.0 + 1.0 {
        return (xs, ys);
    }
    let mut x = w - 210.0;
    while x >= 40.0 {
        xs.push(x);
        x -= 190.0;
    }
    let mut y = 297.0;
    while h - y >= 40.0 {
        ys.push(y);
        y += 297.0;
    }
    (xs, ys)
}

/// The format as the block writes it: `60,5 x 42,0 cm (6 x A4)`, with
/// how many A4 the sheet folds to.
pub fn format_label(w: f64, h: f64) -> String {
    // an A4 either way up is one
    let n = if w * h <= 297.0 * 210.0 + 1.0 {
        1.0
    } else {
        (w / 210.0 - 0.08).ceil().max(1.0) * (h / 297.0 - 0.08).ceil().max(1.0)
    };
    let cm = |v: f64| format!("{:.1}", v / 10.0).replace('.', ",");
    format!("{} x {} cm ({} x A4)", cm(w), cm(h), n as u32)
}

/// The profession a sheet belongs to: its own (`fields["profession"]`),
/// or by its number in the drawing set (`S01_ZTI_001_N01`).
pub fn profession(sheet: &Sheet) -> String {
    if let Some(p) = sheet
        .fields
        .get("profession")
        .filter(|p| !p.trim().is_empty())
    {
        return p.clone();
    }
    let name = [
        ("_STA_", "Structural design"),
        ("_ZTI_", "Plumbing"),
        ("_VYK_", "Heating"),
        ("_VZT_", "Ventilation"),
        ("_PLY_", "Gas installation"),
        ("_ELI_", "Electrical installation"),
        ("C00_", "Site drawings"),
    ]
    .iter()
    .find(|(k, _)| sheet.number.contains(k))
    .map_or("Architecture and construction", |p| p.1);
    tr(name)
}

/// The date as the block writes it: an ISO day as `11 / 2026`, anything
/// else as given.
fn month(date: &str) -> String {
    let parts: Vec<&str> = date.trim().split('-').collect();
    match parts.as_slice() {
        [y, m, _] if y.len() == 4 => match m.parse::<u32>() {
            Ok(m) => format!("{m} / {y}"),
            Err(_) => date.to_string(),
        },
        _ => date.trim().to_string(),
    }
}

/// The marks at a sheet's edges and, unless the sheet goes without, its
/// title block.
pub fn draw(
    c: &mut dyn Canvas,
    sheet: &Sheet,
    project: &ProjectInfo,
    title: &str,
    scales: &str,
    info: &SheetInfo,
) {
    let (w, h) = (sheet.width, sheet.height);
    let hair = Stroke::solid(0.1, 0.0);
    // folds to A4, on anything larger: 5 mm in from the drawing's edge
    let (xs, ys) = fold_marks(w, h);
    for x in xs {
        c.line(&[DVec2::new(x, EDGE), DVec2::new(x, EDGE + 5.0)], hair);
        c.line(
            &[DVec2::new(x, h - EDGE), DVec2::new(x, h - EDGE - 5.0)],
            hair,
        );
    }
    for y in ys {
        c.line(&[DVec2::new(EDGE, y), DVec2::new(EDGE + 5.0, y)], hair);
        c.line(
            &[DVec2::new(w - EDGE, y), DVec2::new(w - EDGE - 5.0, y)],
            hair,
        );
    }
    // where the plot is trimmed: the upper left and lower right corners
    for (at, sx, sy) in [
        (DVec2::new(EDGE, h - EDGE), 1.0, -1.0),
        (DVec2::new(w - EDGE, EDGE), -1.0, 1.0),
    ] {
        c.line(
            &[
                at + DVec2::new(8.0 * sx, 0.0),
                at,
                at + DVec2::new(0.0, 8.0 * sy),
            ],
            hair,
        );
    }
    if !sheet.title_block {
        return;
    }
    let o = DVec2::new(w - EDGE, EDGE + LIFT);
    for [x1, y1, x2, y2, pen] in LINES {
        c.line(
            &[o + DVec2::new(x1, y1), o + DVec2::new(x2, y2)],
            Stroke::solid(pen, 0.0),
        );
    }
    // the office's logo in the cell at the upper right, whole
    if let Some(r) = project
        .logo
        .as_ref()
        .and_then(|l| crate::raster::stored_image(l.width, l.height, &l.rgba))
    {
        let (lo, hi) = (o + DVec2::new(-49.6, 30.1), o + DVec2::new(-0.5, 53.7));
        let (box_, mid) = (hi - lo, (lo + hi) / 2.0);
        let l = project.logo.as_ref().map_or((1.0, 1.0), |l| {
            (l.width.max(1) as f64, l.height.max(1) as f64)
        });
        let k = (box_.x / l.0).min(box_.y / l.1);
        let half = DVec2::new(l.0, l.1) * k / 2.0;
        c.image(mid - half, mid + half, None, &r);
    }
    let or = |a: &str, b: &str| {
        if a.trim().is_empty() {
            b.trim().to_string()
        } else {
            a.trim().to_string()
        }
    };
    let place = [
        (!project.parcels.trim().is_empty())
            .then(|| format!("{} {}", tr("plot no."), project.parcels.trim())),
        (!project.cadastre.trim().is_empty())
            .then(|| format!("{} {}", tr("cad. district"), project.cadastre.trim())),
        (!project.address.trim().is_empty()).then(|| project.address.trim().to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let height = if project.elevation.abs() > 1e-9 {
        format!(
            "±0,000 ≡ {} {}",
            format!("{:.2}", project.elevation).replace('.', ","),
            tr("m a.s.l. (Bpv)")
        )
    } else {
        String::new()
    };
    let building = or(&project.building_name, &info.project);
    let date = or(&project.issue_date, &month(&info.date));
    let texts: [(f64, f64, f64, Sit, f64, String); 27] = [
        (-140.1, 52.2, 2.5, MID, 38.0, tr("CHIEF ARCHITECT")),
        (-102.3, 52.2, 2.5, MID, 32.0, tr("DRAWN BY")),
        (-67.5, 52.2, 2.5, MID, 32.0, tr("RESPONSIBLE DESIGNER")),
        (
            -140.1,
            46.3,
            2.5,
            MID,
            38.0,
            or(&project.architect, &project.designer),
        ),
        (
            -102.3,
            46.3,
            2.5,
            MID,
            32.0,
            or(&project.drawn_by, &project.designer),
        ),
        (
            -67.5,
            46.3,
            2.5,
            MID,
            32.0,
            project.designer.trim().to_string(),
        ),
        (-158.5, 37.2, 2.5, LEFT, 15.0, tr("CLIENT")),
        (
            -142.2,
            37.2,
            2.5,
            LEFT,
            90.0,
            project.client.trim().to_string(),
        ),
        (-158.5, 32.1, 2.5, LEFT, 15.0, tr("PROFESSION")),
        (-142.2, 32.1, 2.5, LEFT, 90.0, profession(sheet)),
        (-158.6, 21.7, 4.0, FOOT, 107.0, building),
        (-158.5, 11.5, 2.5, FOOT, 26.0, tr("BUILDING SITE")),
        (-130.2, 11.5, 2.5, FOOT, 78.0, place),
        (-158.8, 2.0, 3.0, FOOT, 107.0, title.to_string()),
        (-48.1, 27.1, 2.5, LEFT, 23.0, tr("FORMAT")),
        (-48.1, 22.0, 2.5, LEFT, 23.0, tr("DATE")),
        (-48.1, 16.9, 2.5, LEFT, 23.0, tr("STAGE")),
        (-48.1, 11.8, 2.5, LEFT, 23.0, tr("JOB NO.")),
        (-12.8, 27.1, 2.5, MID, 24.0, format_label(w, h)),
        (-12.8, 22.0, 2.5, MID, 24.0, date),
        (
            -12.8,
            16.9,
            2.5,
            MID,
            24.0,
            project.status.trim().to_string(),
        ),
        (
            -12.8,
            11.8,
            2.5,
            MID,
            24.0,
            project.number.trim().to_string(),
        ),
        (-48.1, 7.9, 2.5, HEAD, 23.0, tr("SCALE")),
        (-37.9, 1.4, 3.0, FOOT_MID, 23.0, scales.to_string()),
        (-12.8, 8.6, 2.5, HEAD_MID, 24.0, tr("DRAWING NO.")),
        (-12.8, 0.6, 3.0, FOOT_MID, 24.0, sheet.number.clone()),
        (-160.6, 55.2, 3.0, FOOT, 110.0, height),
    ];
    for (x, y, size, (anchor, up), room, text) in texts {
        if text.is_empty() {
            continue;
        }
        // a text too long for its cell is written smaller
        let wide = crate::paper::rough_width(&text, size);
        let size = if wide > room {
            size * room / wide
        } else {
            size
        };
        let lift = match up {
            0 => size * 0.5,
            2 => -size * 0.5,
            _ => 0.0,
        };
        c.text(o + DVec2::new(x, y + lift), size, &text, anchor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_say_how_many_a4_they_fold_to() {
        assert_eq!(format_label(605.0, 420.0), "60,5 x 42,0 cm (6 x A4)");
        assert_eq!(format_label(297.0, 210.0), "29,7 x 21,0 cm (1 x A4)");
        assert_eq!(format_label(420.0, 297.0), "42,0 x 29,7 cm (2 x A4)");
        assert_eq!(format_label(841.0, 594.0), "84,1 x 59,4 cm (8 x A4)");
        // an A4 has no folds, an A3 one, 630 × 420 two and one
        assert_eq!(fold_marks(297.0, 210.0), (vec![], vec![]));
        assert_eq!(fold_marks(420.0, 297.0), (vec![210.0], vec![]));
        assert_eq!(
            fold_marks(630.0, 420.0),
            (vec![420.0, 230.0, 40.0], vec![297.0])
        );
        assert_eq!(month("2026-11-18"), "11 / 2026");
        assert_eq!(month("12 / 2025"), "12 / 2025");
    }

    #[test]
    fn the_block_is_filled_from_the_project_and_can_be_left_out() {
        use crate::paper::SvgCanvas;
        let mut sheet = Sheet::a("S01_ZTI_001_N01", "A3");
        let project = ProjectInfo {
            client: "DM s.r.o.".into(),
            designer: "Marek Tahotný".into(),
            number: "27/2025".into(),
            status: "PROJEKT STAVBY".into(),
            parcels: "1838/116".into(),
            elevation: 134.82,
            building_name: "Rodinný dom".into(),
            ..Default::default()
        };
        let info = SheetInfo {
            project: "file".into(),
            date: "2026-11-18".into(),
        };
        let svg = |s: &Sheet| {
            let mut c = SvgCanvas::new(DVec2::new(s.width, s.height));
            draw(&mut c, s, &project, "Pôdorys 1. NP", "1:50", &info);
            c.finish()
        };
        let on = svg(&sheet);
        for want in [
            "DM s.r.o.",
            "Marek Tahotný",
            "27/2025",
            "PROJEKT STAVBY",
            "1838/116",
            "134,82",
            "Rodinný dom",
            "Pôdorys 1. NP",
            "1:50",
            "S01_ZTI_001_N01",
            "42,0 x 29,7 cm (2 x A4)",
            "11 / 2026",
            "Plumbing",
        ] {
            assert!(on.contains(want), "{want}");
        }
        // the designer signs all three when no one else is named
        assert_eq!(on.matches("Marek Tahotný").count(), 3);
        // the office's logo goes in its cell
        assert!(!on.contains("<image"));
        let logo = ProjectInfo {
            logo: Some(creabim_core::project::Logo {
                width: 2,
                height: 1,
                rgba: crate::raster::store_image(&[255, 0, 0, 255, 0, 0, 255, 255]),
            }),
            ..project.clone()
        };
        let mut c = SvgCanvas::new(DVec2::new(sheet.width, sheet.height));
        draw(&mut c, &sheet, &logo, "x", "1:50", &info);
        assert!(c.finish().contains("<image"));
        sheet.title_block = false;
        let off = svg(&sheet);
        assert!(!off.contains("DM s.r.o.") && !off.contains("S01_ZTI"));
        // the fold and trim marks stay
        assert!(off.contains("<polyline") || off.contains("<line") || off.contains("<path"));
    }
}
