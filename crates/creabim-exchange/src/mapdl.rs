//! A package from Pilon Map Downloader (a ZIP: `situacia.dxf`,
//! `ortofoto.jpg` with its `.jgw`, `teren.xyz`, `bod_pociatku.txt`),
//! ported from PilonDesigner's `mapdl.ts`: the site plan's drawings
//! (cadastre, networks, contours), the orthophoto under them, a terrain
//! from the elevation model, and where the project is in S-JTSK
//! (EPSG:5514) and above sea level (Bpv). When it holds both local and
//! S-JTSK files, the local ones are taken.

use crate::xyz::is_jtsk;
use glam::DVec2;
use std::io::Read;

/// Where the project's origin is: east and north in S-JTSK (negative),
/// ±0,000 metres above sea level, and the ground's height there if known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Origin {
    pub e: f64,
    pub n: f64,
    pub h0: f64,
    pub ground: Option<f64>,
}

/// What a package holds, by the names of its files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Package {
    pub origin: Option<Origin>,
    /// The site plan's drawings, and whether each is in S-JTSK.
    pub dxfs: Vec<(String, bool)>,
    /// The elevation model's points.
    pub xyz: Option<String>,
    /// The orthophoto and its world file.
    pub ortho: Option<(String, String)>,
}

/// A number as the Downloader writes it (a comma or a point, spaces
/// between thousands).
fn num(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}' && *c != '\u{202f}')
        .map(|c| match c {
            ',' => '.',
            '−' => '-',
            c => c,
        })
        .collect();
    t.parse().ok().filter(|v: &f64| v.is_finite())
}

/// The number after `key =` in `line` (where `key` is followed by `=`).
fn after(line: &str, key: &str) -> Option<f64> {
    line.match_indices(key).find_map(|(i, _)| {
        let rest = line[i + key.len()..]
            .trim_start()
            .strip_prefix('=')?
            .trim_start();
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || matches!(c, '-' | '−' | '.' | ',')))
            .unwrap_or(rest.len());
        num(&rest[..end])
    })
}

/// `bod_pociatku.txt`: the origin's E and N, ±0,000 and the ground there.
pub fn parse_origin(text: &str) -> Option<Origin> {
    let text = text.trim_start_matches('\u{feff}');
    let mut en = None;
    let mut h0 = 0.0;
    let mut ground = None;
    for line in text.lines() {
        if en.is_none() {
            if let (Some(e), Some(n)) = (after(line, " E"), after(line, " N")) {
                en = Some((e, n));
            }
        }
        if let Some(h) = after(line, "±0,000") {
            h0 = h;
        }
        if line.contains("m n. m.") && line.contains(" Z") {
            ground = after(line, " Z").or(ground);
        }
    }
    let (e, n) = en?;
    Some(Origin { e, n, h0, ground })
}

/// An orthophoto's lower left corner and size, metres, from its world
/// file and its size in pixels; in S-JTSK it is brought to `origin`.
pub fn ortho_placement(world: &str, px: (u32, u32), origin: Option<&Origin>) -> Option<[DVec2; 2]> {
    let v: Vec<f64> = world.split_whitespace().filter_map(num).collect();
    let [a, _, _, e, c, f] = v.get(..6)?.try_into().ok()?;
    if a <= 0.0 || e == 0.0 {
        return None;
    }
    let mut min_x = c - a / 2.0;
    let mut max_y = f - e / 2.0;
    if is_jtsk(min_x, max_y) {
        let o = origin?;
        min_x -= o.e;
        max_y -= o.n;
    }
    let size = DVec2::new(a * px.0 as f64, e.abs() * px.1 as f64);
    Some([DVec2::new(min_x, max_y - size.y), size])
}

/// The package's files by name (folders left out of the names).
pub fn unzip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut out = vec![];
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f
            .name()
            .map_err(|e| e.to_string())?
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        // only a name, never a path out of the folder it is put in
        if name.is_empty() || name.contains('\\') || name.starts_with('.') {
            continue;
        }
        let mut b = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut b).map_err(|e| e.to_string())?;
        out.push((name, b));
    }
    Ok(out)
}

/// What the files of a package are (`origin` from `bod_pociatku.txt`).
pub fn package(files: &[(String, Vec<u8>)]) -> Package {
    let lower: Vec<String> = files.iter().map(|(n, _)| n.to_lowercase()).collect();
    let find = |f: &dyn Fn(&str) -> bool| lower.iter().position(|n| f(n));
    let text = |i: usize| crate::dxf::decode(&files[i].1);
    let origin = find(&|n| n == "bod_pociatku.txt").and_then(|i| parse_origin(&text(i)));
    let local = lower
        .iter()
        .any(|n| n.ends_with("_lokal.dxf") || n.ends_with("_lokal.xyz"));
    let pick = |n: &str| !local || !n.contains("_jtsk.");
    let dxfs = lower
        .iter()
        .enumerate()
        .filter(|(_, n)| n.ends_with(".dxf") && !n.starts_with("teren_3d") && pick(n))
        .map(|(i, n)| (files[i].0.clone(), n.contains("_jtsk.")))
        .collect();
    let xyz = find(&|n| n.ends_with(".xyz") && pick(n)).map(|i| files[i].0.clone());
    let img = find(&|n| matches!(n, "ortofoto.jpg" | "ortofoto.jpeg" | "ortofoto.png"));
    let world = find(&|n| matches!(n, "ortofoto.jgw" | "ortofoto.pgw" | "ortofoto.wld"));
    Package {
        origin,
        dxfs,
        xyz,
        ortho: img
            .zip(world)
            .map(|(i, w)| (files[i].0.clone(), files[w].0.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "\u{feff}Bod lokálneho počiatku (Pilon Map Downloader)\r\n\
        S-JTSK / Krovak East North (EPSG:5514):  E = -450123.456   N = -1250456.789\r\n\
        S-JTSK (Y, X kladné):                    Y = 450123.456   X = 1250456.789\r\n\
        WGS84:  šírka 48.1234567°  dĺžka 17.1234567°\r\n\
        Výška ±0,000 = 312.450 m n. m. (Bpv)\r\n\
        Výška terénu v bode počiatku Z = 311.900 m n. m. (Bpv)  →  lokálne z = -0.550\r\n";

    #[test]
    fn the_origin_is_read_from_its_text() {
        let o = parse_origin(ORIGIN).unwrap();
        assert_eq!((o.e, o.n, o.h0), (-450123.456, -1250456.789, 312.45));
        assert_eq!(o.ground, Some(311.9));
        assert!(parse_origin("nothing here").is_none());
    }

    #[test]
    fn an_orthophoto_is_placed_by_its_world_file() {
        let o = parse_origin(ORIGIN).unwrap();
        // 0.1 m pixels, upper left pixel's middle in S-JTSK
        let w = "0.1\r\n0.0\r\n0.0\r\n-0.1\r\n-450173.40\r\n-1250406.80\r\n";
        let [min, size] = ortho_placement(w, (1000, 800), Some(&o)).unwrap();
        assert!((size - DVec2::new(100.0, 80.0)).length() < 1e-9);
        assert!(
            (min - DVec2::new(-49.994, -29.961)).length() < 1e-6,
            "{min}"
        );
        // local: as it is
        let w = "0.5 0 0 -0.5 -20.25 40.25";
        let [min, _] = ortho_placement(w, (100, 100), None).unwrap();
        assert!((min - DVec2::new(-20.5, -9.5)).length() < 1e-9);
    }

    #[test]
    fn local_files_are_taken_over_sjtsk_ones() {
        let f = |n: &str| (n.to_string(), vec![]);
        let p = package(&[
            ("bod_pociatku.txt".into(), ORIGIN.as_bytes().to_vec()),
            f("situacia_lokal.dxf"),
            f("situacia_jtsk.dxf"),
            f("teren_3D_lokal.dxf"),
            f("teren_lokal.xyz"),
            f("teren_jtsk.xyz"),
            f("ortofoto.jpg"),
            f("ortofoto.jgw"),
        ]);
        assert!(p.origin.is_some());
        assert_eq!(p.dxfs, vec![("situacia_lokal.dxf".to_string(), false)]);
        assert_eq!(p.xyz.as_deref(), Some("teren_lokal.xyz"));
        assert_eq!(
            p.ortho,
            Some(("ortofoto.jpg".into(), "ortofoto.jgw".into()))
        );
        let only = package(&[f("situacia_jtsk.dxf")]);
        assert_eq!(only.dxfs, vec![("situacia_jtsk.dxf".to_string(), true)]);
    }
}
