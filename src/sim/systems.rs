//! Ship systems: reactor budget, power and coolant sliders, heat (T11) and damage/repair (T12).
//!
//! Idea ported from the power model of the reference game (`power-model.c`): a reactor produces a
//! fixed budget, systems request power, heat builds with power and is carried off by coolant.
// ref: REFERENCE_tobe_DELETED power-model.c (idea only)

use super::clock::DT;

/// Ship systems that can be damaged. The first six also take power and coolant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sys {
    Engines,
    Warp,
    Shields,
    Lances,
    Sensors,
    LifeSupport,
    Reactor,
}

pub const SYS_COUNT: usize = 7;
/// Systems with power/coolant sliders.
pub const POWERED: usize = 6;

impl Sys {
    pub const ALL: [Sys; SYS_COUNT] = [
        Sys::Engines,
        Sys::Warp,
        Sys::Shields,
        Sys::Lances,
        Sys::Sensors,
        Sys::LifeSupport,
        Sys::Reactor,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Sys::Engines => "ENGINES",
            Sys::Warp => "WARP",
            Sys::Shields => "SHIELDS",
            Sys::Lances => "LANCES",
            Sys::Sensors => "SENSORS",
            Sys::LifeSupport => "LIFE SUP",
            Sys::Reactor => "REACTOR",
        }
    }
}

/// Power presets (index `1..=3`): Balanced, Battle, Run. Values are fractions of nominal.
const PRESETS: [[f32; POWERED]; 3] = [
    [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    [1.0, 0.2, 1.6, 1.6, 0.8, 0.6],
    [1.8, 1.5, 0.8, 0.4, 0.6, 0.8],
];

#[derive(Debug, Clone)]
pub struct Systems {
    /// Requested power per powered system, `0.0..=2.0` (`1.0` = nominal).
    pub power: [f32; POWERED],
    /// Relative coolant request per powered system, `0.0..=1.0`.
    pub coolant: [f32; POWERED],
    /// Heat `0..=100`; above 100 the system burns.
    pub heat: [f32; SYS_COUNT],
    /// Health `0..=100`.
    pub health: [f32; SYS_COUNT],
    /// Repair team assignments.
    pub teams: [Option<Sys>; 3],
    pub crew: u32,
    pub crew_max: u32,
    /// Reactor upgrade level `1..=3`.
    pub reactor_level: u8,
    air_timer: f32,
}

impl Default for Systems {
    fn default() -> Self {
        Systems::new(24)
    }
}

impl Systems {
    pub fn new(crew: u32) -> Systems {
        Systems {
            power: PRESETS[0],
            coolant: [1.0; POWERED],
            heat: [0.0; SYS_COUNT],
            health: [100.0; SYS_COUNT],
            teams: [None; 3],
            crew,
            crew_max: crew,
            reactor_level: 1,
            air_timer: 0.0,
        }
    }

    pub fn preset(&mut self, n: u8) {
        if let Some(p) = PRESETS.get(n.saturating_sub(1) as usize) {
            self.power = *p;
        }
    }

    /// Reactor power budget (sum of nominal requests = 6.0 at full health, level 1).
    pub fn output(&self) -> f32 {
        let h = self.health[Sys::Reactor.idx()] / 100.0;
        6.0 * (0.35 + 0.65 * h) * (1.0 + 0.2 * (self.reactor_level as f32 - 1.0))
    }

    /// Sum of requested power.
    pub fn demand(&self) -> f32 {
        self.power.iter().sum()
    }

    fn scale(&self) -> f32 {
        let d = self.demand();
        if d > self.output() {
            self.output() / d
        } else {
            1.0
        }
    }

    /// Delivered power after the budget cut.
    pub fn delivered(&self, s: Sys) -> f32 {
        if s == Sys::Reactor {
            return 1.0;
        }
        self.power[s.idx()] * self.scale()
    }

    /// Performance factor of a system: power × health × overheating (`0.0..~2.0`, `1.0` nominal).
    pub fn eff(&self, s: Sys) -> f32 {
        let i = s.idx();
        let hf = (self.health[i] / 50.0).clamp(0.0, 1.0);
        let of = (1.0 - (self.heat[i] - 80.0) / 40.0).clamp(0.0, 1.0);
        if s == Sys::Reactor {
            return hf;
        }
        self.delivered(s) * hf * of
    }

    pub fn set_power(&mut self, s: Sys, v: f32) {
        if s.idx() < POWERED {
            self.power[s.idx()] = v.clamp(0.0, 2.0);
        }
    }

    pub fn set_coolant(&mut self, s: Sys, v: f32) {
        if s.idx() < POWERED {
            self.coolant[s.idx()] = v.clamp(0.0, 1.0);
        }
    }

    pub fn add_heat(&mut self, s: Sys, h: f32) {
        self.heat[s.idx()] = (self.heat[s.idx()] + h).clamp(0.0, 120.0);
    }

    /// Damage in percentage points of health.
    pub fn damage(&mut self, s: Sys, pct: f32) {
        let h = &mut self.health[s.idx()];
        *h = (*h - pct).max(0.0);
    }

    pub fn repair_all(&mut self) {
        self.health = [100.0; SYS_COUNT];
        self.heat = [0.0; SYS_COUNT];
        self.crew = self.crew_max;
    }

    /// Lowest health among all systems.
    pub fn worst(&self) -> f32 {
        self.health.iter().copied().fold(100.0, f32::min)
    }

    /// Advance heat, burning, repairs and life support by one fixed step.
    pub fn step(&mut self) {
        let sc = self.scale();
        let cool_total: f32 = self.coolant.iter().sum::<f32>().max(0.001);
        for i in 0..POWERED {
            let p = self.power[i] * sc;
            let gen_heat = p * p * 4.0;
            let share = self.coolant[i] / cool_total;
            let cooling = share * 27.0 + self.heat[i] * 0.02;
            self.heat[i] = (self.heat[i] + (gen_heat - cooling) * DT).clamp(0.0, 120.0);
            if self.heat[i] >= 100.0 {
                self.health[i] = (self.health[i] - 2.0 * DT).max(0.0);
            }
        }
        // Reactor runs hot only when systems are starved of coolant overall.
        self.heat[Sys::Reactor.idx()] = (self.heat[Sys::Reactor.idx()]
            + (self.demand().min(self.output() * 1.5) * 0.5 - 3.0) * DT)
            .clamp(0.0, 120.0);
        // Repair teams work at 1.5 %/s, scaled by surviving crew.
        let crew_f = (self.crew as f32 / self.crew_max.max(1) as f32).clamp(0.0, 1.0);
        for t in 0..3 {
            if let Some(s) = self.teams[t] {
                let h = &mut self.health[s.idx()];
                if *h < 100.0 {
                    *h = (*h + 1.5 * crew_f * DT).min(100.0);
                }
            }
        }
        // Life support failure kills the crew slowly.
        if self.eff(Sys::LifeSupport) < 0.15 && self.crew > 0 {
            self.air_timer += DT;
            if self.air_timer >= 5.0 {
                self.air_timer = 0.0;
                self.crew -= 1;
            }
        } else {
            self.air_timer = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_scales_requests_down() {
        let mut s = Systems::new(10);
        s.power = [2.0; POWERED];
        assert!(s.demand() > s.output());
        let total: f32 = Sys::ALL[..POWERED].iter().map(|&x| s.delivered(x)).sum();
        assert!((total - s.output()).abs() < 1e-3);
    }

    #[test]
    fn damaged_reactor_shrinks_budget() {
        let mut s = Systems::new(10);
        let full = s.output();
        s.damage(Sys::Reactor, 60.0);
        assert!(s.output() < full * 0.8);
    }

    #[test]
    fn overheating_burns_and_coolant_prevents_it() {
        let mut hot = Systems::new(10);
        hot.power = [1.0, 0.1, 1.9, 0.1, 0.1, 0.1];
        hot.coolant = [0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        for _ in 0..3600 {
            hot.step();
        }
        assert!(hot.health[Sys::Shields.idx()] < 100.0);
        let mut cool = Systems::new(10);
        cool.power = [1.0, 0.1, 1.9, 0.1, 0.1, 0.1];
        cool.coolant = [0.1, 0.1, 1.0, 0.1, 0.1, 0.1];
        for _ in 0..3600 {
            cool.step();
        }
        assert_eq!(cool.health[Sys::Shields.idx()], 100.0);
    }

    #[test]
    fn teams_repair_and_degraded_systems_perform_worse() {
        let mut s = Systems::new(10);
        s.damage(Sys::Engines, 70.0);
        let hurt = s.eff(Sys::Engines);
        assert!(hurt < 0.7);
        s.teams[0] = Some(Sys::Engines);
        for _ in 0..(60 * 20) {
            s.step();
        }
        assert!(s.health[Sys::Engines.idx()] > 50.0);
        assert!(s.eff(Sys::Engines) > hurt);
        s.damage(Sys::Engines, 100.0);
        assert_eq!(s.eff(Sys::Engines), 0.0);
    }

    #[test]
    fn life_support_failure_costs_crew() {
        let mut s = Systems::new(5);
        s.damage(Sys::LifeSupport, 100.0);
        for _ in 0..(60 * 12) {
            s.step();
        }
        assert!(s.crew < 5);
    }
}
