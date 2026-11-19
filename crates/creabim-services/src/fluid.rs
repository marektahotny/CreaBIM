//! Water's density and kinematic viscosity by temperature.

/// A fluid at one temperature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fluid {
    pub temp_c: f64,
    /// Density, kg/m³.
    pub rho: f64,
    /// Kinematic viscosity, m²/s.
    pub nu: f64,
    /// Specific heat, J/(kg·K).
    pub cp: f64,
}

/// Temperature °C, density kg/m³, kinematic viscosity m²/s.
const TABLE: [[f64; 3]; 13] = [
    [0.0, 999.8, 1.792e-6],
    [5.0, 1000.0, 1.519e-6],
    [10.0, 999.7, 1.306e-6],
    [15.0, 999.1, 1.139e-6],
    [20.0, 998.2, 1.004e-6],
    [30.0, 995.7, 8.01e-7],
    [40.0, 992.2, 6.58e-7],
    [50.0, 988.0, 5.53e-7],
    [55.0, 985.7, 5.12e-7],
    [60.0, 983.2, 4.74e-7],
    [70.0, 977.8, 4.13e-7],
    [80.0, 971.8, 3.65e-7],
    [90.0, 965.3, 3.26e-7],
];

impl Fluid {
    /// Water at `temp_c`, interpolated in the table (held at its ends).
    pub fn water(temp_c: f64) -> Fluid {
        let n = TABLE.len();
        let t = temp_c.clamp(TABLE[0][0], TABLE[n - 1][0]);
        let (rho, nu) = TABLE
            .windows(2)
            .find(|w| t <= w[1][0])
            .map(|w| {
                let f = (t - w[0][0]) / (w[1][0] - w[0][0]);
                (
                    w[0][1] + f * (w[1][1] - w[0][1]),
                    w[0][2] + f * (w[1][2] - w[0][2]),
                )
            })
            .unwrap_or((TABLE[n - 1][1], TABLE[n - 1][2]));
        Fluid {
            temp_c,
            rho,
            nu,
            cp: 4186.0,
        }
    }

    /// The flow in l/s that carries `power_w` at a temperature drop `dt`.
    pub fn flow_for_power(&self, power_w: f64, dt: f64) -> f64 {
        if dt <= 0.0 {
            0.0
        } else {
            power_w / (self.rho * self.cp * dt) * 1000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_is_read_from_the_table_and_between_its_rows() {
        let w = Fluid::water(10.0);
        assert_eq!((w.rho, w.nu), (999.7, 1.306e-6));
        let w = Fluid::water(57.5);
        assert!((w.rho - (985.7 + 983.2) / 2.0).abs() < 1e-9);
        // held at the ends
        assert_eq!(Fluid::water(120.0).rho, 965.3);
        assert_eq!(Fluid::water(-5.0).rho, 999.8);
    }
}
