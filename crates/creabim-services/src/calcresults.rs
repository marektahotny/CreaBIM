//! Each calculation's findings as values on the elements they are about
//! ([`creabim_core::results`]): heat losses, floor heating, radiators,
//! cooling and air on the rooms; flows, velocities, pressure losses and
//! proposed sizes on the pipes and ducts; what the radiators, appliances,
//! hydrants, fixtures and outlets get; the checks of the circuits.
//!
//! The keys are the same in every calculation for the same quantity
//! ("flow", "velocity", "loss"...), so the IFC export puts them in the
//! standard property sets where IFC has one.

use crate::{
    airbalance, circuits, circulation, coolingload, ductsizing, firewater, floorheat, gas, heating,
    heatloss, medgas, model::ModelNetwork, radiators, sanitary, storm, WaterResult,
};
use creabim_core::results::{CalcValue as V, ResultSet, Unit};

/// The keys the IFC export knows: a room's.
pub mod key {
    pub const TI: &str = "ti";
    pub const PHI_T: &str = "phi_t";
    pub const PHI_V: &str = "phi_v";
    pub const HEAT_LOSS: &str = "phi_hl";
    pub const AREA: &str = "area";
    pub const VOLUME: &str = "volume";
    pub const AIR_MIN: &str = "v_min";
    pub const AIR_INF: &str = "v_inf";
    pub const COOL_LOAD: &str = "phi_c";
    pub const AIR_NEED: &str = "air_need";
    pub const AIR_SUPPLY: &str = "air_supply";
    pub const AIR_EXHAUST: &str = "air_exhaust";
    // a pipe's or duct's
    pub const FLOW: &str = "flow";
    pub const VELOCITY: &str = "velocity";
    pub const R: &str = "r";
    pub const LOSS: &str = "loss";
    pub const DN_PROPOSED: &str = "dn_proposed";
    pub const CRITICAL: &str = "critical";
    pub const SLOPE: &str = "slope";
    pub const FILL: &str = "fill";
    // a device's
    pub const POWER: &str = "power";
    pub const PRESSURE: &str = "pressure";
    pub const KV: &str = "kv";
    // a circuit's
    pub const CURRENT: &str = "ib";
    pub const DU: &str = "du";
}

fn critical(v: bool) -> V {
    V::flag(key::CRITICAL, "Na kritickej vetve", v)
}

fn proposed(mm: f64, label: &str) -> V {
    if label.is_empty() {
        V::num(key::DN_PROPOSED, "Navrhnutá dimenzia", mm, Unit::Mm)
    } else {
        V::text(key::DN_PROPOSED, "Navrhnutá dimenzia", label)
    }
}

/// The heat loss of each room (STN EN 12831-1).
pub fn heat_loss(b: &heatloss::BuildingLoss, o: &heatloss::Opts) -> ResultSet {
    let mut s = ResultSet::new("heatloss", "Tepelné straty", "STN EN 12831-1");
    s.put_system(
        "",
        vec![
            V::num(key::HEAT_LOSS, "Tepelná strata budovy ΦHL", b.phi, Unit::W),
            V::num(key::AREA, "Vykurovaná plocha", b.area, Unit::M2),
            V::num("te", "Vonkajšia výpočtová teplota θe", o.te, Unit::C),
        ],
    );
    for r in &b.rooms {
        s.put(
            r.id,
            vec![
                V::num(key::TI, "Vnútorná teplota θint", r.ti, Unit::C),
                V::num("te", "Vonkajšia výpočtová teplota θe", o.te, Unit::C),
                V::num(key::PHI_T, "Tepelná strata prechodom ΦT", r.phi_t, Unit::W),
                V::num(key::PHI_V, "Tepelná strata vetraním ΦV", r.phi_v, Unit::W),
                V::num(key::HEAT_LOSS, "Celková tepelná strata ΦHL", r.phi, Unit::W),
                V::num(
                    "phi_m2",
                    "Tepelná strata na m²",
                    if r.area > 0.0 { r.phi / r.area } else { 0.0 },
                    Unit::Wm2,
                ),
                V::num(key::AREA, "Podlahová plocha", r.area, Unit::M2),
                V::num(key::VOLUME, "Objem", r.volume, Unit::M3),
                V::num(key::AIR_MIN, "Minimálne vetranie Vmin", r.v_min, Unit::M3h),
                V::num(key::AIR_INF, "Infiltrácia Vinf", r.v_inf, Unit::M3h),
            ],
        );
    }
    s
}

/// The floor heating of each room (STN EN 1264).
pub fn floor_heat(d: &floorheat::Design) -> ResultSet {
    let mut s = ResultSet::new("floorheat", "Podlahové vykurovanie", "STN EN 1264");
    s.put_system(
        "",
        vec![
            V::num(
                key::POWER,
                "Výkon podlahového vykurovania",
                d.output_kw,
                Unit::KW,
            ),
            V::num(key::FLOW, "Prietok", d.flow_ls, Unit::Ls),
            V::num("loops", "Počet okruhov", d.loops as f64, Unit::None),
            V::num("head", "Potrebný dopravný tlak", d.head_kpa, Unit::KPa),
        ],
    );
    for r in d.rooms.iter().filter(|r| r.on) {
        s.put(
            r.id,
            vec![
                V::num("ufh_q", "Merný tepelný tok q", r.q, Unit::Wm2),
                V::num("ufh_q_req", "Potrebný tepelný tok", r.q_req, Unit::Wm2),
                V::num("ufh_spacing", "Rozostup rúr", r.spacing * 1000.0, Unit::Mm),
                V::num("ufh_area", "Plocha s rúrami", r.area, Unit::M2),
                V::num(key::POWER, "Výkon podlahy", r.covered, Unit::W),
                V::num("ufh_loops", "Počet okruhov", r.loops as f64, Unit::None),
                V::num("ufh_loop_len", "Dĺžka okruhu", r.loop_len, Unit::M),
                V::num(key::FLOW, "Prietok", r.flow_ls, Unit::Ls),
                V::num(key::LOSS, "Tlaková strata okruhu", r.dp_kpa, Unit::KPa),
                V::flag("ok", "Pokrytie straty", r.ok),
            ],
        );
    }
    s
}

/// The radiators each room needs and has.
pub fn radiators(d: &radiators::Design) -> ResultSet {
    let mut s = ResultSet::new("radiators", "Návrh vykurovacích telies", "STN EN 442");
    for r in &d.rooms {
        let mut v = vec![
            V::num("rad_need", "Potrebný výkon telies", r.need, Unit::W),
            V::num("rad_count", "Počet telies", r.count as f64, Unit::None),
            V::num("rad_output", "Výkon navrhnutých telies", r.output, Unit::W),
            V::num(key::FLOW, "Prietok", r.flow, Unit::Kgh),
            V::num(
                "rad_installed",
                "Výkon osadených telies",
                r.installed_w,
                Unit::W,
            ),
            V::flag("ok", "Pokrytie straty", r.ok),
        ];
        if let Some(p) = &r.panel {
            v.insert(2, V::text("rad_panel", "Navrhnuté teleso", p.label()));
        }
        s.put(r.id, v);
    }
    s
}

/// The cooling load of each room (STN 73 0548).
pub fn cooling(b: &coolingload::BuildingLoad) -> ResultSet {
    let mut s = ResultSet::new("cooling", "Tepelná záťaž", "STN 73 0548");
    s.put_system(
        "",
        vec![
            V::num(key::COOL_LOAD, "Tepelná záťaž budovy", b.phi, Unit::W),
            V::num(key::FLOW, "Prietok chladiacej vody", b.flow_ls, Unit::Ls),
        ],
    );
    for r in b.rooms.iter().filter(|r| r.cooled) {
        let mut v = vec![
            V::num(key::COOL_LOAD, "Tepelná záťaž Φc", r.phi, Unit::W),
            V::num("fancoil", "Výkon fancoilu", r.fancoil_kw, Unit::KW),
        ];
        if let Some(h) = r.hour {
            v.push(V::num("hour", "Hodina maxima", h as f64, Unit::None));
        }
        s.put(r.id, v);
    }
    s
}

/// The air each room needs and gets.
pub fn air(b: &airbalance::Balance) -> ResultSet {
    let mut s = ResultSet::new("air", "Bilancia vzduchu", "STN EN 16798-1");
    s.put_system(
        "",
        vec![
            V::num(key::AIR_SUPPLY, "Privádzaný vzduch", b.supply, Unit::M3h),
            V::num(key::AIR_EXHAUST, "Odvádzaný vzduch", b.exhaust, Unit::M3h),
            V::num("need_supply", "Potreba prívodu", b.need_supply, Unit::M3h),
            V::num("need_exhaust", "Potreba odvodu", b.need_exhaust, Unit::M3h),
        ],
    );
    for r in &b.rooms {
        s.put(
            r.id,
            vec![
                V::num(key::AIR_NEED, "Potrebný vzduch", r.need, Unit::M3h),
                V::num(key::AIR_SUPPLY, "Privádzaný vzduch", r.supply, Unit::M3h),
                V::num(key::AIR_EXHAUST, "Odvádzaný vzduch", r.exhaust, Unit::M3h),
                V::num(
                    "air_changes",
                    "Intenzita výmeny",
                    if r.volume > 0.0 {
                        r.supply.max(r.exhaust) / r.volume
                    } else {
                        0.0
                    },
                    Unit::PerH,
                ),
                V::flag("ok", "Potreba pokrytá", r.ok),
            ],
        );
    }
    s
}

/// The ducts of system `code`, sized.
pub fn ducts(set: &mut ResultSet, code: &str, rows: &[ductsizing::Row]) {
    let most = rows.iter().map(|r| r.flow_m3h).fold(0.0, f64::max);
    set.put_system(
        code,
        vec![V::num(key::FLOW, "Prietok vzduchu", most, Unit::M3h)],
    );
    for r in rows {
        let mut v = vec![
            V::num(key::FLOW, "Prietok vzduchu", r.flow_m3h, Unit::M3h),
            V::num(key::VELOCITY, "Rýchlosť", r.v_drawn, Unit::Ms),
            V::num("v_limit", "Najvyššia rýchlosť", r.limit, Unit::Ms),
        ];
        if let Some(mm) = r.proposed_mm {
            v.push(proposed(mm, ""));
            v.push(V::num(
                "velocity_proposed",
                "Rýchlosť pri návrhu",
                r.v,
                Unit::Ms,
            ));
            v.push(V::num(
                key::R,
                "Merná tlaková strata R",
                r.r_pa_m,
                Unit::PaM,
            ));
            v.push(V::num(
                key::LOSS,
                "Tlaková strata",
                r.r_pa_m * r.length,
                Unit::Pa,
            ));
        }
        set.put(r.id, v);
    }
}

/// The cold or hot water system `code` (STN EN 806-3, STN 73 6655).
pub fn water(set: &mut ResultSet, code: &str, net: &ModelNetwork, result: &WaterResult) {
    let mut sys = vec![
        V::num(
            key::FLOW,
            "Výpočtový prietok",
            result.root_flow_ls,
            Unit::Ls,
        ),
        V::num(
            key::PRESSURE,
            "Potrebný pretlak",
            result.required_pa / 1000.0,
            Unit::KPa,
        ),
        V::num(
            "available",
            "Dostupný pretlak",
            result.available_pa / 1000.0,
            Unit::KPa,
        ),
        V::num("reserve", "Rezerva", result.reserve_pa / 1000.0, Unit::KPa),
        V::num(key::TI, "Teplota vody", result.fluid_temp_c, Unit::C),
    ];
    if let Some(r) = result.reducer_setting_kpa {
        sys.push(V::num(
            "reducer",
            "Nastavenie redukčného ventilu",
            r,
            Unit::KPa,
        ));
    }
    set.put_system(code, sys);
    let mut loss: Vec<(creabim_core::ElementId, f64)> = vec![];
    for (i, n) in net.net.nodes.iter().enumerate() {
        if let Some(id) = net.pipes[i] {
            let pa = n.linear_loss_pa + n.local_loss_pa + n.fixed_loss_pa;
            match loss.iter_mut().find(|(x, _)| *x == id) {
                Some((_, l)) => *l += pa,
                None => loss.push((id, pa)),
            }
        }
        if let Some(id) = net.fixtures[i] {
            set.put(
                id,
                vec![
                    V::num(key::FLOW, "Výpočtový prietok", n.flow_ls, Unit::Ls),
                    V::num(
                        key::PRESSURE,
                        "Potrebný pretlak",
                        n.leaf_required_pa / 1000.0,
                        Unit::KPa,
                    ),
                    critical(n.on_critical_path),
                ],
            );
        }
    }
    for (id, r) in crate::model::pipe_results(net) {
        let mut v = vec![
            V::num(key::FLOW, "Výpočtový prietok", r.flow_ls, Unit::Ls),
            V::num(key::VELOCITY, "Rýchlosť", r.velocity_ms, Unit::Ms),
            V::num(key::R, "Merná tlaková strata R", r.r_pa_per_m, Unit::PaM),
        ];
        if let Some((_, l)) = loss.iter().find(|(x, _)| *x == id) {
            v.push(V::num(key::LOSS, "Tlaková strata", *l, Unit::Pa));
        }
        if let Some(p) = r.proposed {
            v.push(proposed(p.nominal_mm, &p.label()));
        }
        v.push(critical(r.on_critical_path));
        set.put(id, v);
    }
}

/// The hot water circulation.
pub fn circulation(r: &circulation::CircResult) -> ResultSet {
    let mut s = ResultSet::new("circulation", "Cirkulácia TV", "STN EN 806-3");
    s.put_system(
        "C",
        vec![
            V::num(key::FLOW, "Cirkulačný prietok", r.flow_ls, Unit::Ls),
            V::num("heat_loss", "Tepelná strata rozvodov", r.loss_w, Unit::W),
            V::num("pump", "Dopravná výška čerpadla", r.pump_kpa, Unit::KPa),
        ],
    );
    for p in &r.pipes {
        s.put(
            p.id,
            vec![
                V::num(key::FLOW, "Cirkulačný prietok", p.flow_ls, Unit::Ls),
                V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms),
                V::num(key::R, "Merná tlaková strata R", p.r_pa_m, Unit::PaM),
                proposed(p.proposed_mm, &p.proposed),
            ],
        );
    }
    for l in &r.loops {
        s.put(
            l.circ_pipe,
            vec![
                V::num("valve_dp", "Tlak na vyregulovanie", l.valve_kpa, Unit::KPa),
                V::num(key::KV, "kv ventilu", l.kv, Unit::M3h),
            ],
        );
    }
    s
}

/// The drainage (STN EN 12056-2).
pub fn drain(r: &sanitary::DrainResult) -> ResultSet {
    let mut s = ResultSet::new("drain", "Kanalizácia", "STN EN 12056-2");
    s.put_system(
        "K",
        vec![
            V::num(key::FLOW, "Odtok Qww", r.flow_ls, Unit::Ls),
            V::num(
                "sum_du",
                "Súčet výpočtových odtokov ΣDU",
                r.sum_du,
                Unit::Ls,
            ),
            V::num("outlet_dn", "Dimenzia prípojky", r.outlet_dn, Unit::Mm),
            V::num(
                key::FILL,
                "Plnenie prípojky",
                r.outlet_fill * 100.0,
                Unit::Pct,
            ),
        ],
    );
    for p in &r.pipes {
        let mut v = vec![
            V::num(key::FLOW, "Odtok Qww", p.flow_ls, Unit::Ls),
            V::num(
                "sum_du",
                "Súčet výpočtových odtokov ΣDU",
                p.sum_du,
                Unit::Ls,
            ),
            V::num("fixtures", "Počet predmetov", p.fixtures as f64, Unit::None),
            proposed(p.proposed_mm, &p.proposed),
            V::num("capacity", "Kapacita", p.capacity_ls, Unit::Ls),
        ];
        if !p.stack {
            v.push(V::num(key::SLOPE, "Sklon", p.slope * 100.0, Unit::Pct));
            v.push(V::num(key::FILL, "Plnenie h/d", p.fill * 100.0, Unit::Pct));
            v.push(V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms));
        }
        if let Some((db, limit)) = p.noise_db {
            v.push(V::num("noise", "Hluk stúpačky", db, Unit::DbA));
            v.push(V::num("noise_limit", "Limit hluku", limit, Unit::DbA));
        }
        s.put(p.id, v);
    }
    s
}

/// The storm water (STN EN 12056-3).
pub fn storm(r: &storm::StormResult) -> ResultSet {
    let mut s = ResultSet::new("storm", "Dažďová kanalizácia", "STN EN 12056-3");
    s.put_system(
        "D",
        vec![
            V::num(key::AREA, "Odvodňovaná plocha", r.area_m2, Unit::M2),
            V::num(key::FLOW, "Odtok Q", r.flow_ls, Unit::Ls),
        ],
    );
    for d in &r.drains {
        s.put(
            d.id,
            vec![
                V::num(key::AREA, "Odvodňovaná plocha", d.area_m2, Unit::M2),
                V::num("coef", "Súčiniteľ odtoku C", d.coef, Unit::None),
                V::num(key::FLOW, "Odtok Q", d.flow_ls, Unit::Ls),
            ],
        );
    }
    for p in &r.pipes {
        let mut v = vec![
            V::num(key::FLOW, "Odtok Q", p.flow_ls, Unit::Ls),
            proposed(p.proposed_mm, ""),
            V::num("capacity", "Kapacita", p.capacity_ls, Unit::Ls),
        ];
        if !p.vertical {
            v.push(V::num(key::SLOPE, "Sklon", p.slope * 100.0, Unit::Pct));
            v.push(V::num(key::FILL, "Plnenie h/d", p.fill * 100.0, Unit::Pct));
            v.push(V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms));
        }
        s.put(p.id, v);
    }
    s
}

/// The fire main (STN 92 0400).
pub fn fire(r: &firewater::FireResult) -> ResultSet {
    let mut s = ResultSet::new("fire", "Požiarny vodovod", "STN 92 0400");
    s.put_system(
        "PV",
        vec![
            V::num(key::FLOW, "Prietok", r.flow_ls, Unit::Ls),
            V::num(key::PRESSURE, "Potrebný pretlak", r.required_kpa, Unit::KPa),
        ],
    );
    for h in &r.hydrants {
        s.put(
            h.id,
            vec![
                V::num(key::FLOW, "Prietok", h.flow_ls, Unit::Ls),
                V::num(key::PRESSURE, "Pretlak na vstupe", h.inlet_kpa, Unit::KPa),
                critical(h.critical),
            ],
        );
    }
    for p in &r.pipes {
        s.put(
            p.id,
            vec![
                V::num(key::FLOW, "Prietok", p.flow_ls, Unit::Ls),
                V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms),
                V::num(key::R, "Merná tlaková strata R", p.r_pa_m, Unit::PaM),
                V::num(key::LOSS, "Tlaková strata", p.loss_pa, Unit::Pa),
                proposed(p.proposed_mm, ""),
                critical(p.critical),
            ],
        );
    }
    s
}

/// The gas installation (STN EN 1775, TPP 704 01).
pub fn gas(r: &gas::GasResult) -> ResultSet {
    let mut s = ResultSet::new("gas", "Plynoinštalácia", "STN EN 1775, TPP 704 01");
    let mut sys = vec![
        V::num(key::POWER, "Inštalovaný príkon", r.installed_kw, Unit::KW),
        V::num("installed", "Inštalovaný odber", r.installed_m3h, Unit::M3h),
        V::num(key::FLOW, "Redukovaný odber Vr", r.vr_m3h, Unit::M3h),
        V::num(key::LOSS, "Tlaková strata", r.loss_pa, Unit::Pa),
        V::num("allowed", "Dovolená tlaková strata", r.allowed_pa, Unit::Pa),
        V::num(
            key::PRESSURE,
            "Pretlak pred najvzdialenejším spotrebičom",
            r.pressure_kpa,
            Unit::KPa,
        ),
    ];
    if let Some((m, _)) = r.meter {
        sys.push(V::text("meter", "Plynomer", m));
    }
    s.put_system("ZP", sys);
    for a in &r.appliances {
        s.put(
            a.id,
            vec![
                V::num(key::POWER, "Menovitý príkon", a.kw, Unit::KW),
                V::num(key::FLOW, "Odber plynu", a.flow_m3h, Unit::M3h),
                V::num(
                    key::PRESSURE,
                    "Pretlak pred spotrebičom",
                    a.pressure_kpa,
                    Unit::KPa,
                ),
                V::num(
                    key::LOSS,
                    "Tlaková strata k spotrebiču",
                    a.loss_pa,
                    Unit::Pa,
                ),
                V::text("gas_cat", "Kategória", a.kind),
                critical(a.critical),
            ],
        );
    }
    for p in &r.pipes {
        s.put(
            p.id,
            vec![
                V::num(key::FLOW, "Redukovaný odber Vr", p.vr_m3h, Unit::M3h),
                V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms),
                V::num(key::R, "Merná tlaková strata R", p.r_pa_m, Unit::PaM),
                V::num(key::LOSS, "Tlaková strata", p.loss_pa, Unit::Pa),
                proposed(p.proposed_mm, &p.proposed),
                critical(p.critical),
            ],
        );
    }
    s
}

/// The heating pipework (STN EN 12828).
pub fn heating(r: &heating::HeatResult) -> ResultSet {
    let mut s = ResultSet::new("heating", "Ústredné vykurovanie", "STN EN 12828");
    s.put_system(
        "UKP",
        vec![
            V::num(key::POWER, "Výkon", r.power_w, Unit::W),
            V::num(key::FLOW, "Prietok", r.flow_ls, Unit::Ls),
            V::num(
                key::LOSS,
                "Strata kritického okruhu",
                r.critical_pa / 1000.0,
                Unit::KPa,
            ),
            V::num("pump", "Dopravný tlak čerpadla", r.pump_kpa, Unit::KPa),
            V::num("pump_m", "Dopravná výška čerpadla", r.pump_m, Unit::M),
        ],
    );
    for t in &r.radiators {
        s.put(
            t.id,
            vec![
                V::num(key::POWER, "Výkon", t.power_w, Unit::W),
                V::num(key::FLOW, "Prietok", t.flow_ls, Unit::Ls),
                V::num(
                    "circuit_dp",
                    "Tlaková strata okruhu",
                    t.circuit_pa / 1000.0,
                    Unit::KPa,
                ),
                V::num(
                    "throttle",
                    "Tlak na vyregulovanie",
                    t.throttle_pa / 1000.0,
                    Unit::KPa,
                ),
                critical(t.critical),
            ],
        );
    }
    for p in &r.pipes {
        s.put(
            p.id,
            vec![
                V::num(key::FLOW, "Prietok", p.flow_ls, Unit::Ls),
                V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms),
                V::num(key::R, "Merná tlaková strata R", p.r_pa_m, Unit::PaM),
                V::num(key::LOSS, "Tlaková strata", p.loss_pa, Unit::Pa),
                proposed(p.proposed_mm, &p.proposed),
                critical(p.critical),
            ],
        );
    }
    for v in &r.presets {
        s.put(
            v.valve,
            vec![
                V::num(key::FLOW, "Prietok", v.flow_m3h, Unit::M3h),
                V::num(key::KV, "Potrebné kv", v.kv_req, Unit::M3h),
                V::text("preset", "Prednastavenie", v.preset),
                V::text("product", "Výrobok", v.product),
            ],
        );
    }
    s
}

/// The medical gases (STN EN ISO 7396-1).
pub fn med(rs: &[medgas::MedResult]) -> ResultSet {
    let mut s = ResultSet::new("medgas", "Medicinálne plyny", "STN EN ISO 7396-1");
    for r in rs {
        let gas = r.gas.code();
        s.put_system(
            gas,
            vec![
                V::num(key::FLOW, "Súčasný prietok", r.flow_lmin, Unit::Lmin),
                V::num(key::LOSS, "Tlaková strata", r.loss_pa, Unit::Pa),
                V::num("allowed", "Dovolená tlaková strata", r.limit_pa, Unit::Pa),
                V::num(key::PRESSURE, "Pretlak zdroja", r.source_kpa, Unit::KPa),
                V::num("outlet_p", "Pretlak na vývode", r.outlet_kpa, Unit::KPa),
            ],
        );
        for o in &r.outlets {
            s.put(
                o.id,
                vec![
                    V::text("gas", "Plyn", gas),
                    V::num(key::LOSS, "Tlaková strata k vývodu", o.loss_pa, Unit::Pa),
                    critical(o.critical),
                ],
            );
        }
        for p in &r.pipes {
            s.put(
                p.id,
                vec![
                    V::num(key::FLOW, "Prietok", p.flow_lmin, Unit::Lmin),
                    V::num("outlets", "Počet vývodov", p.outlets as f64, Unit::None),
                    V::num(key::VELOCITY, "Rýchlosť", p.velocity_ms, Unit::Ms),
                    V::num(key::R, "Merná tlaková strata R", p.r_pa_m, Unit::PaM),
                    proposed(p.proposed_mm, &p.proposed),
                    critical(p.critical),
                ],
            );
        }
    }
    s
}

/// The electrical circuits (STN 33 2000-5-52, -4-41).
pub fn circuits(rs: &[circuits::CircuitResult]) -> ResultSet {
    let mut s = ResultSet::new("circuits", "Elektrické obvody", "STN 33 2000");
    for c in rs {
        let mut v = vec![
            V::num(key::POWER, "Inštalovaný výkon", c.power_w, Unit::W),
            V::num(key::CURRENT, "Výpočtový prúd Ib", c.ib, Unit::A),
            V::num("length", "Dĺžka vedenia", c.length, Unit::M),
            V::num("devices", "Počet zariadení", c.devices as f64, Unit::None),
        ];
        if let Some(p) = c.phase {
            v.push(V::text("phase", "Fáza", format!("L{p}")));
        }
        if let Some(k) = &c.check {
            v.push(V::num(key::DU, "Úbytok napätia", k.du, Unit::Pct));
            v.push(V::num("du_limit", "Dovolený úbytok", k.du_limit, Unit::Pct));
            if let Some(iz) = k.iz {
                v.push(V::num("iz", "Dovolený prúd Iz", iz, Unit::A));
            }
            if let Some(zs) = k.zs {
                v.push(V::num("zs", "Impedancia slučky Zs [Ω]", zs, Unit::None));
            }
            v.push(V::flag("ok", "Vyhovuje", k.ok()));
        }
        if let Some(r) = c.rcd_required {
            v.push(V::text("rcd", "Prúdový chránič", r));
        }
        s.put(c.id, v);
    }
    s
}
