//! Point clouds: a laser scan or a survey of a building or a room, read
//! from text (XYZ, PTS, TXT, CSV, ASC: `x y z`, then intensity and
//! colour where the file has them) or from LAS 1.0 to 1.4 (not the
//! compressed LAZ), after PilonDesigner's `pointcloud.ts`. The points
//! are kept as small numbers from the cloud's own origin, so a cloud in
//! national coordinates keeps its millimetres; S-JTSK is turned to east
//! and north as [`crate::xyz`] does. A file of more points than
//! [`MOST`] is read every second, fourth… point.
//!
//! A plan shows a [`Slice`] of it: the points between two heights, one
//! per small cell of the plan, which is what walls cut at breast height
//! look like, and what the pointer snaps to.

use glam::{DVec2, DVec3};
use std::collections::HashMap;
use std::io::{BufRead, Read, Seek, SeekFrom};

/// Most points kept of a file.
pub const MOST: usize = 6_000_000;

/// The files a cloud is read from.
pub const FILES: &[&str] = &["xyz", "pts", "txt", "csv", "asc", "las"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cloud {
    /// What the points are counted from.
    pub origin: DVec3,
    pub points: Vec<[f32; 3]>,
    /// A colour per point; empty when the file has none.
    pub rgb: Vec<[u8; 3]>,
    /// The file's span (all its points').
    pub lo: DVec3,
    pub hi: DVec3,
    /// The points the file holds.
    pub total: usize,
    /// It was in S-JTSK.
    pub jtsk: bool,
}

struct Collector {
    cloud: Cloud,
    origin: Option<DVec3>,
    colours: bool,
    stride: usize,
    most: usize,
}

impl Collector {
    fn new(most: usize) -> Self {
        Self {
            cloud: Cloud {
                lo: DVec3::INFINITY,
                hi: DVec3::NEG_INFINITY,
                ..Default::default()
            },
            origin: None,
            colours: false,
            stride: 1,
            most: most.max(2),
        }
    }

    fn push(&mut self, mut p: DVec3, colour: Option<[u8; 3]>) {
        if !p.is_finite() {
            return;
        }
        if crate::xyz::is_jtsk(p.x, p.y) {
            self.cloud.jtsk = true;
            if p.x > 0.0 {
                (p.x, p.y) = (-p.x, -p.y);
            }
        }
        let c = &mut self.cloud;
        c.lo = c.lo.min(p);
        c.hi = c.hi.max(p);
        c.total += 1;
        if !(c.total - 1).is_multiple_of(self.stride) {
            return;
        }
        let o = *self.origin.get_or_insert(p.floor());
        let q = p - o;
        c.points.push([q.x as f32, q.y as f32, q.z as f32]);
        self.colours |= colour.is_some();
        c.rgb.push(colour.unwrap_or([110; 3]));
        if c.points.len() > self.most {
            // every second one of those kept, and of those to come
            let mut n = 0;
            c.points.retain(|_| {
                n += 1;
                n % 2 == 1
            });
            n = 0;
            c.rgb.retain(|_| {
                n += 1;
                n % 2 == 1
            });
            self.stride *= 2;
        }
    }

    fn done(mut self) -> Cloud {
        self.cloud.origin = self.origin.unwrap_or_default();
        if !self.colours {
            self.cloud.rgb = vec![];
        }
        self.cloud
    }
}

/// A colour's part as a file gives it: 0 to 255, or 16 bits.
fn part(v: f64) -> u8 {
    if v > 255.0 {
        (v / 256.0).min(255.0) as u8
    } else {
        v.max(0.0) as u8
    }
}

/// A cloud from a text of points: `x y z`, `x y z r g b` or
/// `x y z intensity r g b` a line (a leading point number is skipped as
/// in [`crate::xyz::read`]); at most about `most` points.
pub fn read_text(r: impl BufRead, most: usize) -> Cloud {
    let mut c = Collector::new(most);
    let mut r = r;
    let mut raw = vec![];
    let mut nums: Vec<f64> = Vec::with_capacity(8);
    loop {
        raw.clear();
        match r.read_until(b'\n', &mut raw) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = String::from_utf8_lossy(&raw);
        nums.clear();
        nums.extend(
            line.split(|ch: char| ch.is_whitespace() || ch == ';' || ch == ',')
                .filter(|t| !t.is_empty())
                .map_while(|t| t.parse::<f64>().ok()),
        );
        if nums.len() < 3 {
            continue;
        }
        let k = usize::from(
            nums.len() >= 4
                && nums[0].fract() == 0.0
                && nums[0].abs() < 1e6
                && crate::xyz::is_jtsk(nums[1], nums[2])
                && !crate::xyz::is_jtsk(nums[0], nums[1]),
        );
        let rest = &nums[k + 3..];
        let colour = match rest.len() {
            3 => Some(&rest[0..3]),
            n if n >= 4 => Some(&rest[1..4]),
            _ => None,
        }
        .map(|v| [part(v[0]), part(v[1]), part(v[2])]);
        c.push(DVec3::new(nums[k], nums[k + 1], nums[k + 2]), colour);
    }
    c.done()
}

/// A cloud from a LAS file (1.0 to 1.4, point formats 0 to 10); at most
/// about `most` points.
pub fn read_las(mut r: impl Read + Seek, most: usize) -> Result<Cloud, String> {
    let mut h = [0u8; 375];
    let mut got = 0;
    while got < h.len() {
        match r.read(&mut h[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    if got < 227 || &h[0..4] != b"LASF" {
        return Err("not a LAS file".into());
    }
    let u16_at = |i: usize| u16::from_le_bytes([h[i], h[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
    let f64_at = |i: usize| f64::from_le_bytes(h[i..i + 8].try_into().unwrap_or_default());
    let minor = h[25];
    let data = u32_at(96) as u64;
    let format = h[104];
    if format & 0x80 != 0 {
        return Err(
            "the file is compressed (LAZ); save it as plain LAS (CloudCompare or las2las does it)"
                .into(),
        );
    }
    let format = format & 0x3f;
    let len = u16_at(105) as usize;
    let mut count = u32_at(107) as u64;
    if minor >= 4 && got >= 255 {
        let n = u64::from_le_bytes(h[247..255].try_into().unwrap_or_default());
        if n > 0 {
            count = n;
        }
    }
    let scale = DVec3::new(f64_at(131), f64_at(139), f64_at(147));
    let offset = DVec3::new(f64_at(155), f64_at(163), f64_at(171));
    // where a point's colour starts, by its format
    let rgb_at = match format {
        2 => Some(20),
        3 | 5 => Some(28),
        7 | 8 | 10 => Some(30),
        0 | 1 | 4 | 6 | 9 => None,
        f => return Err(format!("LAS point format {f} isn't read")),
    };
    if len < 12 || rgb_at.is_some_and(|at| len < at + 6) {
        return Err("the LAS file's point records are too short".into());
    }
    r.seek(SeekFrom::Start(data)).map_err(|e| e.to_string())?;
    let mut c = Collector::new(most);
    // a block of whole records at a time
    let per = (1 << 20) / len.max(1) + 1;
    let mut buf = vec![0u8; per * len];
    let mut left = count;
    // 16-bit colours, unless the file wrote 8-bit ones
    let mut wide = false;
    let mut first = true;
    while left > 0 {
        let want = (left.min(per as u64) as usize) * len;
        let mut have = 0;
        while have < want {
            match r.read(&mut buf[have..want]) {
                Ok(0) => break,
                Ok(n) => have += n,
                Err(e) => return Err(e.to_string()),
            }
        }
        let n = have / len;
        if n == 0 {
            break;
        }
        if let (true, Some(at)) = (first, rgb_at) {
            wide = buf[..n * len].chunks_exact(len).any(|p| {
                (0..3).any(|k| u16::from_le_bytes([p[at + 2 * k], p[at + 2 * k + 1]]) > 255)
            });
            first = false;
        }
        for p in buf[..n * len].chunks_exact(len) {
            let i = |k: usize| i32::from_le_bytes([p[k], p[k + 1], p[k + 2], p[k + 3]]) as f64;
            let at = DVec3::new(i(0), i(4), i(8)) * scale + offset;
            let colour = rgb_at.map(|a| {
                let v = |k: usize| {
                    let x = u16::from_le_bytes([p[a + 2 * k], p[a + 2 * k + 1]]);
                    if wide {
                        (x >> 8) as u8
                    } else {
                        x as u8
                    }
                };
                [v(0), v(1), v(2)]
            });
            c.push(at, colour);
        }
        left -= n as u64;
        if have < want {
            break;
        }
    }
    Ok(c.done())
}

/// A reader that tells how far it has read, and stops when told to.
struct Watched<'a, R> {
    inner: R,
    at: u64,
    told: u64,
    progress: &'a dyn Fn(u64) -> bool,
}

impl<R: Read> Read for Watched<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.at += n as u64;
        if self.at - self.told >= 1 << 20 || n == 0 {
            self.told = self.at;
            if !(self.progress)(self.at) {
                return Err(std::io::Error::other(CANCELLED));
            }
        }
        Ok(n)
    }
}

impl<R: Seek> Seek for Watched<'_, R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let p = self.inner.seek(pos)?;
        self.at = p;
        Ok(p)
    }
}

/// What a read stopped by its caller says.
pub const CANCELLED: &str = "cancelled";

/// The cloud of a file, by what the file is.
pub fn read_file(path: &str) -> Result<Cloud, String> {
    read_file_with(path, MOST, &|_| true)
}

/// The cloud of a file, at most about `most` points; `progress` is told
/// the bytes read so far (about every megabyte) and stops the read by
/// answering false, the read then failing with [`CANCELLED`].
pub fn read_file_with(
    path: &str,
    most: usize,
    progress: &dyn Fn(u64) -> bool,
) -> Result<Cloud, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("couldn't read {path}: {e}"))?;
    let f = Watched {
        inner: f,
        at: 0,
        told: 0,
        progress,
    };
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut r = std::io::BufReader::with_capacity(1 << 20, f);
    let mut magic = [0u8; 4];
    let las = r.read(&mut magic).is_ok_and(|n| n == 4) && &magic == b"LASF";
    r.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let c = if ext == "laz" {
        return Err(format!(
            "{path}: the file is compressed (LAZ); save it as plain LAS (CloudCompare or las2las does it)"
        ));
    } else if las || ext == "las" {
        read_las(r, most).map_err(|e| {
            if e.contains(CANCELLED) {
                CANCELLED.to_string()
            } else {
                format!("{path}: {e}")
            }
        })?
    } else {
        let stop = std::cell::Cell::new(false);
        let c = read_text(
            Stopped {
                inner: r,
                stop: &stop,
            },
            most,
        );
        if stop.get() {
            return Err(CANCELLED.into());
        }
        c
    };
    if c.points.is_empty() {
        return Err(format!("{path}: no points in it"));
    }
    Ok(c)
}

/// A text reader that notes when its read was stopped (read_text takes
/// an error for the end of the file).
struct Stopped<'a, R> {
    inner: R,
    stop: &'a std::cell::Cell<bool>,
}

impl<R: Read> Read for Stopped<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf).inspect_err(|e| {
            if e.to_string() == CANCELLED {
                self.stop.set(true);
            }
        })
    }
}

impl<R: BufRead> BufRead for Stopped<'_, R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        match self.inner.fill_buf() {
            Ok(b) => Ok(b),
            Err(e) => {
                if e.to_string() == CANCELLED {
                    self.stop.set(true);
                }
                Err(e)
            }
        }
    }
    fn consume(&mut self, n: usize) {
        self.inner.consume(n)
    }
}

/// The points of a cloud between two heights, as a plan shows them: one
/// per cell, in the cloud's file coordinates.
#[derive(Clone, Debug, Default)]
pub struct Slice {
    pub points: Vec<DVec2>,
    /// A colour per point; empty when the cloud has none.
    pub rgb: Vec<[u8; 3]>,
    /// The points by the cell of a coarse grid they lie in.
    cells: HashMap<(i32, i32), Vec<u32>>,
    lo: DVec2,
}

/// The side of a cell of a slice's search grid, metres.
const SEARCH: f64 = 0.25;

impl Slice {
    fn index(mut self) -> Self {
        self.lo = self
            .points
            .iter()
            .fold(DVec2::splat(f64::INFINITY), |a, p| a.min(*p));
        for (n, p) in self.points.iter().enumerate() {
            let k = ((*p - self.lo) / SEARCH).floor().as_ivec2();
            self.cells.entry((k.x, k.y)).or_default().push(n as u32);
        }
        self
    }

    /// The point nearest `p`, within `reach`.
    pub fn nearest(&self, p: DVec2, reach: f64) -> Option<DVec2> {
        if self.points.is_empty() || reach.is_nan() || reach <= 0.0 {
            return None;
        }
        let a = ((p - DVec2::splat(reach) - self.lo) / SEARCH)
            .floor()
            .as_ivec2();
        let b = ((p + DVec2::splat(reach) - self.lo) / SEARCH)
            .floor()
            .as_ivec2();
        // far out, a pointer's reach covers the whole cloud: look at all
        if (b.x - a.x) as i64 * (b.y - a.y) as i64 > 4096 {
            return self
                .points
                .iter()
                .map(|q| (q.distance_squared(p), *q))
                .filter(|(d, _)| *d <= reach * reach)
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .map(|(_, q)| q);
        }
        let mut best: Option<(f64, DVec2)> = None;
        for i in a.x..=b.x {
            for j in a.y..=b.y {
                for n in self.cells.get(&(i, j)).into_iter().flatten() {
                    let q = self.points[*n as usize];
                    let d = q.distance_squared(p);
                    if d <= reach * reach && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, q));
                    }
                }
            }
        }
        best.map(|(_, q)| q)
    }
}

impl Cloud {
    /// A point where the file has it.
    pub fn at(&self, n: usize) -> DVec3 {
        let p = self.points[n];
        self.origin + DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64)
    }

    /// What it spans in plan.
    pub fn bounds(&self) -> [DVec2; 2] {
        [self.lo.truncate(), self.hi.truncate()]
    }

    /// Its points between the heights `from` and `to` (the file's) and
    /// within `clip`, one per cell of a grid fine enough to keep at most
    /// `most`: 5 mm, or coarser.
    pub fn slice(&self, from: f64, to: f64, clip: Option<[DVec2; 2]>, most: usize) -> Slice {
        let (zf, zt) = ((from - self.origin.z) as f32, (to - self.origin.z) as f32);
        let o = self.origin.truncate();
        let inside = |p: &[f32; 3]| {
            p[2] >= zf
                && p[2] <= zt
                && clip.is_none_or(|[a, b]| {
                    let q = o + DVec2::new(p[0] as f64, p[1] as f64);
                    q.cmpge(a).all() && q.cmple(b).all()
                })
        };
        let mut cell = 0.005f32;
        let kept = loop {
            let mut kept: HashMap<(i32, i32), u32> = HashMap::new();
            let mut over = false;
            for (n, p) in self.points.iter().enumerate() {
                if !inside(p) {
                    continue;
                }
                let k = ((p[0] / cell).floor() as i32, (p[1] / cell).floor() as i32);
                kept.entry(k).or_insert(n as u32);
                if kept.len() > most {
                    over = true;
                    break;
                }
            }
            if !over {
                break kept;
            }
            cell *= 2.0;
        };
        let mut idx: Vec<u32> = kept.into_values().collect();
        idx.sort_unstable();
        Slice {
            points: idx
                .iter()
                .map(|n| self.at(*n as usize).truncate())
                .collect(),
            rgb: if self.rgb.is_empty() {
                vec![]
            } else {
                idx.iter().map(|n| self.rgb[*n as usize]).collect()
            },
            ..Default::default()
        }
        .index()
    }

    /// The points a section or an elevation shows: each point (in the
    /// file's coordinates) `to_view` takes to the view's drawing (along
    /// its line, height), or leaves out (behind it, past its depth), one
    /// per cell of a grid fine enough to keep at most `most`. The slice's
    /// points are the view's own, not the file's.
    pub fn section(&self, to_view: &dyn Fn(DVec3) -> Option<DVec2>, most: usize) -> Slice {
        let shown: Vec<(u32, DVec2)> = (0..self.points.len())
            .filter_map(|n| to_view(self.at(n)).map(|q| (n as u32, q)))
            .collect();
        let mut cell = 0.005f64;
        let kept = loop {
            let mut kept: HashMap<(i64, i64), (u32, DVec2)> = HashMap::new();
            let mut over = false;
            for (n, q) in &shown {
                let k = ((q.x / cell).floor() as i64, (q.y / cell).floor() as i64);
                kept.entry(k).or_insert((*n, *q));
                if kept.len() > most {
                    over = true;
                    break;
                }
            }
            if !over {
                break kept;
            }
            cell *= 2.0;
        };
        let mut kept: Vec<(u32, DVec2)> = kept.into_values().collect();
        kept.sort_unstable_by_key(|(n, _)| *n);
        Slice {
            points: kept.iter().map(|(_, q)| *q).collect(),
            rgb: if self.rgb.is_empty() {
                vec![]
            } else {
                kept.iter().map(|(n, _)| self.rgb[*n as usize]).collect()
            },
            ..Default::default()
        }
        .index()
    }

    /// The ground under it: its lowest point in each `cell` (metres) of
    /// the plan, within `clip`, where the file has it (PilonDesigner's
    /// terrain from a cloud).
    pub fn ground(&self, cell: f64, clip: Option<[DVec2; 2]>) -> Vec<DVec3> {
        let cell = cell.max(0.01) as f32;
        let mut low: HashMap<(i32, i32), u32> = HashMap::new();
        for (n, p) in self.points.iter().enumerate() {
            if clip.is_some_and(|[a, b]| {
                let q = self.origin.truncate() + DVec2::new(p[0] as f64, p[1] as f64);
                !(q.cmpge(a).all() && q.cmple(b).all())
            }) {
                continue;
            }
            let k = ((p[0] / cell).floor() as i32, (p[1] / cell).floor() as i32);
            let e = low.entry(k).or_insert(n as u32);
            if p[2] < self.points[*e as usize][2] {
                *e = n as u32;
            }
        }
        let mut idx: Vec<u32> = low.into_values().collect();
        idx.sort_unstable();
        idx.into_iter().map(|n| self.at(n as usize)).collect()
    }

    /// About `most` of its points, evenly through the file: what 3D
    /// shows of it.
    pub fn sample(&self, most: usize) -> Vec<u32> {
        let step = self.points.len().div_ceil(most.max(1)).max(1);
        (0..self.points.len())
            .step_by(step)
            .map(|n| n as u32)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_file_read_tells_its_progress_and_stops_when_told() {
        let dir = std::env::temp_dir().join(format!("creabim-cloud-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.xyz");
        let mut text = String::new();
        for i in 0..200_000 {
            text.push_str(&format!("{} {} 1.5\n", i % 100, i / 100));
        }
        std::fs::write(&path, &text).unwrap();
        let path = path.display().to_string();
        let seen = std::cell::Cell::new(0u64);
        let c = read_file_with(&path, MOST, &|n| {
            seen.set(n);
            true
        })
        .unwrap();
        assert_eq!(c.total, 200_000);
        assert_eq!(seen.get(), text.len() as u64);
        let e = read_file_with(&path, MOST, &|n| n < 1 << 20).unwrap_err();
        assert_eq!(e, CANCELLED);
        // fewer points kept when asked
        let c = read_file_with(&path, 10_000, &|_| true).unwrap();
        assert!(c.points.len() <= 10_001 && c.total == 200_000);
        std::fs::remove_dir_all(&dir).ok();
    }

    use super::*;

    /// A LAS 1.2 file of `points` in point format 2 (with colour).
    fn las(points: &[([f64; 3], [u16; 3])]) -> Vec<u8> {
        let mut h = vec![0u8; 227];
        h[0..4].copy_from_slice(b"LASF");
        h[24] = 1;
        h[25] = 2;
        h[94..96].copy_from_slice(&227u16.to_le_bytes());
        h[96..100].copy_from_slice(&227u32.to_le_bytes());
        h[104] = 2;
        h[105..107].copy_from_slice(&26u16.to_le_bytes());
        h[107..111].copy_from_slice(&(points.len() as u32).to_le_bytes());
        for k in 0..3 {
            h[131 + 8 * k..139 + 8 * k].copy_from_slice(&0.001f64.to_le_bytes());
        }
        h[155..163].copy_from_slice(&1000.0f64.to_le_bytes());
        for (p, c) in points {
            let mut r = [0u8; 26];
            for k in 0..3 {
                let off = if k == 0 { 1000.0 } else { 0.0 };
                let v = ((p[k] - off) / 0.001).round() as i32;
                r[4 * k..4 * k + 4].copy_from_slice(&v.to_le_bytes());
                r[20 + 2 * k..22 + 2 * k].copy_from_slice(&c[k].to_le_bytes());
            }
            h.extend(r);
        }
        h
    }

    #[test]
    fn a_text_cloud_reads_points_and_their_colours() {
        let t = "8\n1.0 2.0 0.5 120 255 0 0\n1.5;2.0;1.2;0;255;0\n# note\n3,4,1.0\n";
        let c = read_text(t.as_bytes(), MOST);
        assert_eq!((c.points.len(), c.total), (3, 3));
        assert!(c.at(0).distance(DVec3::new(1.0, 2.0, 0.5)) < 1e-6);
        assert_eq!(c.rgb[0], [255, 0, 0]);
        assert_eq!(c.rgb[1], [0, 255, 0]);
        assert_eq!(c.bounds(), [DVec2::new(1.0, 2.0), DVec2::new(3.0, 4.0)]);
        // without colours none are kept
        assert!(read_text("1 2 3\n4 5 6\n".as_bytes(), MOST).rgb.is_empty());
    }

    #[test]
    fn a_las_cloud_reads_scaled_points_and_colours() {
        let bytes = las(&[
            ([1001.25, 20.5, 1.0], [65535, 0, 0]),
            ([1002.0, 21.0, 2.5], [0, 32768, 0]),
        ]);
        let c = read_las(std::io::Cursor::new(&bytes), MOST).unwrap();
        assert_eq!(c.points.len(), 2);
        assert!(c.at(0).distance(DVec3::new(1001.25, 20.5, 1.0)) < 1e-4);
        assert_eq!((c.rgb[0], c.rgb[1]), ([255, 0, 0], [0, 128, 0]));
        let mut laz = bytes.clone();
        laz[104] |= 0x80;
        assert!(read_las(std::io::Cursor::new(&laz), MOST)
            .unwrap_err()
            .contains("LAZ"));
        assert!(read_las(std::io::Cursor::new(b"nothing".to_vec()), MOST).is_err());
    }

    #[test]
    fn a_big_file_is_read_every_few_points_and_keeps_its_span() {
        let t: String = (0..1000)
            .map(|i| format!("{} 0 {}\n", i as f64 * 0.01, i % 3))
            .collect();
        let c = read_text(t.as_bytes(), 100);
        assert_eq!(c.total, 1000);
        assert!(
            c.points.len() <= 100 && c.points.len() >= 50,
            "{}",
            c.points.len()
        );
        assert!((c.hi.x - 9.99).abs() < 1e-9);
    }

    #[test]
    fn a_plan_shows_the_slice_between_two_heights_and_snaps_to_its_points() {
        // a wall along x at y = 2, scanned at four heights
        let mut t = String::new();
        for i in 0..200 {
            for z in [0.2, 1.0, 1.4, 2.5] {
                t += &format!("{} 2.0 {z}\n", i as f64 * 0.02);
            }
        }
        // and a stool under the cut
        t += "1.0 1.0 0.4\n";
        let c = read_text(t.as_bytes(), MOST);
        let s = c.slice(0.9, 1.5, None, 100_000);
        // two heights fall on the same cells: one point a cell
        assert_eq!(s.points.len(), 200);
        assert!(s.points.iter().all(|p| (p.y - 2.0).abs() < 1e-6));
        let near = s.nearest(DVec2::new(1.005, 2.03), 0.05).unwrap();
        assert!(near.distance(DVec2::new(1.0, 2.0)) < 1e-6, "{near}");
        assert!(s.nearest(DVec2::new(1.0, 1.0), 0.05).is_none());
        // a coarser grid when that is too many
        assert!(c.slice(0.9, 1.5, None, 60).points.len() <= 60);
        let clip = Some([DVec2::new(0.0, 0.0), DVec2::new(1.0, 3.0)]);
        assert_eq!(c.slice(0.9, 1.5, clip, 100_000).points.len(), 51);
        assert_eq!(c.sample(10).len(), 10);
        // the ground: the lowest point of each metre square
        let g = c.ground(1.0, None);
        assert!(g.iter().all(|p| p.z <= 0.4 + 1e-6), "{g:?}");
        assert_eq!(g.len(), 5, "four metres of wall and the stool");
    }
}
