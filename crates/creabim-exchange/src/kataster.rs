//! A parcel's data from the cadastre of real estate (ÚGKK SR, ESKN: the
//! public WMS GetFeatureInfo service), ported from PilonDesigner's
//! `kataster.ts`: the parcel's number, its cadastral district, its area,
//! the kind of land and the number of the sheet of ownership. Owners
//! the public service does not give. The point goes from S-JTSK
//! (EPSG:5514, Křovák) over the Bessel ellipsoid and a seven-parameter
//! transformation to WGS84 and on to Web Mercator (EPSG:3857), in which
//! the service is asked.

const DEG: f64 = std::f64::consts::PI / 180.0;
const BESSEL: (f64, f64) = (6377397.155, 1.0 / 299.1528128);
const WGS84: (f64, f64) = (6378137.0, 1.0 / 298.257223563);

/// Křovák inverse (EPSG method 9819, Guidance Note 7-2): E, N of
/// EPSG:5514 (negative) to latitude and longitude on Bessel, degrees.
pub fn krovak_inverse(e_: f64, n_: f64) -> (f64, f64) {
    let (a, f) = BESSEL;
    let e2 = 2.0 * f - f * f;
    let e = e2.sqrt();
    let phi_c = 49.5 * DEG;
    let lam0 = (24.0 + 50.0 / 60.0) * DEG;
    let alpha_c = (30.0 + 17.0 / 60.0 + 17.30311 / 3600.0) * DEG;
    let phi_p = 78.5 * DEG;
    let kp = 0.9999;
    let s_c = phi_c.sin();
    let big_a = a * (1.0 - e2).sqrt() / (1.0 - e2 * s_c * s_c);
    let big_b = (1.0 + e2 * phi_c.cos().powi(4) / (1.0 - e2)).sqrt();
    let g0 = (s_c / big_b).asin();
    let t0 = (std::f64::consts::FRAC_PI_4 + g0 / 2.0).tan()
        * ((1.0 + e * s_c) / (1.0 - e * s_c)).powf(e * big_b / 2.0)
        / (std::f64::consts::FRAC_PI_4 + phi_c / 2.0)
            .tan()
            .powf(big_b);
    let n = phi_p.sin();
    let r0 = kp * big_a / phi_p.tan();
    // the southing and the westing
    let (xp, yp) = (-n_, -e_);
    let r = xp.hypot(yp);
    let th = yp.atan2(xp);
    let d = th / n;
    let t = 2.0
        * (((r0 / r).powf(1.0 / n) * (std::f64::consts::FRAC_PI_4 + phi_p / 2.0).tan()).atan()
            - std::f64::consts::FRAC_PI_4);
    let u = (alpha_c.cos() * t.sin() - alpha_c.sin() * t.cos() * d.cos()).asin();
    let v = (t.cos() * d.sin() / u.cos()).asin();
    let mut phi = u;
    for _ in 0..20 {
        let nx = 2.0
            * ((t0.powf(-1.0 / big_b)
                * (u / 2.0 + std::f64::consts::FRAC_PI_4)
                    .tan()
                    .powf(1.0 / big_b)
                * ((1.0 + e * phi.sin()) / (1.0 - e * phi.sin())).powf(e / 2.0))
            .atan()
                - std::f64::consts::FRAC_PI_4);
        let done = (nx - phi).abs() < 1e-14;
        phi = nx;
        if done {
            break;
        }
    }
    (phi / DEG, (lam0 - v / big_b) / DEG)
}

fn to_ecef(lat: f64, lon: f64, el: (f64, f64)) -> [f64; 3] {
    let e2 = 2.0 * el.1 - el.1 * el.1;
    let s = (lat * DEG).sin();
    let n = el.0 / (1.0 - e2 * s * s).sqrt();
    [
        n * (lat * DEG).cos() * (lon * DEG).cos(),
        n * (lat * DEG).cos() * (lon * DEG).sin(),
        n * (1.0 - e2) * s,
    ]
}

fn from_ecef(p: [f64; 3], el: (f64, f64)) -> (f64, f64) {
    let e2 = 2.0 * el.1 - el.1 * el.1;
    let r = p[0].hypot(p[1]);
    let mut lat = p[2].atan2(r * (1.0 - e2));
    for _ in 0..10 {
        let s = lat.sin();
        let n = el.0 / (1.0 - e2 * s * s).sqrt();
        lat = (p[2] + e2 * n * s).atan2(r);
    }
    (lat / DEG, p[1].atan2(p[0]) / DEG)
}

/// S-JTSK (EPSG:5514) to WGS84 latitude and longitude, degrees: Křovák,
/// Bessel, the position vector transformation of EPSG:5514
/// (+towgs84=485.021,169.465,483.839,7.786342,4.397554,4.102655,0).
pub fn jtsk_to_wgs84(e: f64, n: f64) -> (f64, f64) {
    let (lat, lon) = krovak_inverse(e, n);
    let [x, y, z] = to_ecef(lat, lon, BESSEL);
    let sec = DEG / 3600.0;
    let (rx, ry, rz) = (7.786342 * sec, 4.397554 * sec, 4.102655 * sec);
    from_ecef(
        [
            485.021 + (x - rz * y + ry * z),
            169.465 + (rz * x + y - rx * z),
            483.839 + (-ry * x + rx * y + z),
        ],
        WGS84,
    )
}

pub fn to_web_mercator(lat: f64, lon: f64) -> (f64, f64) {
    let r = 6378137.0;
    (
        r * lon * DEG,
        r * (std::f64::consts::FRAC_PI_4 + lat * DEG / 2.0).tan().ln(),
    )
}

pub const ESKN: &str = "https://kataster.skgeodesy.sk/eskn/services/NR";

/// The register of the cadastre asked: C (the parcels as they are used)
/// or E (the parcels as they are owned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register {
    C,
    E,
}

/// The GetFeatureInfo address for a point in S-JTSK.
pub fn eskn_url(e: f64, n: f64, reg: Register) -> String {
    let (lat, lon) = jtsk_to_wgs84(e, n);
    let (x, y) = to_web_mercator(lat, lon);
    let d = 2.0;
    let (svc, layer) = match reg {
        Register::C => ("kn_wms_norm", "5"),
        Register::E => ("uo_wms_norm", "0"),
    };
    format!(
        "{ESKN}/{svc}/MapServer/WmsServer?service=WMS&version=1.3.0&request=GetFeatureInfo&layers={layer}&query_layers={layer}&styles=&crs=EPSG:3857&bbox={:.2},{:.2},{:.2},{:.2}&width=41&height=41&i=20&j=20&info_format=text/xml&feature_count=3",
        x - d,
        y - d,
        x + d,
        y + d
    )
}

/// What the cadastre says of a parcel.
#[derive(Clone, Debug, PartialEq)]
pub struct Parcel {
    pub register: Register,
    pub number: Option<String>,
    pub district_code: Option<String>,
    pub district: Option<String>,
    /// Its area, m².
    pub area: Option<f64>,
    pub land_kind: Option<String>,
    /// The number of its sheet of ownership.
    pub sheet: Option<String>,
}

fn un_xml(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// A GetFeatureInfo answer (text/xml, a FIELDS element with attributes)
/// read as a parcel.
pub fn parse_eskn(xml: &str, reg: Register) -> Option<Parcel> {
    let start = xml.find("<FIELDS")? + "<FIELDS".len();
    let rest = &xml[start..];
    let body = &rest[..rest.find('>')?];
    let mut fields: Vec<(String, String)> = vec![];
    let mut s = body;
    while let Some(eq) = s.find('=') {
        let key = s[..eq].split_whitespace().last().unwrap_or("").to_string();
        let after = s[eq + 1..].trim_start();
        let Some(inner) = after.strip_prefix('"') else {
            break;
        };
        let Some(end) = inner.find('"') else { break };
        let value = un_xml(&inner[..end]);
        if !key.is_empty() && !value.is_empty() && value != "Null" {
            fields.push((key, value));
        }
        s = &inner[end + 1..];
    }
    let pick = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| fields.iter().find(|f| f.0 == *k).map(|f| f.1.clone()))
    };
    let p = Parcel {
        register: reg,
        number: pick(&["ČísloparcelyregistraC", "ČísloparcelyregistraE"]),
        district_code: pick(&["Kódkatastrálnehoúzemia"]),
        district: pick(&["Názovkatastrálnehoúzemia"]),
        area: pick(&["VýmeraSPIvm2"])
            .and_then(|v| v.replace(',', ".").parse::<f64>().ok())
            .filter(|v| v.is_finite()),
        land_kind: pick(&["Názovdruhupozemku"]),
        sheet: pick(&["Číslolistuvlastníctva"]),
    };
    (p.number.is_some() || p.district.is_some()).then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn krovak_inverse_matches_the_epsg_example() {
        // Guidance Note 7-2, method 9819
        let (lat, lon) = krovak_inverse(-568990.997, -1050538.643);
        assert!((lat - (50.0 + 12.0 / 60.0 + 32.442 / 3600.0)).abs() < 1e-6);
        assert!((lon - (16.0 + 50.0 / 60.0 + 59.179 / 3600.0)).abs() < 1e-6);
    }

    #[test]
    fn jtsk_goes_to_wgs84_with_the_datum_shift() {
        let b = krovak_inverse(-576800.0, -1279500.0);
        let w = jtsk_to_wgs84(-576800.0, -1279500.0);
        assert!(w.0 > 48.0 && w.0 < 48.3 && w.1 > 17.0 && w.1 < 17.3);
        let dm = ((w.0 - b.0) * 111_000.0).hypot((w.1 - b.1) * 74_000.0);
        assert!(dm > 50.0 && dm < 200.0, "{dm}");
        let url = eskn_url(-576800.0, -1279500.0, Register::C);
        assert!(
            url.contains("kn_wms_norm/MapServer/WmsServer?")
                && url.contains("crs=EPSG:3857&bbox=1")
        );
    }

    #[test]
    fn an_answer_reads_as_a_parcel() {
        let xml = r#"<?xml version="1.0"?><FeatureInfoResponse><FIELDS ČísloparcelyregistraC="1234/5" Kódkatastrálnehoúzemia="804959" Názovkatastrálnehoúzemia="Ružinov" VýmeraSPIvm2="635" Názovdruhupozemku="Záhrada" Číslolistuvlastníctva="4521" Poznámka="Null"/></FeatureInfoResponse>"#;
        let p = parse_eskn(xml, Register::C).unwrap();
        assert_eq!(p.number.as_deref(), Some("1234/5"));
        assert_eq!(p.district.as_deref(), Some("Ružinov"));
        assert_eq!(p.area, Some(635.0));
        assert_eq!(p.sheet.as_deref(), Some("4521"));
        assert_eq!(p.land_kind.as_deref(), Some("Záhrada"));
        assert!(parse_eskn("<x/>", Register::C).is_none());
    }
}
