//! Environnement global : horloge jour/nuit + météo (pur, sans crossterm).
//!
//! Créé par la scène, posé sur `world.env`, tické une fois par frame dans
//! `spawn::step`. Les spawners lisent `world.env` pour décider quoi faire
//! apparaître. Le moteur reste agnostique de la météo.

use crate::engine::EnvSnapshot;
use rand::distributions::{Distribution, WeightedIndex};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// États de la machine météo.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Weather {
    Clear,
    Wind,
    Rain,
    Storm,
}

const WEATHER_STATES: [Weather; 4] =
    [Weather::Clear, Weather::Wind, Weather::Rain, Weather::Storm];
const WEATHER_WEIGHTS: [u32; 4] = [6, 2, 2, 1]; // CLEAR domine => calme par défaut
const DWELL_MIN: f64 = 8.0;
const DWELL_MAX: f64 = 20.0;
pub const WIND_SLANT: f64 = 6.0; // magnitude du dx (pluie/vent) quand il vente

/// Horloge jour/nuit + machine météo. `rng` est injectable pour des tests déterministes.
pub struct Environment {
    pub day_length: f64, // secondes pour un cycle jour+nuit complet
    pub(crate) rng: StdRng,
    pub t: f64,
    pub weather: Weather,
    pub wind_direction: i32,
    pub weather_timer: f64,
}

impl Environment {
    pub fn new(day_length: f64, mut rng: StdRng) -> Self {
        let wind_direction = if rng.gen::<bool>() { 1 } else { -1 };
        let weather_timer = rng.gen_range(DWELL_MIN..DWELL_MAX);
        Environment {
            day_length,
            rng,
            t: 0.0,
            weather: Weather::Clear,
            wind_direction,
            weather_timer,
        }
    }

    pub fn seeded(day_length: f64, seed: u64) -> Self {
        Environment::new(day_length, StdRng::seed_from_u64(seed))
    }

    /// Avance l'horloge et le minuteur météo.
    pub fn update(&mut self, dt: f64) {
        self.t += dt;
        self.weather_timer -= dt;
        if self.weather_timer <= 0.0 {
            self.next_weather();
        }
    }

    fn next_weather(&mut self) {
        let dist = WeightedIndex::new(WEATHER_WEIGHTS).unwrap();
        self.weather = WEATHER_STATES[dist.sample(&mut self.rng)];
        self.wind_direction = if self.rng.gen::<bool>() { 1 } else { -1 };
        self.weather_timer = self.rng.gen_range(DWELL_MIN..DWELL_MAX);
    }

    // --- heure du jour ---
    pub fn phase(&self) -> f64 {
        (self.t / self.day_length).rem_euclid(1.0)
    }
    pub fn is_night(&self) -> bool {
        self.phase() >= 0.5 // jour = première moitié
    }

    // --- requêtes météo ---
    pub fn raining(&self) -> bool {
        matches!(self.weather, Weather::Rain | Weather::Storm)
    }
    pub fn windy(&self) -> bool {
        matches!(self.weather, Weather::Wind | Weather::Storm)
    }
    pub fn storming(&self) -> bool {
        self.weather == Weather::Storm
    }
    pub fn wind_dx(&self) -> f64 {
        if self.windy() {
            self.wind_direction as f64 * WIND_SLANT
        } else {
            0.0
        }
    }

    pub fn snapshot(&self) -> EnvSnapshot {
        EnvSnapshot {
            is_day: !self.is_night(),
            is_night: self.is_night(),
            raining: self.raining(),
            windy: self.windy(),
            storming: self.storming(),
            wind_dx: self.wind_dx(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn phase_wraps_within_unit_interval() {
        let mut env = Environment::seeded(100.0, 0);
        env.update(30.0);
        assert!(env.phase() >= 0.0 && env.phase() < 1.0);
        assert!((env.phase() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn phase_returns_to_start_after_full_day() {
        let mut env = Environment::seeded(50.0, 0);
        env.update(50.0);
        assert!((env.phase() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn is_night_false_in_first_half() {
        let mut env = Environment::seeded(100.0, 0);
        env.update(10.0);
        assert!(!env.is_night());
    }

    #[test]
    fn is_night_true_at_half_phase_boundary() {
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0);
        assert!(env.is_night());
    }

    #[test]
    fn seeded_rng_gives_deterministic_weather_sequence() {
        fn run() -> Vec<Weather> {
            let mut env = Environment::seeded(1000.0, 1234);
            let mut seq = Vec::new();
            for _ in 0..500 {
                env.update(1.0);
                seq.push(env.weather);
            }
            seq
        }
        assert_eq!(run(), run());
    }

    #[test]
    fn weather_changes_over_time() {
        let mut env = Environment::seeded(10000.0, 1234);
        let mut seen: HashSet<Weather> = HashSet::new();
        for _ in 0..3000 {
            env.update(1.0);
            seen.insert(env.weather);
        }
        assert!(seen.len() > 1);
    }

    #[test]
    fn clear_has_no_weather_effects() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Clear;
        assert!(!env.raining());
        assert!(!env.windy());
        assert!(!env.storming());
        assert_eq!(env.wind_dx(), 0.0);
    }

    #[test]
    fn rain_is_raining_only() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Rain;
        assert!(env.raining());
        assert!(!env.windy());
        assert!(!env.storming());
    }

    #[test]
    fn wind_is_windy_only() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Wind;
        assert!(env.windy());
        assert!(!env.raining());
        assert!(!env.storming());
    }

    #[test]
    fn storm_implies_rain_wind_and_lightning() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Storm;
        assert!(env.raining());
        assert!(env.windy());
        assert!(env.storming());
    }

    #[test]
    fn wind_dx_zero_when_calm_signed_when_windy() {
        let mut env = Environment::seeded(90.0, 0);
        env.weather = Weather::Clear;
        assert_eq!(env.wind_dx(), 0.0);
        env.weather = Weather::Wind;
        assert_ne!(env.wind_dx(), 0.0);
        env.wind_direction = 1;
        assert!(env.wind_dx() > 0.0);
        env.wind_direction = -1;
        assert!(env.wind_dx() < 0.0);
    }
}
