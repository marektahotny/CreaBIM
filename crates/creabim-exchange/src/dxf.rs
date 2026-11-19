//! Reading a DXF drawing for a reference (a survey, a consultant's plan),
//! ported from PilonDesigner's `readDxf`: its lines, arcs, circles,
//! ellipses, splines and polylines (with bulges, 2D and 3D), texts,
//! hatches (solid fills and pattern lines), points, 3D faces and meshes,
//! all by layer, through blocks (inserts, arrays, dimensions), in metres
//! from the file's units. Heights are kept where the drawing has them (a
//! 3D survey), so a reference can stand in 3D and give spot heights.

use glam::{DVec2, DVec3};

/// A polyline of the drawing, metres.
#[derive(Clone, Debug, PartialEq)]
pub struct Poly {
    pub pts: Vec<DVec2>,
    pub closed: bool,
    pub layer: String,
    pub dashed: bool,
    /// Its vertices' heights, metres, where the drawing is 3D.
    pub z: Option<Vec<f64>>,
}

/// A text, at its insertion point, `h` metres tall, turned `angle`
/// degrees; `align` 1 to 9 as MTEXT's (1 top left … 7 bottom left).
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub p: DVec2,
    pub text: String,
    pub h: f64,
    pub angle: f64,
    pub layer: String,
    pub align: u8,
    /// Width factor.
    pub wf: f64,
}

/// A solid fill: its rings, even-odd.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub rings: Vec<Vec<DVec2>>,
    pub layer: String,
    /// 0xRRGGBB.
    pub color: u32,
}

/// A point to snap to: a POINT, the middle of a small circle (a survey
/// mark) or a block's insertion point.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub p: DVec2,
    pub layer: String,
    pub z: Option<f64>,
}

/// A 3D face (3DFACE, a polyface or polygon mesh), metres.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub pts: Vec<DVec3>,
    pub layer: String,
    pub color: u32,
}

/// What a DXF file holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DxfImport {
    pub polys: Vec<Poly>,
    pub texts: Vec<Text>,
    pub fills: Vec<Fill>,
    pub points: Vec<Mark>,
    pub faces: Vec<Face>,
    /// It has heights other than 0 (a 3D survey).
    pub has3d: bool,
    /// Its layers, in the order met.
    pub layers: Vec<String>,
    /// Its units in metres (mm: 0.001).
    pub unit: f64,
    /// What it spans in plan, metres.
    pub bounds: Option<[DVec2; 2]>,
}

impl DxfImport {
    /// The range of its heights, metres, if it has any.
    pub fn z_range(&self) -> Option<(f64, f64)> {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut add = |z: f64| {
            lo = lo.min(z);
            hi = hi.max(z);
        };
        for p in &self.polys {
            for &z in p.z.iter().flatten() {
                add(z);
            }
        }
        for m in &self.points {
            if let Some(z) = m.z {
                add(z);
            }
        }
        for f in &self.faces {
            for q in &f.pts {
                add(q.z);
            }
        }
        (lo <= hi).then_some((lo, hi))
    }

    /// Its points with heights on layers not `off` and within `clip`:
    /// the vertices of 3D lines, points and the corners of 3D faces, each
    /// place once (shared corners of a TIN), for a terrain (PilonDesigner's
    /// `cadZPoints`).
    pub fn heights(&self, off: &[String], clip: Option<[DVec2; 2]>) -> Vec<DVec3> {
        let on = |l: &str| !off.iter().any(|x| x == l);
        let inside = |p: DVec2| clip.is_none_or(|[a, b]| p.cmpge(a).all() && p.cmple(b).all());
        let mut seen = std::collections::HashSet::new();
        let mut out = vec![];
        let mut add = |q: DVec3| {
            if inside(q.truncate())
                && q.is_finite()
                && seen.insert(((q.x * 1000.0).round() as i64, (q.y * 1000.0).round() as i64))
            {
                out.push(q);
            }
        };
        for p in self.polys.iter().filter(|p| on(&p.layer)) {
            if let Some(z) = &p.z {
                for (q, z) in p.pts.iter().zip(z) {
                    add(q.extend(*z));
                }
            }
        }
        for m in self.points.iter().filter(|m| on(&m.layer)) {
            if let Some(z) = m.z {
                add(m.p.extend(z));
            }
        }
        for f in self.faces.iter().filter(|f| on(&f.layer)) {
            for q in &f.pts {
                add(*q);
            }
        }
        out
    }

    /// Far from the origin, as drawings in national coordinates are
    /// (S-JTSK: about −1 200 km, −500 km), more than 2 km.
    pub fn is_far(&self) -> bool {
        self.bounds
            .is_some_and(|[a, b]| [a.x, a.y, b.x, b.y].iter().any(|v| v.abs() > 2000.0))
    }
}

type Pair = (i32, String);

struct Ent {
    kind: String,
    pairs: Vec<Pair>,
}

impl Ent {
    fn num(&self, c: i32, d: f64) -> f64 {
        self.pairs
            .iter()
            .find(|p| p.0 == c)
            .and_then(|p| p.1.trim().parse().ok())
            .unwrap_or(d)
    }

    fn has(&self, c: i32) -> bool {
        self.pairs.iter().any(|p| p.0 == c)
    }

    fn str(&self, c: i32, d: &str) -> String {
        self.pairs
            .iter()
            .find(|p| p.0 == c)
            .map_or(d.to_string(), |p| p.1.trim().to_string())
    }

    fn int(&self, c: i32, d: i64) -> i64 {
        self.num(c, d as f64) as i64
    }
}

/// Units by `$INSUNITS`, in metres.
fn unit_of(code: i64) -> Option<f64> {
    Some(match code {
        1 => 0.0254,
        2 => 0.3048,
        4 => 0.001,
        5 => 0.01,
        6 => 1.0,
        14 => 0.1,
        _ => return None,
    })
}

fn pairs_of(text: &str) -> Vec<Pair> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::with_capacity(lines.len() / 2);
    let mut i = 0;
    while i + 1 < lines.len() {
        match lines[i].trim().parse::<i32>() {
            Ok(c) => {
                out.push((c, lines[i + 1].to_string()));
                i += 2;
            }
            Err(_) => i += 1,
        }
    }
    out
}

/// MTEXT without its formatting codes.
pub fn mtext_plain(s: &str) -> String {
    let mut out = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '\\' && i + 1 < cs.len() {
            let k = cs[i + 1];
            match k {
                'P' => {
                    out.push('\n');
                    i += 2;
                }
                'A' | 'C' | 'c' | 'F' | 'f' | 'H' | 'h' | 'Q' | 'q' | 'T' | 't' | 'W' | 'w' => {
                    // up to and with the next ;
                    let mut j = i + 2;
                    while j < cs.len() && cs[j] != ';' {
                        j += 1;
                    }
                    i = j + 1;
                }
                'S' => {
                    // stacked: a^b, a/b, a#b → a/b
                    let mut j = i + 2;
                    let mut part = String::new();
                    while j < cs.len() && cs[j] != ';' {
                        part.push(match cs[j] {
                            '^' | '#' => '/',
                            x => x,
                        });
                        j += 1;
                    }
                    out.push_str(&part);
                    i = j + 1;
                }
                'L' | 'l' | 'O' | 'o' | 'K' | 'k' => i += 2,
                '~' => {
                    out.push(' ');
                    i += 2;
                }
                '\\' | '{' | '}' => {
                    out.push(k);
                    i += 2;
                }
                _ => {
                    out.push(c);
                    i += 1;
                }
            }
        } else {
            if c != '{' && c != '}' {
                out.push(c);
            }
            i += 1;
        }
    }
    symbols(&out)
}

/// `%%c` `%%d` `%%p` (diameter, degree, plus-minus); `%%u` `%%o` dropped.
fn symbols(s: &str) -> String {
    let mut out = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '%' && i + 2 < cs.len() && cs[i + 1] == '%' {
            match cs[i + 2].to_ascii_lowercase() {
                'c' => out.push('Ø'),
                'd' => out.push('°'),
                'p' => out.push('±'),
                'u' | 'o' => {}
                x => {
                    out.push_str("%%");
                    out.push(x);
                }
            }
            i += 3;
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    out
}

/// `\U+XXXX` letters and `^J`-style control carets.
fn decode_u(s: &str) -> String {
    let mut out = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '\\'
            && i + 6 < cs.len() + 1
            && cs.get(i + 1) == Some(&'U')
            && cs.get(i + 2) == Some(&'+')
        {
            let hex: String = cs[i + 3..(i + 7).min(cs.len())].iter().collect();
            if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                out.push(ch);
                i += 7;
                continue;
            }
        }
        if cs[i] == '^' && i + 1 < cs.len() {
            let n = cs[i + 1];
            if n == ' ' {
                out.push('^');
            } else if n.is_ascii_uppercase() || "@[\\]^_".contains(n) {
                out.push(' ');
            } else {
                out.push('^');
                out.push(n);
            }
            i += 2;
            continue;
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

/// The points of the arc a polyline's bulge makes from `a` to `b`, without `a`.
fn bulge_pts(a: DVec2, b: DVec2, bulge: f64) -> Vec<DVec2> {
    let th = 4.0 * bulge.atan();
    let d = a.distance(b);
    if bulge.abs() < 1e-9 || d < 1e-12 {
        return vec![b];
    }
    let r = d / (2.0 * (th / 2.0).sin());
    let m = (a + b) / 2.0;
    let h = r * (th / 2.0).cos();
    let u = (b - a) / d;
    let c = DVec2::new(m.x - u.y * h, m.y + u.x * h);
    let a0 = (a.y - c.y).atan2(a.x - c.x);
    let seg = ((th.abs() / (std::f64::consts::PI / 18.0)).ceil() as usize).max(2);
    let rr = r.abs();
    let mut out: Vec<DVec2> = (1..=seg)
        .map(|i| {
            let t = a0 + th * i as f64 / seg as f64;
            c + DVec2::new(t.cos(), t.sin()) * rr
        })
        .collect();
    *out.last_mut().unwrap() = b;
    out
}

fn arc_pts(c: DVec2, r: f64, a0: f64, a1: f64) -> Vec<DVec2> {
    let tau = std::f64::consts::TAU;
    let mut sweep = a1 - a0;
    while sweep <= 0.0 {
        sweep += tau;
    }
    let seg = ((sweep / (std::f64::consts::PI / 18.0)).ceil() as usize).max(4);
    (0..=seg)
        .map(|i| {
            let t = a0 + sweep * i as f64 / seg as f64;
            c + DVec2::new(t.cos(), t.sin()) * r
        })
        .collect()
}

/// An affine map: x' = a x + c y + e, y' = b x + d y + f.
type M = [f64; 6];

fn apply(m: &M, p: DVec2) -> DVec2 {
    DVec2::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

fn mul(m: &M, k: &M) -> M {
    [
        m[0] * k[0] + m[2] * k[1],
        m[1] * k[0] + m[3] * k[1],
        m[0] * k[2] + m[2] * k[3],
        m[1] * k[2] + m[3] * k[3],
        m[0] * k[4] + m[2] * k[5] + m[4],
        m[1] * k[4] + m[3] * k[5] + m[5],
    ]
}

/// An AutoCAD colour index as 0xRRGGBB.
pub fn aci_rgb(i: i64) -> u32 {
    match i {
        1 => return 0xff0000,
        2 => return 0xffff00,
        3 => return 0x00ff00,
        4 => return 0x00ffff,
        5 => return 0x0000ff,
        6 => return 0xff00ff,
        7 => return 0xffffff,
        8 => return 0x808080,
        9 => return 0xc0c0c0,
        250 => return 0x333333,
        251 => return 0x505050,
        252 => return 0x696969,
        253 => return 0x828282,
        254 => return 0xbebebe,
        255 => return 0xffffff,
        _ => {}
    }
    if !(10..=249).contains(&i) {
        return 0xffffff;
    }
    let hue = ((i - 10) / 10) as f64 * 15.0;
    let k = ((i - 10) % 10) as usize;
    let val = [1.0, 1.0, 0.65, 0.65, 0.5, 0.5, 0.3, 0.3, 0.15, 0.15][k];
    let sat = if k % 2 == 1 { 0.5 } else { 1.0 };
    let c = val * sat;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = val - c;
    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let q = |v: f64| ((v + m) * 255.0).round() as u32;
    (q(r) << 16) | (q(g) << 8) | q(b)
}

/// A hatch: its boundary rings, and its pattern's lines.
#[derive(Clone, Debug, Default)]
pub struct Hatch {
    pub solid: bool,
    pub rings: Vec<Vec<DVec2>>,
    pub lines: Vec<HatchLine>,
}

#[derive(Clone, Debug, Default)]
pub struct HatchLine {
    pub angle: f64,
    pub base: DVec2,
    pub off: DVec2,
    pub dash: Vec<f64>,
}

fn read_hatch(e: &Ent) -> Hatch {
    let ps = &e.pairs;
    let f = |k: usize| ps[k].1.trim().parse::<f64>().unwrap_or(0.0);
    let int = |k: usize| ps[k].1.trim().parse::<i64>().unwrap_or(0);
    let solid = ps
        .iter()
        .find(|q| q.0 == 70)
        .is_some_and(|x| x.1.trim().parse::<i64>().unwrap_or(0) & 1 == 1)
        || ps
            .iter()
            .find(|q| q.0 == 2)
            .is_some_and(|x| x.1.trim().eq_ignore_ascii_case("SOLID"));
    let mut h = Hatch {
        solid,
        ..Default::default()
    };
    let mut i = 0;
    while i < ps.len() && ps[i].0 != 91 {
        i += 1;
    }
    if i >= ps.len() {
        return h;
    }
    let n_paths = int(i);
    i += 1;
    let next = |i: &mut usize, code: i32| {
        while *i < ps.len() && ps[*i].0 != code {
            *i += 1;
        }
        *i < ps.len()
    };
    for _ in 0..n_paths {
        if !next(&mut i, 92) {
            break;
        }
        let kind = int(i);
        i += 1;
        let mut ring: Vec<DVec2> = vec![];
        if kind & 2 != 0 {
            let mut has_bulge = false;
            let mut n = 0usize;
            while i < ps.len() && ps[i].0 != 10 {
                if ps[i].0 == 72 {
                    has_bulge = ps[i].1.trim() != "0";
                }
                if ps[i].0 == 93 {
                    n = int(i).max(0) as usize;
                }
                i += 1;
            }
            let mut vs: Vec<(DVec2, f64)> = vec![];
            while i < ps.len() {
                let c = ps[i].0;
                if c == 10 {
                    if vs.len() == n {
                        break;
                    }
                    vs.push((DVec2::new(f(i), 0.0), 0.0));
                } else if c == 20 && !vs.is_empty() {
                    vs.last_mut().unwrap().0.y = f(i);
                } else if c == 42 && has_bulge && !vs.is_empty() {
                    vs.last_mut().unwrap().1 = f(i);
                } else {
                    break;
                }
                i += 1;
            }
            if !vs.is_empty() {
                ring.push(vs[0].0);
                for k in 0..vs.len() {
                    ring.extend(bulge_pts(vs[k].0, vs[(k + 1) % vs.len()].0, vs[k].1));
                }
                ring.pop();
            }
        } else {
            if !next(&mut i, 93) {
                break;
            }
            let n_edges = int(i);
            i += 1;
            for _ in 0..n_edges {
                if !next(&mut i, 72) {
                    break;
                }
                let et = int(i);
                i += 1;
                let mut g: std::collections::HashMap<i32, Vec<f64>> = Default::default();
                while i < ps.len() && ![72, 97, 92].contains(&ps[i].0) {
                    let c = ps[i].0;
                    g.entry(c).or_default().push(f(i));
                    if et != 4 && c == 73 && et != 1 {
                        i += 1;
                        break;
                    }
                    if et == 1 && c == 21 {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let v = |c: i32| g.get(&c).and_then(|x| x.first().copied()).unwrap_or(0.0);
                match et {
                    1 => {
                        ring.push(DVec2::new(v(10), v(20)));
                        ring.push(DVec2::new(v(11), v(21)));
                    }
                    2 | 3 => {
                        let (cx, cy) = (v(10), v(20));
                        let (mut a0, mut a1) = (v(50), v(51));
                        let ccw = g.get(&73).and_then(|x| x.first().copied()).unwrap_or(1.0) != 0.0;
                        if !ccw {
                            a0 = -a0;
                            a1 = -a1;
                        }
                        while a1 < a0 {
                            a1 += 360.0;
                        }
                        let n = (((a1 - a0) / 10.0).ceil() as usize).max(2);
                        if et == 2 {
                            let r = v(40);
                            for k in 0..=n {
                                let a = (a0 + (a1 - a0) * k as f64 / n as f64).to_radians();
                                ring.push(DVec2::new(cx + r * a.cos(), cy + r * a.sin()));
                            }
                        } else {
                            let (mx, my, ratio) = (v(11), v(21), v(40));
                            let rr = mx.hypot(my);
                            let rot = my.atan2(mx);
                            for k in 0..=n {
                                let a = (a0 + (a1 - a0) * k as f64 / n as f64).to_radians();
                                let (x, y) = (rr * a.cos(), rr * ratio * a.sin());
                                ring.push(DVec2::new(
                                    cx + x * rot.cos() - y * rot.sin(),
                                    cy + x * rot.sin() + y * rot.cos(),
                                ));
                            }
                        }
                    }
                    4 => {
                        let xs = g.get(&10).cloned().unwrap_or_default();
                        let ys = g.get(&20).cloned().unwrap_or_default();
                        for (x, y) in xs.into_iter().zip(ys) {
                            ring.push(DVec2::new(x, y));
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut clean: Vec<DVec2> = vec![];
        for (k, q) in ring.iter().enumerate() {
            let prev = ring[(k + ring.len() - 1) % ring.len()];
            if k == 0 || q.distance(prev) > 1e-9 {
                clean.push(*q);
            }
        }
        if clean.len() >= 3 {
            h.rings.push(clean);
        }
        if i < ps.len() && ps[i].0 == 97 {
            let n_src = int(i);
            i += 1;
            let mut k = 0;
            while k < n_src && i < ps.len() && ps[i].0 == 330 {
                i += 1;
                k += 1;
            }
        }
    }
    if !solid && next(&mut i, 78) {
        let n_lines = int(i);
        i += 1;
        for _ in 0..n_lines {
            if !next(&mut i, 53) {
                break;
            }
            let mut ln = HatchLine {
                angle: f(i),
                ..Default::default()
            };
            i += 1;
            while i < ps.len() {
                match ps[i].0 {
                    43 => ln.base.x = f(i),
                    44 => ln.base.y = f(i),
                    45 => ln.off.x = f(i),
                    46 => ln.off.y = f(i),
                    49 => ln.dash.push(f(i)),
                    79 => {}
                    _ => break,
                }
                i += 1;
            }
            h.lines.push(ln);
        }
    }
    h
}

/// A hatch pattern's lines cut to its boundary (even-odd), with their
/// dashes; at most `max` segments, none for a pattern too dense to draw.
pub fn hatch_pattern(h: &Hatch, max: usize) -> Vec<(DVec2, DVec2)> {
    let mut out = vec![];
    let mut edges: Vec<(DVec2, DVec2)> = vec![];
    for r in &h.rings {
        for k in 0..r.len() {
            edges.push((r[k], r[(k + 1) % r.len()]));
        }
    }
    if edges.is_empty() {
        return out;
    }
    let mut lo = DVec2::splat(f64::INFINITY);
    let mut hi = DVec2::splat(f64::NEG_INFINITY);
    for (a, b) in &edges {
        lo = lo.min(*a).min(*b);
        hi = hi.max(*a).max(*b);
    }
    let corners = [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)];
    for ln in &h.lines {
        let a = ln.angle.to_radians();
        let d = DVec2::new(a.cos(), a.sin());
        let n = d.perp();
        let step = ln.off.dot(n);
        if step.abs() < 1e-9 {
            continue;
        }
        let shift = ln.off.dot(d);
        let pn: Vec<f64> = corners.iter().map(|c| (*c - ln.base).dot(n)).collect();
        let mn = pn.iter().copied().fold(f64::INFINITY, f64::min);
        let mx = pn.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let (mut k0, mut k1) = ((mn / step).ceil() as i64, (mx / step).floor() as i64);
        if k0 > k1 {
            std::mem::swap(&mut k0, &mut k1);
        }
        if k1 - k0 > 4000 {
            return out;
        }
        let pat = &ln.dash;
        let len: f64 = pat.iter().map(|x| x.abs()).sum();
        for k in k0..=k1 {
            let o = ln.base + ln.off * k as f64;
            let mut ts: Vec<f64> = vec![];
            for (p, q) in &edges {
                let ep = *q - *p;
                let den = d.x * ep.y - d.y * ep.x;
                if den.abs() < 1e-12 {
                    continue;
                }
                let w = *p - o;
                let s = (w.x * ep.y - w.y * ep.x) / den;
                let u = (w.x * d.y - w.y * d.x) / den;
                if (0.0..1.0).contains(&u) {
                    ts.push(s);
                }
            }
            ts.sort_by(f64::total_cmp);
            let mut j = 0;
            while j + 1 < ts.len() {
                let (t0, t1) = (ts[j], ts[j + 1]);
                if pat.is_empty() || len < 1e-9 {
                    out.push((o + d * t0, o + d * t1));
                } else {
                    let ph = shift * k as f64;
                    let mut t = ((t0 - ph) / len).floor() * len + ph;
                    'dash: while t < t1 && out.len() < max {
                        for &x in pat {
                            let l = x.abs();
                            let (a0, a1) = (t.max(t0), (t + l).min(t1));
                            if x > 0.0 && a1 > a0 {
                                out.push((o + d * a0, o + d * a1));
                            }
                            t += l;
                            if t >= t1 {
                                break 'dash;
                            }
                        }
                    }
                }
                if out.len() >= max {
                    return out;
                }
                j += 2;
            }
        }
    }
    out
}

/// A DXF file's text from its bytes: UTF-8 from AutoCAD 2007 (AC1021) on,
/// else by its `$DWGCODEPAGE` (ANSI_1250 for Slovak and Czech drawings).
pub fn decode(bytes: &[u8]) -> String {
    let head: String = bytes[..bytes.len().min(8192)]
        .iter()
        .map(|&b| b as char)
        .collect();
    let after = |key: &str, code: &str| -> Option<String> {
        let i = head.find(key)?;
        let mut lines = head[i + key.len()..].lines().map(str::trim);
        while let Some(l) = lines.next() {
            if l == code {
                return lines.next().map(str::to_string);
            }
            if !l.is_empty() && l.parse::<i32>().is_err() {
                return None;
            }
        }
        None
    };
    let ver = after("$ACADVER", "1")
        .and_then(|v| v.strip_prefix("AC").and_then(|n| n.parse::<u32>().ok()));
    if ver.is_some_and(|v| v >= 1021) {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let cp = after("$DWGCODEPAGE", "3")
        .map(|s| s.to_ascii_uppercase())
        .unwrap_or_default();
    let enc = match cp.as_str() {
        "ANSI_1250" => encoding_rs::WINDOWS_1250,
        "ANSI_1251" => encoding_rs::WINDOWS_1251,
        "ANSI_1253" => encoding_rs::WINDOWS_1253,
        "ANSI_1254" => encoding_rs::WINDOWS_1254,
        "ANSI_1257" => encoding_rs::WINDOWS_1257,
        _ => encoding_rs::WINDOWS_1252,
    };
    enc.decode(bytes).0.into_owned()
}

/// Read a DXF drawing; `unit` overrides its units (metres per unit),
/// else `$INSUNITS`, else millimetres, as building drawings are.
pub fn read(text: &str, unit: Option<f64>) -> DxfImport {
    let pairs = pairs_of(text);
    let mut insunits = 0;
    let mut blocks: std::collections::HashMap<String, (DVec2, Vec<Ent>)> = Default::default();
    let mut model: Vec<Ent> = vec![];
    let mut ltype_of_layer: std::collections::HashMap<String, String> = Default::default();
    let mut color_of_layer: std::collections::HashMap<String, i64> = Default::default();
    let mut section = String::new();
    let mut cur: Option<Ent> = None;
    let mut block: Option<(String, DVec2, Vec<Ent>)> = None;

    let mut flush =
        |cur: &mut Option<Ent>, section: &str, block: &mut Option<(String, DVec2, Vec<Ent>)>| {
            let Some(e) = cur.take() else { return };
            match section {
                "ENTITIES" => model.push(e),
                "BLOCKS" => {
                    if e.kind == "BLOCK" {
                        *block = Some((
                            e.str(2, ""),
                            DVec2::new(e.num(10, 0.0), e.num(20, 0.0)),
                            vec![],
                        ));
                    } else if e.kind == "ENDBLK" {
                        if let Some((n, b, ents)) = block.take() {
                            blocks.insert(n, (b, ents));
                        }
                    } else if let Some(b) = block.as_mut() {
                        b.2.push(e);
                    }
                }
                "TABLES" if e.kind == "LAYER" => {
                    ltype_of_layer.insert(e.str(2, ""), e.str(6, "Continuous"));
                    color_of_layer.insert(e.str(2, ""), e.int(62, 7).abs());
                }
                _ => {}
            }
        };
    let mut i = 0;
    while i < pairs.len() {
        let (c, v) = &pairs[i];
        if *c == 0 {
            flush(&mut cur, &section, &mut block);
            let t = v.trim();
            if t == "SECTION" {
                section = match pairs.get(i + 1) {
                    Some((2, n)) => n.trim().to_string(),
                    _ => String::new(),
                };
                i += 2;
                continue;
            }
            if t == "ENDSEC" {
                section.clear();
                i += 1;
                continue;
            }
            if t == "EOF" {
                break;
            }
            cur = Some(Ent {
                kind: t.to_string(),
                pairs: vec![],
            });
            i += 1;
            continue;
        }
        if section == "HEADER" && *c == 9 && v.trim() == "$INSUNITS" {
            insunits = pairs
                .get(i + 1)
                .and_then(|p| p.1.trim().parse::<i64>().ok())
                .unwrap_or(0);
            i += 2;
            continue;
        }
        if let Some(e) = cur.as_mut() {
            e.pairs.push((*c, v.clone()));
        }
        i += 1;
    }
    flush(&mut cur, &section, &mut block);

    let unit = unit.or_else(|| unit_of(insunits)).unwrap_or(0.001);
    let mut r = Reader {
        out: DxfImport {
            unit,
            ..Default::default()
        },
        blocks: &blocks,
        ltype_of_layer: &ltype_of_layer,
        color_of_layer: &color_of_layer,
        unit,
    };
    r.emit(&model, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 0, None, (1.0, 0.0));
    let mut out = r.out;
    let mut bb: Option<[DVec2; 2]> = None;
    let mut grow = |p: DVec2| {
        bb = Some(match bb {
            Some([lo, hi]) => [lo.min(p), hi.max(p)],
            None => [p, p],
        });
    };
    out.polys.iter().flat_map(|p| &p.pts).for_each(|p| grow(*p));
    out.texts.iter().for_each(|t| grow(t.p));
    out.points.iter().for_each(|m| grow(m.p));
    out.faces
        .iter()
        .flat_map(|f| &f.pts)
        .for_each(|q| grow(q.truncate()));
    out.bounds = bb;
    out
}

struct Reader<'a> {
    out: DxfImport,
    blocks: &'a std::collections::HashMap<String, (DVec2, Vec<Ent>)>,
    ltype_of_layer: &'a std::collections::HashMap<String, String>,
    color_of_layer: &'a std::collections::HashMap<String, i64>,
    unit: f64,
}

/// Most faces a drawing gives.
const MAX_FACES: usize = 400_000;

impl Reader<'_> {
    fn layer(&mut self, l: &str) {
        if !self.out.layers.iter().any(|x| x == l) {
            self.out.layers.push(l.to_string());
        }
    }

    fn color_of(&self, e: &Ent, layer: &str) -> u32 {
        if let Some(tc) = e.pairs.iter().find(|x| x.0 == 420) {
            return tc.1.trim().parse::<i64>().unwrap_or(0) as u32 & 0xffffff;
        }
        let c = e.int(62, 256);
        aci_rgb(if c == 256 || c == 0 {
            *self.color_of_layer.get(layer).unwrap_or(&7)
        } else {
            c.abs()
        })
    }

    fn dashed(&self, e: &Ent, layer: &str) -> bool {
        let own = e.str(6, "");
        let lt = if own.is_empty() {
            self.ltype_of_layer
                .get(layer)
                .cloned()
                .unwrap_or_else(|| "Continuous".into())
        } else {
            own
        }
        .to_uppercase();
        !matches!(lt.as_str(), "CONTINUOUS" | "BYLAYER" | "BYBLOCK" | "")
    }

    fn emit(&mut self, ents: &[Ent], m: &M, depth: usize, parent: Option<&str>, zt: (f64, f64)) {
        if depth > 8 {
            return;
        }
        let u = self.unit;
        let mut i = 0;
        while i < ents.len() {
            let e = &ents[i];
            let mut layer = e.str(8, "0");
            if layer == "0" {
                if let Some(p) = parent {
                    layer = p.to_string();
                }
            }
            let p = |x: f64, y: f64| apply(m, DVec2::new(x, y));
            let zz = |z: f64| (zt.1 + zt.0 * z) * u;
            let dashed = self.dashed(e, &layer);
            match e.kind.as_str() {
                "LINE" => self.push(
                    &layer,
                    dashed,
                    vec![
                        p(e.num(10, 0.0), e.num(20, 0.0)),
                        p(e.num(11, 0.0), e.num(21, 0.0)),
                    ],
                    false,
                    Some(vec![zz(e.num(30, 0.0)), zz(e.num(31, 0.0))]),
                ),
                "POINT" => self.mark(
                    &layer,
                    p(e.num(10, 0.0), e.num(20, 0.0)),
                    zz(e.num(30, 0.0)),
                ),
                "3DFACE" => {
                    let q: Vec<DVec3> = [10, 11, 12, 13]
                        .iter()
                        .map(|&c| {
                            let w = p(e.num(c, 0.0), e.num(c + 10, 0.0));
                            DVec3::new(w.x, w.y, e.num(c + 20, 0.0))
                        })
                        .collect();
                    let color = self.color_of(e, &layer);
                    self.face(&layer, color, q, zt);
                }
                "LWPOLYLINE" => {
                    let mut vs: Vec<(DVec2, f64)> = vec![];
                    for (c, v) in &e.pairs {
                        let f = v.trim().parse::<f64>().unwrap_or(0.0);
                        match c {
                            10 => vs.push((DVec2::new(f, 0.0), 0.0)),
                            20 if !vs.is_empty() => vs.last_mut().unwrap().0.y = f,
                            42 if !vs.is_empty() => vs.last_mut().unwrap().1 = f,
                            _ => {}
                        }
                    }
                    if !vs.is_empty() {
                        let closed = e.int(70, 0) & 1 == 1;
                        let pts = bulged(&vs, closed);
                        let z = e.num(38, 0.0);
                        self.push(
                            &layer,
                            dashed,
                            pts.into_iter().map(|q| p(q.x, q.y)).collect(),
                            closed,
                            (z != 0.0).then(|| vec![zz(z)]),
                        );
                    }
                }
                "POLYLINE" => {
                    let mut vx: Vec<&Ent> = vec![];
                    let mut j = i + 1;
                    while j < ents.len() && ents[j].kind == "VERTEX" {
                        vx.push(&ents[j]);
                        j += 1;
                    }
                    i = if ents.get(j).is_some_and(|x| x.kind == "SEQEND") {
                        j
                    } else {
                        j - 1
                    };
                    let fl = e.int(70, 0);
                    let closed = fl & 1 == 1;
                    let v3 = |v: &Ent| DVec3::new(v.num(10, 0.0), v.num(20, 0.0), v.num(30, 0.0));
                    let color = self.color_of(e, &layer);
                    let to = |q: DVec3| {
                        let w = p(q.x, q.y);
                        DVec3::new(w.x, w.y, q.z)
                    };
                    if fl & 64 != 0 {
                        let verts: Vec<DVec3> = vx
                            .iter()
                            .filter(|v| v.int(70, 0) & 64 == 64)
                            .map(|v| v3(v))
                            .collect();
                        for v in &vx {
                            if v.int(70, 0) & (64 | 128) == 128 {
                                let ids: Vec<usize> = [71, 72, 73, 74]
                                    .iter()
                                    .map(|&c| v.int(c, 0).unsigned_abs() as usize)
                                    .filter(|&k| k > 0 && k <= verts.len())
                                    .collect();
                                let q = ids.iter().map(|&k| to(verts[k - 1])).collect();
                                self.face(&layer, color, q, zt);
                            }
                        }
                    } else if fl & 16 != 0 {
                        let (mm, nn) = (e.int(71, 0) as usize, e.int(72, 0) as usize);
                        let vs: Vec<DVec3> = vx.iter().map(|v| v3(v)).collect();
                        if mm >= 2 && nn >= 2 && vs.len() >= mm * nn {
                            let (cm, cn) = (fl & 1 == 1, fl & 32 == 32);
                            for a in 0..(if cm { mm } else { mm - 1 }) {
                                for b in 0..(if cn { nn } else { nn - 1 }) {
                                    let (a1, b1) = ((a + 1) % mm, (b + 1) % nn);
                                    let q = [a * nn + b, a * nn + b1, a1 * nn + b1, a1 * nn + b]
                                        .iter()
                                        .map(|&k| to(vs[k]))
                                        .collect();
                                    self.face(&layer, color, q, zt);
                                }
                            }
                        }
                    } else if !vx.is_empty() {
                        if fl & 8 != 0 {
                            // a 3D polyline: a height at each vertex, no arcs
                            let pts = vx
                                .iter()
                                .map(|v| p(v.num(10, 0.0), v.num(20, 0.0)))
                                .collect();
                            let zs = vx.iter().map(|v| zz(v.num(30, 0.0))).collect();
                            self.push(&layer, dashed, pts, closed, Some(zs));
                        } else {
                            let vs: Vec<(DVec2, f64)> = vx
                                .iter()
                                .map(|v| {
                                    (DVec2::new(v.num(10, 0.0), v.num(20, 0.0)), v.num(42, 0.0))
                                })
                                .collect();
                            let pts = bulged(&vs, closed);
                            let z = e.num(30, 0.0);
                            self.push(
                                &layer,
                                dashed,
                                pts.into_iter().map(|q| p(q.x, q.y)).collect(),
                                closed,
                                (z != 0.0).then(|| vec![zz(z)]),
                            );
                        }
                    }
                }
                "CIRCLE" => {
                    let c = DVec2::new(e.num(10, 0.0), e.num(20, 0.0));
                    let r = e.num(40, 0.0);
                    let z = e.num(30, 0.0);
                    if r * u < 1.5 {
                        // a survey mark
                        self.mark(&layer, p(c.x, c.y), zz(z));
                    }
                    let mut pts = arc_pts(c, r, 0.0, std::f64::consts::TAU);
                    pts.pop();
                    self.push(
                        &layer,
                        dashed,
                        pts.into_iter().map(|q| p(q.x, q.y)).collect(),
                        true,
                        (z != 0.0).then(|| vec![zz(z)]),
                    );
                }
                "ARC" => {
                    let c = DVec2::new(e.num(10, 0.0), e.num(20, 0.0));
                    let pts = arc_pts(
                        c,
                        e.num(40, 0.0),
                        e.num(50, 0.0).to_radians(),
                        e.num(51, 0.0).to_radians(),
                    );
                    let z = e.num(30, 0.0);
                    self.push(
                        &layer,
                        dashed,
                        pts.into_iter().map(|q| p(q.x, q.y)).collect(),
                        false,
                        (z != 0.0).then(|| vec![zz(z)]),
                    );
                }
                "ELLIPSE" => {
                    let c = DVec2::new(e.num(10, 0.0), e.num(20, 0.0));
                    let mj = DVec2::new(e.num(11, 0.0), e.num(21, 0.0));
                    let ratio = e.num(40, 1.0);
                    let t0 = e.num(41, 0.0);
                    let mut t1 = e.num(42, std::f64::consts::TAU);
                    if t1 <= t0 {
                        t1 += std::f64::consts::TAU;
                    }
                    let mn = DVec2::new(-mj.y * ratio, mj.x * ratio);
                    let seg = (((t1 - t0) / (std::f64::consts::PI / 24.0)).ceil() as usize).max(8);
                    let mut pts: Vec<DVec2> = (0..=seg)
                        .map(|k| {
                            let t = t0 + (t1 - t0) * k as f64 / seg as f64;
                            let q = c + mj * t.cos() + mn * t.sin();
                            p(q.x, q.y)
                        })
                        .collect();
                    let full = (t1 - t0 - std::f64::consts::TAU).abs() < 1e-6;
                    if full {
                        pts.pop();
                    }
                    self.push(&layer, dashed, pts, full, None);
                }
                "SPLINE" => {
                    let (mut fit, mut ctl): (Vec<DVec2>, Vec<DVec2>) = (vec![], vec![]);
                    let mut mode = ' ';
                    for (c, v) in &e.pairs {
                        let f = v.trim().parse::<f64>().unwrap_or(0.0);
                        match c {
                            10 => {
                                ctl.push(DVec2::new(f, 0.0));
                                mode = 'c';
                            }
                            11 => {
                                fit.push(DVec2::new(f, 0.0));
                                mode = 'f';
                            }
                            20 if mode == 'c' => ctl.last_mut().unwrap().y = f,
                            21 if mode == 'f' => fit.last_mut().unwrap().y = f,
                            _ => {}
                        }
                    }
                    let pts = if fit.len() >= 2 { fit } else { ctl };
                    self.push(
                        &layer,
                        dashed,
                        pts.into_iter().map(|q| p(q.x, q.y)).collect(),
                        e.int(70, 0) & 1 == 1,
                        None,
                    );
                }
                "TEXT" | "MTEXT" | "ATTRIB" => {
                    let mtext = e.kind == "MTEXT";
                    let raw = if mtext {
                        let mut s: String = e
                            .pairs
                            .iter()
                            .filter(|q| q.0 == 3)
                            .map(|q| q.1.as_str())
                            .collect();
                        if let Some(q) = e.pairs.iter().find(|q| q.0 == 1) {
                            s.push_str(&q.1);
                        }
                        s
                    } else {
                        e.str(1, "")
                    };
                    let s = decode_u(&raw);
                    let mut s = if mtext { mtext_plain(&s) } else { symbols(&s) };
                    if s.trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    let aligned = !mtext && (e.int(72, 0) != 0 || e.int(73, 0) != 0) && e.has(11);
                    let p0 = if aligned {
                        p(e.num(11, 0.0), e.num(21, 0.0))
                    } else {
                        p(e.num(10, 0.0), e.num(20, 0.0))
                    };
                    let mut ang = e.num(50, 0.0);
                    if mtext && e.has(11) {
                        ang = e.num(21, 0.0).atan2(e.num(11, 0.0)).to_degrees();
                    }
                    let sc = m[0].hypot(m[1]);
                    ang += m[1].atan2(m[0]).to_degrees();
                    let align = if mtext {
                        e.int(71, 1).clamp(1, 9) as u8
                    } else if aligned {
                        let (h72, v73) = (e.int(72, 0), e.int(73, 0));
                        let col = match h72 {
                            1 | 4 => 2,
                            2 => 3,
                            _ => 1,
                        };
                        let row = if h72 == 4 {
                            2
                        } else {
                            match v73 {
                                3 => 1,
                                2 => 2,
                                _ => 3,
                            }
                        };
                        ((row - 1) * 3 + col) as u8
                    } else {
                        7
                    };
                    let wf = if mtext {
                        1.0
                    } else {
                        let w = e.num(41, 1.0);
                        if w == 0.0 {
                            1.0
                        } else {
                            w
                        }
                    };
                    let hh = e.num(40, 2.5);
                    if mtext && e.num(41, 0.0) > 0.0 {
                        s = wrap_lines(&s, e.num(41, 0.0), hh);
                    }
                    self.layer(&layer);
                    self.out.texts.push(Text {
                        p: p0 * u,
                        text: s,
                        h: hh * sc * u,
                        angle: ang,
                        layer: layer.clone(),
                        align,
                        wf,
                    });
                }
                "HATCH" => {
                    let h = read_hatch(e);
                    if !h.rings.is_empty() {
                        self.layer(&layer);
                        if h.solid {
                            let rings = h
                                .rings
                                .iter()
                                .map(|r| r.iter().map(|q| p(q.x, q.y) * u).collect())
                                .collect();
                            let color = self.color_of(e, &layer);
                            self.out.fills.push(Fill {
                                rings,
                                layer: layer.clone(),
                                color,
                            });
                        } else {
                            for (a, b) in hatch_pattern(&h, 20_000) {
                                self.out.polys.push(Poly {
                                    pts: vec![p(a.x, a.y) * u, p(b.x, b.y) * u],
                                    closed: false,
                                    layer: layer.clone(),
                                    dashed: false,
                                    z: None,
                                });
                            }
                        }
                    }
                }
                "SOLID" | "TRACE" => {
                    let q = vec![
                        p(e.num(10, 0.0), e.num(20, 0.0)) * u,
                        p(e.num(11, 0.0), e.num(21, 0.0)) * u,
                        p(e.num(13, e.num(12, 0.0)), e.num(23, e.num(22, 0.0))) * u,
                        p(e.num(12, 0.0), e.num(22, 0.0)) * u,
                    ];
                    self.layer(&layer);
                    let color = self.color_of(e, &layer);
                    self.out.fills.push(Fill {
                        rings: vec![q],
                        layer: layer.clone(),
                        color,
                    });
                }
                "DIMENSION" => {
                    if let Some((_, ents)) = self.blocks.get(&e.str(2, "")) {
                        self.emit(ents, m, depth + 1, Some(&layer), zt);
                    }
                }
                "INSERT" => {
                    if depth < 2 {
                        self.mark(
                            &layer,
                            p(e.num(10, 0.0), e.num(20, 0.0)),
                            zz(e.num(30, 0.0)),
                        );
                    }
                    if let Some((base, ents)) = self.blocks.get(&e.str(2, "")) {
                        let (sx, sy) = (e.num(41, 1.0), e.num(42, 1.0));
                        let rot = e.num(50, 0.0).to_radians();
                        let (cs, sn) = (rot.cos(), rot.sin());
                        let cols = e.int(70, 1).max(1);
                        let rows = e.int(71, 1).max(1);
                        let (dc, dr) = (e.num(44, 0.0), e.num(45, 0.0));
                        for r in 0..rows {
                            for c in 0..cols {
                                let (ox, oy) = (c as f64 * dc, r as f64 * dr);
                                let tx = e.num(10, 0.0) + cs * ox - sn * oy;
                                let ty = e.num(20, 0.0) + sn * ox + cs * oy;
                                let mut k: M = [cs * sx, sn * sx, -sn * sy, cs * sy, 0.0, 0.0];
                                k[4] = tx - (k[0] * base.x + k[2] * base.y);
                                k[5] = ty - (k[1] * base.x + k[3] * base.y);
                                let zt2 = (zt.0 * e.num(43, 1.0), zt.1 + zt.0 * e.num(30, 0.0));
                                self.emit(ents, &mul(m, &k), depth + 1, Some(&layer), zt2);
                            }
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn mark(&mut self, layer: &str, q: DVec2, z: f64) {
        self.layer(layer);
        if z != 0.0 {
            self.out.has3d = true;
        }
        self.out.points.push(Mark {
            p: q * self.unit,
            layer: layer.to_string(),
            z: (z != 0.0).then_some(z),
        });
    }

    /// A polyline; `z` one height for all its points, or one for each.
    fn push(
        &mut self,
        layer: &str,
        dashed: bool,
        pts: Vec<DVec2>,
        closed: bool,
        z: Option<Vec<f64>>,
    ) {
        if pts.len() < 2 {
            return;
        }
        self.layer(layer);
        let zs = z.map(|z| {
            if z.len() == pts.len() {
                z
            } else {
                vec![z.first().copied().unwrap_or(0.0); pts.len()]
            }
        });
        let zs = zs.filter(|z| z.iter().any(|v| *v != 0.0));
        if zs.is_some() {
            self.out.has3d = true;
        }
        let u = self.unit;
        self.out.polys.push(Poly {
            pts: pts.into_iter().map(|q| q * u).collect(),
            closed,
            layer: layer.to_string(),
            dashed,
            z: zs,
        });
    }

    /// A face; `q` in plan already mapped, heights in file units before
    /// the block's.
    fn face(&mut self, layer: &str, color: u32, q: Vec<DVec3>, zt: (f64, f64)) {
        if self.out.faces.len() >= MAX_FACES {
            return;
        }
        let u = self.unit;
        let mut pts: Vec<DVec3> = vec![];
        for v in q {
            let k = DVec3::new(v.x * u, v.y * u, (zt.1 + zt.0 * v.z) * u);
            if !pts.contains(&k) {
                pts.push(k);
            }
        }
        if pts.len() < 3 {
            return;
        }
        self.layer(layer);
        self.out.has3d = true;
        self.out.faces.push(Face {
            pts,
            layer: layer.to_string(),
            color,
        });
    }
}

/// A polyline's points with its bulges' arcs.
fn bulged(vs: &[(DVec2, f64)], closed: bool) -> Vec<DVec2> {
    let mut pts = vec![vs[0].0];
    let cnt = if closed { vs.len() } else { vs.len() - 1 };
    for k in 0..cnt {
        pts.extend(bulge_pts(vs[k].0, vs[(k + 1) % vs.len()].0, vs[k].1));
    }
    if closed {
        pts.pop();
    }
    pts
}

/// Wrap a text's lines to `width`, a letter about half its height wide.
fn wrap_lines(s: &str, width: f64, h: f64) -> String {
    let per = ((width / (h * 0.5)).floor() as usize).max(1);
    s.split('\n')
        .map(|line| {
            let mut out: Vec<String> = vec![];
            let mut cur = String::new();
            for w in line.split(' ') {
                if !cur.is_empty() && cur.chars().count() + 1 + w.chars().count() > per {
                    out.push(std::mem::take(&mut cur));
                }
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(w);
            }
            out.push(cur);
            out.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dxf(header: &str, tables: &str, blocks: &str, entities: &str) -> String {
        format!(
            "0\nSECTION\n2\nHEADER\n{header}0\nENDSEC\n0\nSECTION\n2\nTABLES\n{tables}0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n{blocks}0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n"
        )
    }

    #[test]
    fn lines_texts_and_layers_come_in_metres() {
        let t = dxf(
            "9\n$INSUNITS\n70\n4\n",
            "0\nLAYER\n2\nOSI\n6\nDASHED\n62\n1\n",
            "",
            "0\nLINE\n8\nOSI\n10\n0\n20\n0\n11\n5000\n21\n0\n\
             0\nTEXT\n8\nPOPIS\n10\n100\n20\n200\n40\n250\n1\nKuchyňa %%c100\n\
             0\nLWPOLYLINE\n8\nSTENY\n70\n1\n10\n0\n20\n0\n10\n1000\n20\n0\n42\n1\n10\n1000\n20\n1000\n",
        );
        let r = read(&t, None);
        assert_eq!(r.unit, 0.001);
        assert_eq!(r.layers, vec!["OSI", "POPIS", "STENY"]);
        let line = &r.polys[0];
        assert_eq!(line.pts, vec![DVec2::ZERO, DVec2::new(5.0, 0.0)]);
        assert!(line.dashed, "dashed by its layer's line type");
        assert!(line.z.is_none());
        let tx = &r.texts[0];
        assert_eq!(tx.text, "Kuchyňa Ø100");
        assert!((tx.h - 0.25).abs() < 1e-12 && (tx.p - DVec2::new(0.1, 0.2)).length() < 1e-12);
        // a bulge of 1 is a half circle between its vertices
        let wall = &r.polys[1];
        assert!(wall.closed && wall.pts.len() > 4);
        assert!(
            wall.pts.iter().any(|p| p.x > 1.4),
            "the arc bulges out to x = 1.5"
        );
        assert!(!r.has3d && !r.is_far());
    }

    #[test]
    fn blocks_are_inserted_turned_and_scaled_on_their_layer() {
        let t = dxf(
            "9\n$INSUNITS\n70\n6\n",
            "",
            "0\nBLOCK\n2\nB\n10\n0\n20\n0\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n0\nENDBLK\n",
            "0\nINSERT\n8\nZNACKY\n2\nB\n10\n10\n20\n5\n41\n2\n42\n2\n50\n90\n",
        );
        let r = read(&t, None);
        let l = &r.polys[0];
        assert_eq!(l.layer, "ZNACKY");
        assert!((l.pts[0] - DVec2::new(10.0, 5.0)).length() < 1e-9);
        assert!(
            (l.pts[1] - DVec2::new(10.0, 7.0)).length() < 1e-9,
            "{:?}",
            l.pts
        );
        // its insertion point snaps
        assert_eq!(r.points[0].p, DVec2::new(10.0, 5.0));
    }

    #[test]
    fn a_survey_in_national_coordinates_is_far_and_3d() {
        let t = dxf(
            "9\n$INSUNITS\n70\n6\n",
            "",
            "",
            "0\nPOINT\n8\nBODY\n10\n-450123.5\n20\n-1250456.2\n30\n312.45\n\
             0\n3DFACE\n8\nTEREN\n10\n-450120\n20\n-1250450\n30\n312\n11\n-450110\n21\n-1250450\n31\n313\n12\n-450110\n22\n-1250440\n32\n314\n13\n-450110\n23\n-1250440\n33\n314\n",
        );
        let r = read(&t, None);
        assert!(r.is_far() && r.has3d);
        assert_eq!(r.points[0].z, Some(312.45));
        assert_eq!(
            r.faces[0].pts.len(),
            3,
            "a triangle: its repeated corner dropped"
        );
        assert_eq!(r.z_range(), Some((312.0, 314.0)));
        // its heights: the point and the face's three corners
        assert_eq!(r.heights(&[], None).len(), 4);
        assert_eq!(r.heights(&["TEREN".into()], None).len(), 1);
        let clip = [
            DVec2::new(-450115.0, -1250455.0),
            DVec2::new(-450100.0, -1250435.0),
        ];
        assert_eq!(r.heights(&[], Some(clip)).len(), 2);
    }

    #[test]
    fn a_hatch_gives_its_pattern_lines_or_a_fill() {
        let ring = "91\n1\n92\n2\n72\n0\n73\n1\n93\n4\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n97\n0\n";
        let solid = format!("0\nHATCH\n8\nH\n2\nSOLID\n70\n1\n{ring}");
        let lines = format!(
            "0\nHATCH\n8\nH\n2\nANSI31\n70\n0\n{ring}75\n0\n76\n1\n52\n0\n41\n1\n77\n0\n78\n1\n53\n45\n43\n0\n44\n0\n45\n-1\n46\n1\n79\n0\n"
        );
        let r = read(
            &dxf("9\n$INSUNITS\n70\n6\n", "", "", &(solid + &lines)),
            None,
        );
        assert_eq!(r.fills.len(), 1);
        assert_eq!(r.fills[0].rings[0].len(), 4);
        assert!(r.polys.len() > 5, "pattern lines: {}", r.polys.len());
        for p in &r.polys {
            for q in &p.pts {
                assert!(q.x > -1e-9 && q.x < 10.0 + 1e-9 && q.y > -1e-9 && q.y < 10.0 + 1e-9);
            }
        }
    }

    #[test]
    fn old_drawings_are_read_in_their_code_page() {
        let mut b = b"0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$DWGCODEPAGE\n3\nANSI_1250\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\n0\n10\n0\n20\n0\n40\n1\n1\n".to_vec();
        // "Šťastie" in windows-1250
        b.extend([0x8a, 0x9d, b'a', b's', b't', b'i', b'e']);
        b.extend(b"\n0\nENDSEC\n0\nEOF\n");
        let r = read(&decode(&b), Some(1.0));
        assert_eq!(r.texts[0].text, "Šťastie");
    }

    #[test]
    fn mtext_loses_its_codes() {
        assert_eq!(
            mtext_plain("{\\fArial|b1;Izba}\\P\\H2.5;1.01"),
            "Izba\n1.01"
        );
        assert_eq!(mtext_plain("\\S1^2;"), "1/2");
        assert_eq!(decode_u("\\U+010Dierna"), "čierna");
        assert_eq!(aci_rgb(1), 0xff0000);
        assert_eq!(aci_rgb(10), 0xff0000);
    }
}
