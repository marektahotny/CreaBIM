//! The sun of the 3D view: where it stands, set by hand or from a date and
//! a time at the project's place, and how the shadows it casts look.

use serde::{Deserialize, Serialize};

/// Where the project is when it has no latitude and longitude: Bratislava.
pub const DEFAULT_PLACE: [f64; 2] = [48.1486, 17.1077];

/// How the sun is placed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SunMode {
    /// From a date and a clock time at the project's latitude and
    /// longitude, as it really stands there.
    #[default]
    Time,
    /// Azimuth and altitude typed in.
    Manual,
}

/// The sun and the shadows of the 3D view: a personal setting, like the
/// view's render mode.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sun {
    /// Shadows cast in the 3D view.
    pub shadows: bool,
    pub mode: SunMode,
    /// Degrees clockwise from true north (manual mode).
    pub azimuth: f64,
    /// Degrees above the horizon (manual mode).
    pub altitude: f64,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Clock time, hours (14.5 is 14:30).
    pub hour: f64,
    /// Central European time, with summer time in summer: what the clock
    /// in Slovakia and Czechia shows.
    pub summer_time: bool,
    /// 0 no shadow, 1 the shadow gets no sunlight at all.
    pub strength: f32,
    /// Shadows fall on the ground at ±0,000 where nothing else is drawn.
    pub ground: bool,
    /// The shadow map's side, pixels: 2048, 4096 or 8192.
    pub resolution: u32,
}

impl Default for Sun {
    fn default() -> Self {
        Self {
            shadows: true,
            mode: SunMode::Time,
            azimuth: 146.0,
            altitude: 50.0,
            year: 2026,
            month: 6,
            day: 21,
            hour: 14.0,
            summer_time: true,
            strength: 0.75,
            ground: true,
            resolution: 4096,
        }
    }
}

impl Sun {
    /// Azimuth (degrees clockwise from true north) and altitude (degrees
    /// above the horizon) at `place` (latitude, longitude).
    pub fn position(&self, place: [f64; 2]) -> (f64, f64) {
        match self.mode {
            SunMode::Manual => (self.azimuth, self.altitude),
            SunMode::Time => {
                let offset = if self.summer_time { 2.0 } else { 1.0 };
                solar_position(
                    place[0],
                    place[1],
                    self.year,
                    self.month,
                    self.day,
                    self.hour - offset,
                )
            }
        }
    }

    /// The direction toward the sun in the model's axes (x east and y up
    /// on the drawings, z up), turned by the project's true north (degrees
    /// anticlockwise from the drawings' up).
    pub fn direction(&self, place: [f64; 2], true_north: f64) -> [f32; 3] {
        let (az, alt) = self.position(place);
        direction(az, alt, true_north)
    }

    /// Summer time as the EU keeps it on this date: from the last Sunday
    /// of March to the last Sunday of October.
    pub fn summer_on(year: i32, month: u32, day: u32) -> bool {
        let start = last_sunday(year, 3);
        let end = last_sunday(year, 10);
        match month {
            4..=9 => true,
            3 => day >= start,
            10 => day < end,
            _ => false,
        }
    }
}

/// The unit vector toward a sun at `azimuth` (clockwise from true north)
/// and `altitude`, in the model's axes.
pub fn direction(azimuth: f64, altitude: f64, true_north: f64) -> [f32; 3] {
    // true north is the drawings' up turned anticlockwise by true_north;
    // the sun's bearing turns clockwise from there
    let a = (true_north - azimuth).to_radians();
    let h = altitude.to_radians();
    [
        (-a.sin() * h.cos()) as f32,
        (a.cos() * h.cos()) as f32,
        h.sin() as f32,
    ]
}

/// The day of the month of `month`'s last Sunday.
fn last_sunday(year: i32, month: u32) -> u32 {
    let last = days_in_month(year, month);
    last - (weekday(year, month, last) % 7)
}

/// 0 Sunday … 6 Saturday (Zeller's congruence, Gregorian).
fn weekday(year: i32, month: u32, day: u32) -> u32 {
    let (y, m) = if month < 3 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let k = y.rem_euclid(100);
    let j = y.div_euclid(100);
    let h = (day as i32 + 13 * (m as i32 + 1) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
    // Zeller: 0 Saturday, 1 Sunday …
    ((h + 6) % 7) as u32
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn day_of_year(year: i32, month: u32, day: u32) -> u32 {
    (1..month).map(|m| days_in_month(year, m)).sum::<u32>() + day
}

/// The sun's azimuth (degrees clockwise from north) and altitude (degrees
/// above the horizon, refraction left out) at a latitude and longitude
/// (degrees, north and east positive) on a date at `utc_hour` (hours of
/// universal time, any value: it may run into the day before or after).
/// NOAA's general solar position formulas: within a fraction of a degree,
/// plenty for shadows and insolation studies.
pub fn solar_position(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    utc_hour: f64,
) -> (f64, f64) {
    let leap = days_in_month(year, 2) == 29;
    let days = if leap { 366.0 } else { 365.0 };
    let n = day_of_year(year, month, day) as f64;
    let g = 2.0 * std::f64::consts::PI / days * (n - 1.0 + (utc_hour - 12.0) / 24.0);
    let eqtime = 229.18
        * (0.000075 + 0.001868 * g.cos()
            - 0.032077 * g.sin()
            - 0.014615 * (2.0 * g).cos()
            - 0.040849 * (2.0 * g).sin());
    let decl = 0.006918 - 0.399912 * g.cos() + 0.070257 * g.sin() - 0.006758 * (2.0 * g).cos()
        + 0.000907 * (2.0 * g).sin()
        - 0.002697 * (3.0 * g).cos()
        + 0.00148 * (3.0 * g).sin();
    // true solar time, minutes
    let tst = utc_hour * 60.0 + eqtime + 4.0 * longitude;
    let ha = (tst / 4.0 - 180.0).to_radians();
    let lat = latitude.to_radians();
    let cos_zen = (lat.sin() * decl.sin() + lat.cos() * decl.cos() * ha.cos()).clamp(-1.0, 1.0);
    let altitude = 90.0 - cos_zen.acos().to_degrees();
    // from south, positive west; north is 180 more
    let from_south = ha
        .sin()
        .atan2(ha.cos() * lat.sin() - decl.tan() * lat.cos())
        .to_degrees();
    let azimuth = (from_south + 180.0).rem_euclid(360.0);
    (azimuth, altitude)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midsummer_noon_in_bratislava() {
        // solar noon in Bratislava (17.1° E) is about 10:50 UTC in June
        let (az, alt) = solar_position(48.1486, 17.1077, 2026, 6, 21, 10.85);
        assert!((az - 180.0).abs() < 3.0, "azimuth {az}");
        // 90 − 48.15 + 23.44
        assert!((alt - 65.3).abs() < 0.5, "altitude {alt}");
    }

    #[test]
    fn equinox_sunrise_is_east() {
        // 21 March, about 05:00 UTC: the sun rises in the east
        let (az, alt) = solar_position(48.1486, 17.1077, 2026, 3, 21, 4.85);
        assert!((az - 90.0).abs() < 5.0, "azimuth {az}");
        assert!(alt.abs() < 3.0, "altitude {alt}");
    }

    #[test]
    fn night_is_below_the_horizon() {
        let (_, alt) = solar_position(48.1486, 17.1077, 2026, 12, 21, 23.0);
        assert!(alt < -30.0);
    }

    #[test]
    fn summer_time_dates() {
        // 2026: 29 March to 25 October
        assert!(!Sun::summer_on(2026, 3, 28));
        assert!(Sun::summer_on(2026, 3, 29));
        assert!(Sun::summer_on(2026, 10, 24));
        assert!(!Sun::summer_on(2026, 10, 25));
        assert!(!Sun::summer_on(2026, 1, 10));
    }

    #[test]
    fn directions() {
        // a southern sun on the horizon, north up: toward −y
        let d = direction(180.0, 0.0, 0.0);
        assert!(d[1] < -0.99 && d[2].abs() < 1e-6);
        // east: +x
        let d = direction(90.0, 0.0, 0.0);
        assert!(d[0] > 0.99);
        // true north turned 90° anticlockwise: north is −x on the drawing,
        // so east is +y
        let d = direction(90.0, 0.0, 90.0);
        assert!(d[1] > 0.99, "{d:?}");
        // overhead
        assert!(direction(0.0, 90.0, 0.0)[2] > 0.999);
    }

    #[test]
    fn the_old_light_is_the_manual_default() {
        let s = Sun {
            mode: SunMode::Manual,
            ..Sun::default()
        };
        let d = s.direction(DEFAULT_PLACE, 0.0);
        let old = glam::Vec3::new(0.4, -0.6, 0.85).normalize();
        assert!(glam::Vec3::from(d).dot(old) > 0.995, "{d:?}");
    }
}
