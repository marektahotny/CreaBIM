//! A final circuit checked as PilonDesigner's `elektro.ts` does it: the
//! design current IB from the installed power, the voltage drop after
//! STN 33 2000-5-52 annex G,
//! u = b · (ρ1 · L / S · cos φ + λ · L · sin φ) · IB, Δu = 100 · u / U0
//! (b = 2 single-phase, 1 three-phase; ρ1 = 0.0225 Ω·mm²/m for copper,
//! 0.036 for aluminium; λ = 0.08 mΩ/m), its limits of table G.52.1,
//! the protection IB ≤ In ≤ Iz (STN 33 2000-4-43, 433.1) with Iz of
//! tables B.52.2 and B.52.4, and automatic disconnection in a TN system
//! (STN 33 2000-4-41, 411.4.4): Zs = Ze + loop ≤ U0 / Ia.

/// How the cable is laid (STN 33 2000-5-52 table B.52.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    /// Wires in a conduit in a thermally insulating wall.
    A1,
    /// A cable in a conduit in a thermally insulating wall.
    #[default]
    A2,
    /// Wires in a conduit or trunking on a wall.
    B1,
    /// A cable in a conduit or trunking on a wall.
    B2,
    /// A cable straight on a wall or in plaster.
    C,
}

impl Method {
    pub const ALL: [Method; 5] = [Method::A1, Method::A2, Method::B1, Method::B2, Method::C];

    pub fn name(self) -> &'static str {
        match self {
            Method::A1 => "A1 – wires in a conduit in an insulating wall",
            Method::A2 => "A2 – cable in a conduit in an insulating wall",
            Method::B1 => "B1 – wires in a conduit or trunking on a wall",
            Method::B2 => "B2 – cable in a conduit or trunking on a wall",
            Method::C => "C – cable on a wall or in plaster",
        }
    }

    /// Iz, A, for each of [`IZ_S`]: copper, PVC 70 °C, 30 °C around;
    /// two loaded conductors (table B.52.2) or three (B.52.4).
    fn row(self, three: bool) -> [f64; 7] {
        match (three, self) {
            (false, Method::A1) => [14.5, 19.5, 26.0, 34.0, 46.0, 61.0, 80.0],
            (false, Method::A2) => [14.0, 18.5, 25.0, 32.0, 43.0, 57.0, 75.0],
            (false, Method::B1) => [17.5, 24.0, 32.0, 41.0, 57.0, 76.0, 101.0],
            (false, Method::B2) => [16.5, 23.0, 30.0, 38.0, 52.0, 69.0, 90.0],
            (false, Method::C) => [19.5, 27.0, 36.0, 46.0, 63.0, 85.0, 112.0],
            (true, Method::A1) => [13.5, 18.0, 24.0, 31.0, 42.0, 56.0, 73.0],
            (true, Method::A2) => [13.0, 17.5, 23.0, 29.0, 39.0, 52.0, 68.0],
            (true, Method::B1) => [15.5, 21.0, 28.0, 36.0, 50.0, 68.0, 89.0],
            (true, Method::B2) => [15.0, 20.0, 27.0, 34.0, 46.0, 62.0, 80.0],
            (true, Method::C) => [17.5, 24.0, 32.0, 41.0, 57.0, 76.0, 96.0],
        }
    }
}

/// The cross-sections of the Iz tables, mm².
pub const IZ_S: [f64; 7] = [1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0];

/// Iz of a copper PVC cable, A; none for a cross-section off the table.
pub fn iz_table(method: Method, three_phase: bool, s_mm2: f64) -> Option<f64> {
    let i = IZ_S.iter().position(|x| (x - s_mm2).abs() < 1e-6)?;
    Some(method.row(three_phase)[i])
}

/// Preferred rated currents of circuit breakers, A (STN EN 60898-1).
pub const BREAKER_IN: [f64; 14] = [
    6.0, 8.0, 10.0, 13.0, 16.0, 20.0, 25.0, 32.0, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0,
];

/// The smallest of `list` that is at least `x`.
pub fn next_std(list: &[f64], x: f64) -> Option<f64> {
    list.iter().copied().find(|v| *v >= x - 1e-9)
}

pub const RHO1_CU: f64 = 0.0225;
pub const RHO1_AL: f64 = 0.036;
/// Reactance of a conductor, Ω/m.
pub const LAMBDA: f64 = 0.08e-3;
/// 230/400 V (STN EN 60038).
pub const U0: f64 = 230.0;
pub const UZ: f64 = 400.0;

/// Where the installation is fed from (table G.52.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Supply {
    /// Straight from the public low-voltage network.
    #[default]
    Public,
    /// From its own low-voltage source.
    Own,
}

/// The voltage drop allowed, %.
pub fn du_limit(supply: Supply, lighting: bool) -> f64 {
    match (supply, lighting) {
        (Supply::Public, true) => 3.0,
        (Supply::Public, false) => 5.0,
        (Supply::Own, true) => 6.0,
        (Supply::Own, false) => 8.0,
    }
}

/// A breaker's tripping characteristic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Curve {
    #[default]
    B,
    C,
    D,
}

impl Curve {
    pub const ALL: [Curve; 3] = [Curve::B, Curve::C, Curve::D];

    pub fn letter(self) -> &'static str {
        match self {
            Curve::B => "B",
            Curve::C => "C",
            Curve::D => "D",
        }
    }

    /// Ia / In: the current that trips it at once.
    pub fn trip(self) -> f64 {
        match self {
            Curve::B => 5.0,
            Curve::C => 10.0,
            Curve::D => 20.0,
        }
    }
}

/// A final circuit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circuit {
    /// Installed power, W.
    pub power_w: f64,
    pub cos_phi: f64,
    pub three_phase: bool,
    /// Length of the cable, m.
    pub length_m: f64,
    /// Cross-section, mm².
    pub s_mm2: f64,
    /// The breaker's rated current, A.
    pub in_a: f64,
    pub curve: Curve,
    pub aluminium: bool,
    pub method: Method,
    /// A lighting circuit: the tighter voltage drop.
    pub lighting: bool,
    pub supply: Supply,
    /// Impedance of the fault loop before the board, Ω; none leaves the
    /// disconnection unchecked.
    pub ze: Option<f64>,
}

impl Default for Circuit {
    fn default() -> Self {
        Circuit {
            power_w: 2000.0,
            cos_phi: 0.8,
            three_phase: false,
            length_m: 20.0,
            s_mm2: 2.5,
            in_a: 16.0,
            curve: Curve::B,
            aluminium: false,
            method: Method::A2,
            lighting: false,
            supply: Supply::Public,
            ze: None,
        }
    }
}

/// What the check of a circuit found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Check {
    /// Design current, A.
    pub ib: f64,
    /// Voltage drop, V and %.
    pub u: f64,
    pub du: f64,
    pub du_limit: f64,
    pub ok_du: bool,
    /// Current-carrying capacity from the tables; none for aluminium or
    /// a cross-section off them.
    pub iz: Option<f64>,
    /// IB ≤ In ≤ Iz; none without Iz.
    pub ok_prot: Option<bool>,
    /// Fault loop, Ω, and the most that still trips the breaker.
    pub zs: Option<f64>,
    pub zs_max: f64,
    pub ok_zs: Option<bool>,
}

impl Check {
    /// Nothing checked failed.
    pub fn ok(&self) -> bool {
        self.ok_du && self.ok_prot != Some(false) && self.ok_zs != Some(false)
    }
}

/// The design current of `power_w`, A.
pub fn design_current(power_w: f64, cos_phi: f64, three_phase: bool) -> f64 {
    let cos = if cos_phi > 0.0 && cos_phi <= 1.0 {
        cos_phi
    } else {
        0.8
    };
    if power_w <= 0.0 {
        0.0
    } else if three_phase {
        power_w / (3.0_f64.sqrt() * UZ * cos)
    } else {
        power_w / (U0 * cos)
    }
}

/// Check a circuit.
pub fn check(c: &Circuit) -> Check {
    let cos = if c.cos_phi > 0.0 && c.cos_phi <= 1.0 {
        c.cos_phi
    } else {
        0.8
    };
    let ib = design_current(c.power_w, cos, c.three_phase);
    let rho = if c.aluminium { RHO1_AL } else { RHO1_CU };
    let b = if c.three_phase { 1.0 } else { 2.0 };
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let s = c.s_mm2.max(0.1);
    let u = b * (rho * c.length_m / s * cos + LAMBDA * c.length_m * sin) * ib;
    let du = 100.0 * u / U0;
    let limit = du_limit(c.supply, c.lighting);
    let iz = (!c.aluminium)
        .then(|| iz_table(c.method, c.three_phase, c.s_mm2))
        .flatten();
    let zs = c.ze.map(|ze| ze + 2.0 * rho * c.length_m / s);
    let zs_max = U0 / (c.curve.trip() * c.in_a.max(0.1));
    Check {
        ib,
        u,
        du,
        du_limit: limit,
        ok_du: du <= limit + 1e-9,
        iz,
        ok_prot: iz.map(|iz| ib <= c.in_a + 1e-9 && c.in_a <= iz + 1e-9),
        zs,
        zs_max,
        ok_zs: zs.map(|z| z <= zs_max + 1e-9),
    }
}

/// The smallest preferred breaker (at least `min_in`: 10 A for lighting,
/// 16 A for sockets) and the smallest copper cable of the tables that
/// pass the check; none where even 25 mm² does not.
pub fn propose(c: &Circuit, min_in: f64) -> Option<Circuit> {
    let ib = design_current(c.power_w, c.cos_phi, c.three_phase);
    let in_a = next_std(&BREAKER_IN, ib.max(min_in))?;
    IZ_S.iter().find_map(|s| {
        let x = Circuit {
            in_a,
            s_mm2: *s,
            aluminium: false,
            ..*c
        };
        check(&x).ok().then_some(x)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables() {
        assert_eq!(iz_table(Method::C, false, 2.5), Some(27.0));
        assert_eq!(iz_table(Method::A2, true, 1.5), Some(13.0));
        assert_eq!(iz_table(Method::B2, false, 35.0), None);
        assert_eq!(next_std(&BREAKER_IN, 14.2), Some(16.0));
        assert_eq!(next_std(&BREAKER_IN, 16.0), Some(16.0));
        assert_eq!(next_std(&BREAKER_IN, 130.0), None);
        assert_eq!(du_limit(Supply::Public, true), 3.0);
        assert_eq!(du_limit(Supply::Own, false), 8.0);
    }

    #[test]
    fn a_socket_circuit() {
        // 2 kW, cos φ 0.8, 20 m of 2.5 mm² copper on B16
        let c = Circuit::default();
        let r = check(&c);
        assert!((r.ib - 2000.0 / (230.0 * 0.8)).abs() < 1e-9);
        let u = 2.0 * (0.0225 * 20.0 / 2.5 * 0.8 + 0.08e-3 * 20.0 * 0.6) * r.ib;
        assert!((r.u - u).abs() < 1e-9 && (r.du - 100.0 * u / 230.0).abs() < 1e-9);
        assert!(r.du > 1.3 && r.du < 1.5, "{}", r.du);
        assert!(r.ok_du && r.du_limit == 5.0);
        assert_eq!(r.iz, Some(18.5));
        assert_eq!(r.ok_prot, Some(true));
        assert_eq!(r.ok_zs, None);
        assert!(r.ok());
        // the breaker over the cable's capacity
        let over = check(&Circuit { in_a: 20.0, ..c });
        assert_eq!(over.ok_prot, Some(false));
        assert!(!over.ok());
        // a long thin lighting circuit drops too much
        let long = check(&Circuit {
            length_m: 80.0,
            s_mm2: 1.5,
            in_a: 10.0,
            lighting: true,
            ..c
        });
        assert!(!long.ok_du && long.du_limit == 3.0);
    }

    #[test]
    fn three_phases_and_the_fault_loop() {
        let c = Circuit {
            power_w: 11_000.0,
            cos_phi: 1.0,
            three_phase: true,
            length_m: 30.0,
            s_mm2: 4.0,
            in_a: 16.0,
            ze: Some(0.5),
            ..Circuit::default()
        };
        let r = check(&c);
        assert!((r.ib - 11_000.0 / (3.0_f64.sqrt() * 400.0)).abs() < 1e-9);
        assert!((r.zs.unwrap() - (0.5 + 2.0 * 0.0225 * 30.0 / 4.0)).abs() < 1e-12);
        assert!((r.zs_max - 230.0 / 80.0).abs() < 1e-12);
        assert_eq!(r.ok_zs, Some(true));
        let d = check(&Circuit {
            curve: Curve::D,
            in_a: 63.0,
            ze: Some(0.5),
            ..c
        });
        assert_eq!(d.ok_zs, Some(false));
    }

    #[test]
    fn the_smallest_breaker_and_cable_that_pass() {
        let c = Circuit {
            power_w: 3500.0,
            cos_phi: 1.0,
            length_m: 25.0,
            ..Circuit::default()
        };
        let p = propose(&c, 16.0).unwrap();
        assert_eq!(p.in_a, 16.0);
        assert_eq!(p.s_mm2, 2.5);
        assert!(check(&p).ok());
        // a longer run needs more copper for the voltage drop
        let far = propose(
            &Circuit {
                length_m: 90.0,
                ..c
            },
            16.0,
        )
        .unwrap();
        assert!(far.s_mm2 > 2.5 && check(&far).ok());
        // nothing on the tables carries 60 kW on one phase
        assert_eq!(
            propose(
                &Circuit {
                    power_w: 60_000.0,
                    ..c
                },
                16.0
            ),
            None
        );
    }
}
