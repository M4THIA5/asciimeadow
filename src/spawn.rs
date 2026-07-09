//! Gestion de population : maintient un nombre cible d'entités par type.

use crate::engine::{Entity, World};
use rand::Rng;

/// Cible d'une spec : nombre fixe ou fonction dynamique gatée sur `world.env`.
pub enum Target {
    Fixed(i32),
    Dynamic(fn(&World) -> i32),
}

impl Target {
    pub fn resolve(&self, world: &World) -> i32 {
        match self {
            Target::Fixed(n) => *n,
            Target::Dynamic(f) => f(world),
        }
    }
}

pub struct SpawnSpec {
    pub name: String,
    pub factory: fn(&mut World) -> Option<Entity>,
    pub target: Target,
    pub chance: f64,
}

pub struct Spawner {
    pub specs: Vec<SpawnSpec>,
}

impl Spawner {
    pub fn new() -> Self {
        Spawner { specs: Vec::new() }
    }

    pub fn register(
        &mut self,
        name: &str,
        factory: fn(&mut World) -> Option<Entity>,
        target: Target,
        chance: f64,
    ) {
        self.specs.push(SpawnSpec {
            name: name.to_string(),
            factory,
            target,
            chance,
        });
    }

    pub fn tick(&self, world: &mut World, dt: f64) {
        for spec in &self.specs {
            let target = spec.target.resolve(world);
            let count = world
                .entities
                .iter()
                .filter(|e| e.name.as_deref() == Some(spec.name.as_str()))
                .count() as i32;
            if count < target && world.rng.gen::<f64>() < spec.chance * dt {
                if let Some(e) = (spec.factory)(world) {
                    world.add(e);
                }
            }
        }
    }
}

impl Default for Spawner {
    fn default() -> Self {
        Spawner::new()
    }
}

pub fn step(world: &mut World, spawner: &Spawner, dt: f64) {
    if let Some(env) = world.env.as_mut() {
        env.update(dt); // avance horloge+météo avant que les spawners lisent l'état
    }
    world.advance(dt);
    spawner.tick(world, dt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Entity, World};
    use crate::environment::Environment;

    fn make_world() -> World {
        World::new(20, 10)
    }

    fn bug(_w: &mut World) -> Option<Entity> {
        Some(Entity::new(vec!["x".into()]).pos(1.0, 1.0).with_name("bug"))
    }

    fn count(w: &World, name: &str) -> usize {
        w.entities.iter().filter(|e| e.name.as_deref() == Some(name)).count()
    }

    #[test]
    fn spawner_fills_to_target() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Fixed(2), 1.0);
        for _ in 0..5 {
            sp.tick(&mut w, 1.0);
        }
        assert_eq!(count(&w, "bug"), 2);
    }

    #[test]
    fn spawner_respects_chance_zero() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Fixed(3), 0.0);
        sp.tick(&mut w, 1.0);
        assert_eq!(count(&w, "bug"), 0);
    }

    #[test]
    fn step_advances_then_spawns() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Fixed(1), 1.0);
        w.add(Entity::new(vec!["x".into()]).pos(1.0, 1.0).vel(1.0, 0.0).with_name("mover"));
        step(&mut w, &sp, 1.0);
        let mover = w.entities.iter().find(|e| e.name.as_deref() == Some("mover")).unwrap();
        assert_eq!(mover.x, 2.0);
        assert_eq!(count(&w, "bug"), 1);
    }

    #[test]
    fn callable_target_resolves_through_tick() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Dynamic(|_w| 2), 1.0);
        for _ in 0..5 {
            sp.tick(&mut w, 1.0);
        }
        assert_eq!(count(&w, "bug"), 2);
    }

    #[test]
    fn callable_target_zero_blocks_spawn() {
        let mut w = make_world();
        let mut sp = Spawner::new();
        sp.register("bug", bug, Target::Dynamic(|_w| 0), 1.0);
        for _ in 0..5 {
            sp.tick(&mut w, 1.0);
        }
        assert_eq!(count(&w, "bug"), 0);
    }

    // Prouve que step ticke l'env AVANT que le spawner observe l'état :
    // la factory ne spawne que si la phase a déjà avancé (> 0).
    fn phase_gated(w: &mut World) -> Option<Entity> {
        if w.env.as_ref().unwrap().phase() > 0.0 {
            Some(Entity::new(vec!["x".into()]).with_name("bug"))
        } else {
            None
        }
    }

    #[test]
    fn step_ticks_env_before_spawn() {
        let mut w = make_world();
        w.env = Some(Environment::seeded(1000.0, 0));
        let mut sp = Spawner::new();
        sp.register("bug", phase_gated, Target::Fixed(1), 1.0);
        step(&mut w, &sp, 1.0);
        assert_eq!(count(&w, "bug"), 1);
    }
}
