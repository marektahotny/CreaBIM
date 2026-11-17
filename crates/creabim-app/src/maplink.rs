//! The link to Pilon Map Downloader's site, as PilonDesigner's *Mapové
//! podklady*: the site opens in a window of the app, the area is picked
//! there, and what it sends (the site plan DXF, the orthophoto, the
//! terrain, the origin in S-JTSK) is put into the project as a map
//! package is.
//!
//! The site talks to the app the way it talks to Revit (Pilon Map Link):
//! it is opened with `#revit=<port>`, asks `GET /status` of
//! `http://localhost:<port>` and posts its package to `/insert` as JSON.
//! So the app listens on a port of this computer only, and answers the
//! map site's origin only.

use super::CreaBimApp;
use eframe::egui::{self, Vec2};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

/// The largest package taken (an orthophoto and a terrain), bytes.
const MAX_BODY: usize = 256 << 20;

/// The listener the map site sends to.
pub struct MapLink {
    pub port: u16,
    /// Packages received (JSON), not yet put in.
    inbox: Arc<Mutex<Vec<String>>>,
    /// The project's name and the plan's, for the site to show.
    info: Arc<Mutex<(String, String)>>,
}

impl MapLink {
    /// Listen on a free port of this computer; a package received wakes
    /// the app.
    pub fn start(ctx: &egui::Context) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let inbox: Arc<Mutex<Vec<String>>> = Default::default();
        let info: Arc<Mutex<(String, String)>> = Default::default();
        let mut listeners = vec![listener];
        // `localhost` may mean ::1 to the browser
        if let Ok(l) = TcpListener::bind(("::1", port)) {
            listeners.push(l);
        }
        for l in listeners {
            let (inbox, info, ctx) = (inbox.clone(), info.clone(), ctx.clone());
            std::thread::spawn(move || {
                for stream in l.incoming().flatten() {
                    let (inbox, info, ctx) = (inbox.clone(), info.clone(), ctx.clone());
                    std::thread::spawn(move || {
                        if let Some(body) = serve(stream, &info) {
                            inbox.lock().unwrap().push(body);
                            ctx.request_repaint();
                        }
                    });
                }
            });
        }
        Ok(Self { port, inbox, info })
    }

    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.inbox.lock().unwrap())
    }

    pub fn set_info(&self, project: &str, level: &str) {
        let mut i = self.info.lock().unwrap();
        if i.0 != project || i.1 != level {
            *i = (project.into(), level.into());
        }
    }

    /// The site's address, linked to this listener.
    pub fn url(&self) -> String {
        format!("{}#revit={}", crate::webui::MAPS_URL, self.port)
    }
}

/// Answer one request; a package posted to `/insert` is returned.
fn serve(stream: TcpStream, info: &Mutex<(String, String)>) -> Option<String> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(60)));
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    r.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next()?.to_string(), parts.next()?.to_string());
    let (mut origin, mut length) = (String::new(), 0usize);
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).ok()? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "origin" => origin = v.trim().to_string(),
                "content-length" => length = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    // the map site only: no other page may ask or send
    let allowed = origin == crate::webui::origin(crate::webui::MAPS_URL);
    let cors = if allowed {
        format!(
            "Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n\
             Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
             Access-Control-Allow-Headers: Content-Type\r\n\
             Access-Control-Allow-Private-Network: true\r\n\
             Access-Control-Max-Age: 600\r\n"
        )
    } else {
        String::new()
    };
    let path = path.split(['?', '#']).next().unwrap_or("");
    let mut package = None;
    let (code, body) = match (method.as_str(), path) {
        _ if !allowed => (
            403,
            r#"{"ok":false,"error":"origin not allowed"}"#.to_string(),
        ),
        ("OPTIONS", _) => (204, String::new()),
        ("GET", "/status") => {
            let i = info.lock().unwrap();
            (
                200,
                serde_json::json!({
                    "ok": true,
                    "app": "CreaBIM",
                    "version": env!("CARGO_PKG_VERSION"),
                    "revit": "CreaBIM",
                    "project": i.0,
                    "level": i.1,
                })
                .to_string(),
            )
        }
        ("POST", "/insert") if length > 0 && length <= MAX_BODY => {
            let mut b = vec![0u8; length];
            match r.read_exact(&mut b) {
                Ok(()) => {
                    package = String::from_utf8(b).ok();
                    (
                        200,
                        serde_json::json!({
                            "ok": package.is_some(),
                            "message": "Podklady prijaté: CreaBIM ich vkladá do pôdorysu",
                        })
                        .to_string(),
                    )
                }
                Err(e) => (
                    400,
                    serde_json::json!({"ok": false, "error": e.to_string()}).to_string(),
                ),
            }
        }
        ("POST", "/insert") => (
            413,
            r#"{"ok":false,"error":"package too large"}"#.to_string(),
        ),
        _ => (404, r#"{"ok":false,"error":"not found"}"#.to_string()),
    };
    let reason = match code {
        200 => "OK",
        204 => "No Content",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Bad Request",
    };
    let mut s = r.into_inner();
    let _ = write!(
        s,
        "HTTP/1.1 {code} {reason}\r\n{cors}Content-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = s.flush();
    package
}

/// The files of a map package made from what the site sends (the same
/// content it sends to Revit): local coordinates, heights to ±0,000.
pub fn files_of(payload: &serde_json::Value) -> Vec<(String, Vec<u8>)> {
    use base64::Engine;
    let b64 = |v: &serde_json::Value| {
        base64::engine::general_purpose::STANDARD
            .decode(v.as_str()?.trim())
            .ok()
    };
    let num = |v: &serde_json::Value| v.as_f64().filter(|x| x.is_finite());
    let mut files = vec![];
    let origin = payload["origin"]
        .as_array()
        .and_then(|o| Some((num(o.first()?)?, num(o.get(1)?)?)));
    if let Some((e, n)) = origin {
        let h0 = num(&payload["h0"]).unwrap_or(0.0);
        let mut t = format!(
            "Bod lokálneho počiatku (Pilon Map Downloader)\r\n\
             S-JTSK / Krovak East North (EPSG:5514):  E = {e:.3}   N = {n:.3}\r\n\
             Výška ±0,000 = {h0:.3} m n. m. (Bpv)\r\n"
        );
        if let Some(z) = num(&payload["originZ"]) {
            t.push_str(&format!(
                "Výška terénu v bode počiatku Z = {z:.3} m n. m. (Bpv)\r\n"
            ));
        }
        files.push(("bod_pociatku.txt".to_string(), t.into_bytes()));
    }
    if let Some(d) = b64(&payload["dxf"]) {
        files.push(("situacia.dxf".to_string(), d));
    }
    if let Some(p) = payload["terrain"]["points"].as_array() {
        let mut t = String::new();
        for q in p.chunks(3).filter(|q| q.len() == 3) {
            if let (Some(x), Some(y), Some(z)) = (num(&q[0]), num(&q[1]), num(&q[2])) {
                t.push_str(&format!("{x} {y} {z}\n"));
            }
        }
        if !t.is_empty() {
            files.push(("teren.xyz".to_string(), t.into_bytes()));
        }
    }
    let o = &payload["ortho"];
    if let (Some(jpg), Some(x), Some(y), Some(w), Some(h)) = (
        b64(&o["jpg"]),
        num(&o["minX"]),
        num(&o["minY"]),
        num(&o["width"]),
        num(&o["height"]),
    ) {
        let png = jpg.starts_with(&[0x89, b'P']);
        let size = image::ImageReader::new(std::io::Cursor::new(&jpg))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.into_dimensions().ok());
        if let Some((pw, ph)) = size.filter(|(pw, ph)| *pw > 0 && *ph > 0 && w > 0.0 && h > 0.0) {
            // a world file: the pixel's size, and its upper left pixel's middle
            let (a, e) = (w / pw as f64, h / ph as f64);
            let world = format!(
                "{a:.10}\r\n0.0\r\n0.0\r\n{:.10}\r\n{:.6}\r\n{:.6}\r\n",
                -e,
                x + a / 2.0,
                y + h - e / 2.0
            );
            let (img, wld) = if png {
                ("ortofoto.png", "ortofoto.pgw")
            } else {
                ("ortofoto.jpg", "ortofoto.jgw")
            };
            files.push((img.to_string(), jpg));
            files.push((wld.to_string(), world.into_bytes()));
        }
    }
    files
}

impl CreaBimApp {
    /// `map:open` from the Cadastre tab: the map site in its window.
    pub(super) fn maplink_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "map" || arg != "open" {
            return false;
        }
        self.maps_open = true;
        true
    }

    /// Where what the site sends is unpacked: beside the project's file,
    /// else in the user's temporary folder.
    fn maps_dir(&self) -> std::path::PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let p = std::path::Path::new(&self.path);
        match (p.parent(), p.file_stem()) {
            (Some(dir), Some(stem)) if dir.is_dir() => {
                dir.join(format!("{}-mapy-{stamp}", stem.to_string_lossy()))
            }
            _ => std::env::temp_dir().join(format!("creabim-mapy-{stamp}")),
        }
    }

    /// The map site's window while it is open, and what it sent put in.
    pub(super) fn maps_window(&mut self, ctx: &egui::Context) {
        let t = crate::i18n::tr;
        // what the site sent, put in as a map package is
        let got = self.maplink.as_ref().map(|l| l.take()).unwrap_or_default();
        for body in got {
            let files = serde_json::from_str::<serde_json::Value>(&body)
                .map(|v| files_of(&v))
                .unwrap_or_default();
            if files.is_empty() {
                self.status = t("The map sent nothing to put in.");
                continue;
            }
            let dir = self.maps_dir();
            self.map_put_in(files, &dir, None);
        }
        if !self.maps_open {
            if self.maps_rect.take().is_some() {
                if let Some(web) = &mut self.shell.web {
                    web.maps_load(None);
                }
            }
            return;
        }
        if self.maplink.is_none() {
            match MapLink::start(ctx) {
                Ok(l) => self.maplink = Some(l),
                Err(e) => {
                    self.status = format!("{}: {e}", t("The map link couldn't start"));
                    self.maps_open = false;
                    return;
                }
            }
        }
        let Some(link) = &self.maplink else { return };
        let model = self.doc.model();
        let project = model
            .project_info()
            .map(|(_, p)| p.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "CreaBIM".into());
        let plan = self
            .map_plan(None)
            .and_then(|(v, _)| model.get(v))
            .map_or(String::new(), |e| e.name.clone());
        link.set_info(&project, &plan);
        let url = link.url();
        let mut open = true;
        let mut rect = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let size = (area.size() - Vec2::new(80.0, 80.0)).max(Vec2::new(480.0, 360.0));
        let in_app = self.shell.web.is_some();
        super::shell::window(t("Map underlays"))
            .id(egui::Id::new("maps.window"))
            .open(&mut open)
            .default_pos(area.center() - size / 2.0)
            .default_size(size)
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(t(
                        "Pick the area and what to take, then send it with the map's Revit button (Vložiť do Revitu): it comes here, into the plan open.",
                    ))
                    .weak(),
                );
                if in_app {
                    let (r, _) = ui.allocate_exact_size(
                        ui.available_size().max(Vec2::new(320.0, 240.0)),
                        egui::Sense::hover(),
                    );
                    rect = Some(r);
                } else {
                    // no pages in this window (Wayland): the browser does
                    ui.hyperlink_to(t("Open the map in the browser"), &url);
                }
            });
        if rect.is_some() && self.maps_rect.is_none() {
            if let Some(web) = &mut self.shell.web {
                web.maps_load(Some(&url));
            }
        }
        self.maps_rect = rect;
        if !open {
            self.maps_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_map_sends_becomes_a_packages_files() {
        use base64::Engine;
        let b64 = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
        // a 4 × 2 px picture
        let mut png = vec![];
        image::RgbImage::new(4, 2)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let v = serde_json::json!({
            "app": "PilonMapDownloader",
            "origin": [-450123.456, -1250456.789],
            "h0": 312.45,
            "originZ": 311.9,
            "dxf": b64(b"0\nEOF\n"),
            "terrain": {"points": [0.0, 0.0, -0.5, 10.0, 0.0, 0.25, 0.0, 10.0, 1.0]},
            "ortho": {"jpg": b64(&png), "minX": -10.0, "minY": -5.0, "width": 40.0, "height": 20.0},
        });
        let files = files_of(&v);
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "bod_pociatku.txt",
                "situacia.dxf",
                "teren.xyz",
                "ortofoto.png",
                "ortofoto.pgw"
            ]
        );
        let pkg = creabim_exchange::mapdl::package(&files);
        let o = pkg.origin.unwrap();
        assert_eq!(
            (o.e, o.n, o.h0, o.ground),
            (-450123.456, -1250456.789, 312.45, Some(311.9))
        );
        let world = String::from_utf8(files[4].1.clone()).unwrap();
        let [min, size] =
            creabim_exchange::mapdl::ortho_placement(&world, (4, 2), Some(&o)).unwrap();
        assert!((min - glam::DVec2::new(-10.0, -5.0)).length() < 1e-6);
        assert!((size - glam::DVec2::new(40.0, 20.0)).length() < 1e-6);
    }

    #[test]
    fn the_listener_answers_the_map_site_only() {
        let ctx = egui::Context::default();
        let link = MapLink::start(&ctx).unwrap();
        link.set_info("Dom", "Prízemie");
        let ask = |req: String| {
            let mut s = TcpStream::connect(("127.0.0.1", link.port)).unwrap();
            s.write_all(req.as_bytes()).unwrap();
            let mut out = String::new();
            s.read_to_string(&mut out).unwrap();
            out
        };
        let site = crate::webui::origin(crate::webui::MAPS_URL);
        let ok = ask(format!("GET /status HTTP/1.1\r\nOrigin: {site}\r\n\r\n"));
        assert!(
            ok.starts_with("HTTP/1.1 200") && ok.contains("\"project\":\"Dom\""),
            "{ok}"
        );
        assert!(ok.contains(&format!("Access-Control-Allow-Origin: {site}")));
        let other = ask("GET /status HTTP/1.1\r\nOrigin: https://example.com\r\n\r\n".into());
        assert!(other.starts_with("HTTP/1.1 403"), "{other}");
        let body = r#"{"origin":[1,2]}"#;
        let sent = ask(format!(
            "POST /insert HTTP/1.1\r\nOrigin: {site}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ));
        assert!(sent.starts_with("HTTP/1.1 200"), "{sent}");
        // it is handed over once the answer is sent
        let mut got = vec![];
        for _ in 0..200 {
            got = link.take();
            if !got.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(got, vec![body.to_string()]);
    }
}
