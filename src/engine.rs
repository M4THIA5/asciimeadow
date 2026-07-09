//! Moteur pur (sans crossterm) : couleurs, entités, buffer, compositor.

use rand::rngs::StdRng;
use rand::SeedableRng;

/// Palette logique. Le mapping vers crossterm vit dans la coquille (main.rs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Green,
    Brown,
    Yellow,
    Red,
    Cyan,
    Blue,
    Magenta,
    Black,
}

pub const COLOR_NAMES: [Color; 9] = [
    Color::White,
    Color::Green,
    Color::Brown,
    Color::Yellow,
    Color::Red,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
    Color::Black,
];

/// Caractère de masque -> couleur (style asciiquarium). `None` => couleur de base.
pub fn mask_color(c: char) -> Option<Color> {
    match c {
        'w' => Some(Color::White),
        'g' => Some(Color::Green),
        'n' => Some(Color::Brown),
        'y' => Some(Color::Yellow),
        'r' => Some(Color::Red),
        'c' => Some(Color::Cyan),
        'b' => Some(Color::Blue),
        'm' => Some(Color::Magenta),
        'k' => Some(Color::Black),
        _ => None,
    }
}

// Profondeur : plus grand = plus loin (dessiné en premier).
pub const DEPTH_SUN: i32 = 90;
pub const DEPTH_CLOUD: i32 = 80;
pub const DEPTH_SKY_CREATURE: i32 = 70;
pub const DEPTH_TREE: i32 = 60;
pub const DEPTH_TREE_CREATURE: i32 = 50;
pub const DEPTH_GRASS: i32 = 45; // herbe dense : derrière les animaux du sol
pub const DEPTH_GROUND_ANIMAL: i32 = 40;
pub const DEPTH_FOREGROUND: i32 = 30;

fn flip_char(c: char) -> char {
    match c {
        '<' => '>',
        '>' => '<',
        '[' => ']',
        ']' => '[',
        '(' => ')',
        ')' => '(',
        '{' => '}',
        '}' => '{',
        '/' => '\\',
        '\\' => '/',
        other => other,
    }
}

/// Retourne un sprite horizontalement : inverse chaque ligne et permute les
/// glyphes directionnels. Sert aux entités entrant par la droite.
pub fn flip_horizontal(frame: &str) -> String {
    frame
        .split('\n')
        .map(|line| line.chars().rev().map(flip_char).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Vue immuable de l'environnement, passée par valeur aux comportements chaque
/// frame (évite d'emprunter `world.env` en mutant une entité).
#[derive(Clone, Copy)]
pub struct EnvSnapshot {
    pub is_day: bool,
    pub is_night: bool,
    pub raining: bool,
    pub windy: bool,
    pub storming: bool,
    pub wind_dx: f64,
}

impl EnvSnapshot {
    /// Défaut hors env : plein jour, temps clair.
    pub fn none() -> Self {
        EnvSnapshot {
            is_day: true,
            is_night: false,
            raining: false,
            windy: false,
            storming: false,
            wind_dx: 0.0,
        }
    }
}

/// Comportement d'entité (mouvement / durée de vie / cull). L'état mutable vit
/// dans la variante ; le chaînage = l'ordre du `Vec<Behavior>` de l'entité.
pub enum Behavior {
    Fall { gravity: f64, ground_y: f64, first: bool },
    Hop { ground_y: f64, amplitude: f64, period: f64, t: f64 },
    Orbit { cx: f64, cy: f64, radius: f64, ang_speed: f64, a: f64 },
    Zigzag { top: f64, bottom: f64, vy: f64 },
    Lifespan { seconds: f64, t: f64 },
    /// `day=true` : vit le jour, meurt la nuit. `day=false` : l'inverse.
    EnvCull { day: bool },
}

impl Behavior {
    fn apply(&mut self, e: &mut Entity, dt: f64, env: EnvSnapshot) {
        match self {
            Behavior::Fall { gravity, ground_y, first } => {
                e.dy += *gravity * dt;
                if *first {
                    e.y += 0.5 * *gravity * dt * dt;
                    *first = false;
                }
                let h = e.height() as f64;
                if e.y + h >= *ground_y {
                    e.y = *ground_y - h;
                    e.alive = false;
                }
            }
            Behavior::Hop { ground_y, amplitude, period, t } => {
                *t += dt;
                let phase = (*t / *period) * std::f64::consts::PI;
                e.y = *ground_y - e.height() as f64 - phase.sin().abs() * *amplitude;
            }
            Behavior::Orbit { cx, cy, radius, ang_speed, a } => {
                *a += *ang_speed * dt;
                e.x = *cx + *radius * a.cos();
                e.y = *cy + (*radius / 2.0) * a.sin();
            }
            Behavior::Zigzag { top, bottom, vy } => {
                if e.dy == 0.0 {
                    e.dy = *vy;
                }
                if e.y <= *top && e.dy < 0.0 {
                    e.dy = e.dy.abs();
                } else if e.y >= *bottom && e.dy > 0.0 {
                    e.dy = -e.dy.abs();
                }
            }
            Behavior::Lifespan { seconds, t } => {
                *t += dt;
                if *t >= *seconds {
                    e.alive = false;
                }
            }
            Behavior::EnvCull { day } => {
                let keep = if *day { env.is_day } else { env.is_night };
                if !keep {
                    e.alive = false;
                }
            }
        }
    }
}

/// Callback exécuté quand une entité est supprimée (`on_death`).
pub type DeathFn = Box<dyn FnMut(&mut Entity, &mut World)>;

/// Entité : frames multi-lignes + position/vitesse + comportements + drapeaux.
pub struct Entity {
    pub frames: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub depth: i32,
    pub frame_rate: f64,
    pub color: Color,
    pub color_mask: Option<Vec<String>>,
    pub name: Option<String>,
    pub opaque: bool,
    pub alive: bool,
    pub behaviors: Vec<Behavior>,
    pub on_death: Option<DeathFn>,
    frame_idx: usize,
    anim_accum: f64,
}

impl Entity {
    pub fn new(frames: Vec<String>) -> Self {
        Entity {
            frames,
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
            depth: 0,
            frame_rate: 0.0,
            color: Color::White,
            color_mask: None,
            name: None,
            opaque: false,
            alive: true,
            behaviors: Vec::new(),
            on_death: None,
            frame_idx: 0,
            anim_accum: 0.0,
        }
    }

    pub fn pos(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn vel(mut self, dx: f64, dy: f64) -> Self {
        self.dx = dx;
        self.dy = dy;
        self
    }
    pub fn with_depth(mut self, d: i32) -> Self {
        self.depth = d;
        self
    }
    pub fn with_color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn with_frame_rate(mut self, r: f64) -> Self {
        self.frame_rate = r;
        self
    }
    pub fn with_mask(mut self, m: Vec<String>) -> Self {
        self.color_mask = Some(m);
        self
    }
    pub fn with_name(mut self, n: &str) -> Self {
        self.name = Some(n.to_string());
        self
    }
    pub fn opaque(mut self, v: bool) -> Self {
        self.opaque = v;
        self
    }
    pub fn with_behavior(mut self, b: Behavior) -> Self {
        self.behaviors.push(b);
        self
    }
    pub fn on_death(mut self, f: DeathFn) -> Self {
        self.on_death = Some(f);
        self
    }

    pub fn current_frame(&self) -> &str {
        self.frames[self.frame_idx].as_str()
    }
    pub fn current_mask(&self) -> Option<&str> {
        self.color_mask.as_ref().map(|m| m[self.frame_idx].as_str())
    }
    pub fn height(&self) -> usize {
        self.current_frame().split('\n').count()
    }
    pub fn width(&self) -> usize {
        self.current_frame()
            .split('\n')
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
    }

    pub fn advance(&mut self, dt: f64, env: EnvSnapshot) {
        self.x += self.dx * dt;
        self.y += self.dy * dt;
        if self.frame_rate > 0.0 && self.frames.len() > 1 {
            self.anim_accum += dt;
            let step = 1.0 / self.frame_rate;
            while self.anim_accum >= step {
                self.anim_accum -= step;
                self.frame_idx = (self.frame_idx + 1) % self.frames.len();
            }
        }
        // Comportements chaînés : on sort le Vec pour lever l'alias self/behaviors.
        let mut behaviors = std::mem::take(&mut self.behaviors);
        for b in &mut behaviors {
            b.apply(self, dt, env);
        }
        self.behaviors = behaviors;
    }
}

/// Grilles parallèles de caractères et couleurs. Espace = transparent au draw.
#[derive(Clone)]
pub struct Buffer {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Vec<char>>,
    pub colors: Vec<Vec<Color>>,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        Buffer {
            width,
            height,
            chars: vec![vec![' '; width]; height],
            colors: vec![vec![Color::White; width]; height],
        }
    }

    pub fn draw_entity(&mut self, e: &Entity) {
        let frame = e.current_frame();
        let lines: Vec<&str> = frame.split('\n').collect();
        let mlines: Option<Vec<&str>> = e.current_mask().map(|m| m.split('\n').collect());
        let ox = e.x as i32;
        let oy = e.y as i32;
        for (r, line) in lines.iter().enumerate() {
            let y = oy + r as i32;
            if y < 0 || y >= self.height as i32 {
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            // Sprite opaque : les trous entre le 1er et le dernier glyphe recouvrent
            // le décor avec du vide au lieu de le laisser transparaître.
            let (mut lo, mut hi): (i32, i32) = (-1, -1);
            if e.opaque {
                let first = chars.iter().position(|&c| c != ' ');
                let last = chars.iter().rposition(|&c| c != ' ');
                if let (Some(f), Some(l)) = (first, last) {
                    lo = f as i32;
                    hi = l as i32;
                }
            }
            for (c, &ch) in chars.iter().enumerate() {
                let ci = c as i32;
                if ch == ' ' && !(lo <= ci && ci <= hi) {
                    continue;
                }
                let x = ox + ci;
                if x < 0 || x >= self.width as i32 {
                    continue;
                }
                let (xu, yu) = (x as usize, y as usize);
                self.chars[yu][xu] = ch; // ch == ' ' => recouvre (opaque)
                let mut color = e.color;
                if ch != ' ' {
                    if let Some(mls) = &mlines {
                        if r < mls.len() {
                            if let Some(mc) = mls[r].chars().nth(c) {
                                if mc != ' ' {
                                    color = mask_color(mc).unwrap_or(e.color);
                                }
                            }
                        }
                    }
                }
                self.colors[yu][xu] = color;
            }
        }
    }
}

/// Dessine les entités triées par profondeur décroissante (algo du peintre).
/// Tri stable : à profondeur égale, l'ordre d'insertion est préservé.
pub fn composite(buf: &mut Buffer, entities: &[&Entity]) {
    let mut order: Vec<&Entity> = entities.to_vec();
    order.sort_by_key(|e| std::cmp::Reverse(e.depth));
    for e in &order {
        buf.draw_entity(e);
    }
}

/// Monde : dimensions, entités, environnement optionnel, RNG maître.
pub struct World {
    pub width: usize,
    pub height: usize,
    pub entities: Vec<Entity>,
    pub env: Option<crate::environment::Environment>,
    pub rng: StdRng,
}

impl World {
    pub fn with_rng(width: usize, height: usize, rng: StdRng) -> Self {
        World {
            width,
            height,
            entities: Vec::new(),
            env: None,
            rng,
        }
    }
    pub fn new(width: usize, height: usize) -> Self {
        World::with_rng(width, height, StdRng::seed_from_u64(0))
    }
    pub fn seeded(width: usize, height: usize, seed: u64) -> Self {
        World::with_rng(width, height, StdRng::seed_from_u64(seed))
    }

    pub fn add(&mut self, e: Entity) -> usize {
        self.entities.push(e);
        self.entities.len() - 1
    }

    pub fn env_snapshot(&self) -> EnvSnapshot {
        match &self.env {
            Some(env) => env.snapshot(),
            None => EnvSnapshot::none(),
        }
    }

    fn offscreen(&self, e: &Entity) -> bool {
        if e.dx > 0.0 && e.x >= self.width as f64 {
            return true;
        }
        if e.dx < 0.0 && e.x + e.width() as f64 <= 0.0 {
            return true;
        }
        if e.dy > 0.0 && e.y >= self.height as f64 {
            return true;
        }
        if e.dy < 0.0 && e.y + e.height() as f64 <= 0.0 {
            return true;
        }
        false
    }

    pub fn advance(&mut self, dt: f64) {
        let snap = self.env_snapshot();
        for e in &mut self.entities {
            e.advance(dt, snap);
        }
        let taken = std::mem::take(&mut self.entities);
        let mut kept: Vec<Entity> = Vec::new();
        let mut dead: Vec<Entity> = Vec::new();
        for e in taken {
            if e.alive && !self.offscreen(&e) {
                kept.push(e);
            } else {
                dead.push(e);
            }
        }
        self.entities = kept;
        for mut e in dead {
            if let Some(mut f) = e.on_death.take() {
                f(&mut e, self);
            }
        }
    }

    pub fn render(&self) -> Buffer {
        let mut buf = Buffer::new(self.width, self.height);
        let refs: Vec<&Entity> = self.entities.iter().collect();
        composite(&mut buf, &refs);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_reverses_and_swaps_chars() {
        assert_eq!(flip_horizontal("<o--"), "--o>");
        assert_eq!(flip_horizontal("(_)"), "(_)");
    }

    #[test]
    fn flip_multiline_each_line() {
        assert_eq!(flip_horizontal("ab\ncd"), "ba\ndc");
    }

    #[test]
    fn flip_swaps_braces_and_brackets() {
        assert_eq!(flip_horizontal("{a"), "a}");
        assert_eq!(flip_horizontal("[x"), "x]");
        assert_eq!(flip_horizontal("(y"), "y)");
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn depth_grass_between_ground_animals_and_tree_creatures() {
        assert!(DEPTH_GROUND_ANIMAL < DEPTH_GRASS && DEPTH_GRASS < DEPTH_TREE_CREATURE);
    }

    #[test]
    fn mask_color_known_and_unknown() {
        assert_eq!(mask_color('g'), Some(Color::Green));
        assert_eq!(mask_color('n'), Some(Color::Brown));
        assert_eq!(mask_color(' '), None);
        assert_eq!(mask_color('?'), None);
    }

    // --- Task 3 : EnvSnapshot, Behavior, Entity ---

    #[test]
    fn entity_moves_by_velocity() {
        let mut e = Entity::new(vec!["x".into()]).pos(0.0, 0.0).vel(2.0, -1.0);
        e.advance(0.5, EnvSnapshot::none());
        assert_eq!(e.x, 1.0);
        assert_eq!(e.y, -0.5);
    }

    #[test]
    fn entity_animation_advances_frames() {
        let mut e = Entity::new(vec!["A".into(), "B".into()]).with_frame_rate(2.0);
        assert_eq!(e.current_frame(), "A");
        e.advance(0.5, EnvSnapshot::none());
        assert_eq!(e.current_frame(), "B");
        e.advance(0.5, EnvSnapshot::none());
        assert_eq!(e.current_frame(), "A");
    }

    #[test]
    fn entity_single_frame_does_not_animate() {
        let mut e = Entity::new(vec!["A".into()]).with_frame_rate(5.0);
        e.advance(10.0, EnvSnapshot::none());
        assert_eq!(e.current_frame(), "A");
    }

    #[test]
    fn entity_dimensions() {
        let e = Entity::new(vec!["abc\nde".into()]);
        assert_eq!(e.width(), 3);
        assert_eq!(e.height(), 2);
    }

    #[test]
    fn entity_current_mask_none_vs_present() {
        let e = Entity::new(vec!["ab".into()]).with_mask(vec!["rg".into()]);
        assert_eq!(e.current_mask(), Some("rg"));
        let e2 = Entity::new(vec!["ab".into()]);
        assert_eq!(e2.current_mask(), None);
    }

    #[test]
    fn behavior_fall_accelerates_and_dies_at_ground() {
        let mut e = Entity::new(vec!["@".into()]).pos(0.0, 0.0)
            .with_behavior(Behavior::Fall { gravity: 20.0, ground_y: 5.0, first: true });
        let y0 = e.y;
        e.advance(0.1, EnvSnapshot::none());
        assert!(e.y > y0);
        for _ in 0..100 { e.advance(0.1, EnvSnapshot::none()); }
        assert!(!e.alive);
        assert!(e.y + e.height() as f64 <= 5.0 + 1e-6);
    }

    #[test]
    fn behavior_hop_keeps_feet_near_ground() {
        let mut e = Entity::new(vec!["R".into()]).vel(1.0, 0.0)
            .with_behavior(Behavior::Hop { ground_y: 10.0, amplitude: 3.0, period: 0.5, t: 0.0 });
        for _ in 0..50 {
            e.advance(0.05, EnvSnapshot::none());
            let bottom = e.y + e.height() as f64;
            assert!(bottom <= 10.0 + 1e-6);
            assert!(e.y >= 10.0 - 3.0 - e.height() as f64 - 1e-6);
        }
    }

    #[test]
    fn behavior_orbit_stays_within_radius() {
        let mut e = Entity::new(vec!["b".into()])
            .with_behavior(Behavior::Orbit { cx: 20.0, cy: 10.0, radius: 4.0, ang_speed: 2.0, a: 0.0 });
        for _ in 0..60 {
            e.advance(0.05, EnvSnapshot::none());
            let dist = ((e.x - 20.0).powi(2) + ((e.y - 10.0) * 2.0).powi(2)).sqrt();
            assert!(dist <= 4.0 + 1e-6);
        }
    }

    #[test]
    fn behavior_zigzag_inverts_at_bounds() {
        let mut e = Entity::new(vec!["x".into()]).pos(0.0, 5.0).vel(2.0, 0.0)
            .with_behavior(Behavior::Zigzag { top: 2.0, bottom: 8.0, vy: 10.0 });
        e.advance(0.01, EnvSnapshot::none());
        assert!(e.dy > 0.0);
        let mut saw_up = false;
        for _ in 0..120 {
            e.advance(0.05, EnvSnapshot::none());
            if e.dy < 0.0 { saw_up = true; }
        }
        assert!(saw_up);
    }

    #[test]
    fn behavior_lifespan_kills_after_delay() {
        let mut e = Entity::new(vec!["x".into()])
            .with_behavior(Behavior::Lifespan { seconds: 0.3, t: 0.0 });
        e.advance(0.2, EnvSnapshot::none());
        assert!(e.alive);
        e.advance(0.2, EnvSnapshot::none());
        assert!(!e.alive);
    }

    #[test]
    fn behavior_env_cull_day_dies_at_night() {
        let day = EnvSnapshot { is_day: true, is_night: false, raining: false, windy: false, storming: false, wind_dx: 0.0 };
        let night = EnvSnapshot { is_day: false, is_night: true, raining: false, windy: false, storming: false, wind_dx: 0.0 };
        let mut e = Entity::new(vec!["s".into()]).with_behavior(Behavior::EnvCull { day: true });
        e.advance(0.1, day);
        assert!(e.alive);
        e.advance(0.1, night);
        assert!(!e.alive);
    }

    // --- Task 4 : Buffer, draw_entity, composite ---

    #[test]
    fn buffer_init_blank() {
        let b = Buffer::new(3, 2);
        assert_eq!(b.chars, vec![vec![' '; 3]; 2]);
    }

    #[test]
    fn draw_entity_places_chars_and_skips_spaces() {
        let mut b = Buffer::new(5, 2);
        let e = Entity::new(vec!["a b".into()]).pos(1.0, 0.0).with_color(Color::Green);
        b.draw_entity(&e);
        assert_eq!(b.chars[0][1], 'a');
        assert_eq!(b.chars[0][2], ' '); // espace transparent
        assert_eq!(b.chars[0][3], 'b');
        assert_eq!(b.colors[0][1], Color::Green);
    }

    #[test]
    fn draw_entity_clips_at_edges() {
        let mut b = Buffer::new(3, 1);
        let e = Entity::new(vec!["xyz".into()]).pos(2.0, 0.0);
        b.draw_entity(&e);
        assert_eq!(b.chars[0][2], 'x');
    }

    #[test]
    fn draw_entity_color_mask_overrides() {
        let mut b = Buffer::new(3, 1);
        let e = Entity::new(vec!["ab".into()]).pos(0.0, 0.0).with_color(Color::White)
            .with_mask(vec!["r ".into()]);
        b.draw_entity(&e);
        assert_eq!(b.colors[0][0], Color::Red);
        assert_eq!(b.colors[0][1], Color::White);
    }

    #[test]
    fn composite_nearer_entity_wins() {
        let mut b = Buffer::new(1, 1);
        let far = Entity::new(vec!["F".into()]).with_depth(80);
        let near = Entity::new(vec!["N".into()]).with_depth(30);
        composite(&mut b, &[&near, &far]);
        assert_eq!(b.chars[0][0], 'N');
    }

    #[test]
    fn opaque_animal_hides_grass_in_its_silhouette() {
        let mut buf = Buffer::new(20, 4);
        let grass = Entity::new(vec!["v.,'vv,w.'v,.v'wWvWv".into()]).pos(0.0, 1.0)
            .with_depth(DEPTH_GRASS).with_name("grass");
        let mut animal = Entity::new(vec!["( )".into()]).pos(5.0, 1.0)
            .with_depth(DEPTH_GROUND_ANIMAL).opaque(true);
        composite(&mut buf, &[&grass, &animal]);
        assert_eq!(buf.chars[1][6], ' '); // trou intérieur vidé
        let mut buf2 = Buffer::new(20, 4);
        animal.opaque = false;
        composite(&mut buf2, &[&grass, &animal]);
        assert_ne!(buf2.chars[1][6], ' '); // sans opaque, l'herbe transparaît
    }

    // --- Task 6 : World ---

    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn world_has_generic_env_slot_defaulting_none() {
        let w = World::new(10, 5);
        assert!(w.env.is_none());
    }

    #[test]
    fn world_culls_entity_exiting_its_direction() {
        let mut w = World::new(10, 5);
        w.add(Entity::new(vec!["x".into()]).pos(9.0, 0.0).vel(1.0, 0.0));
        w.advance(2.0);
        assert!(w.entities.is_empty());
    }

    #[test]
    fn world_allows_entry_from_offscreen() {
        let mut w = World::new(10, 5);
        w.add(Entity::new(vec!["xxx".into()]).pos(-3.0, 0.0).vel(1.0, 0.0));
        w.advance(1.0);
        assert_eq!(w.entities.len(), 1);
    }

    #[test]
    fn world_keeps_static_entity() {
        let mut w = World::new(10, 5);
        w.add(Entity::new(vec!["x".into()]).pos(0.0, 0.0));
        w.advance(100.0);
        assert_eq!(w.entities.len(), 1);
    }

    #[test]
    fn world_on_death_called_when_culled() {
        let mut w = World::new(5, 5);
        let hit = Rc::new(Cell::new(false));
        let hit2 = hit.clone();
        let e = Entity::new(vec!["x".into()]).pos(4.0, 0.0).vel(1.0, 0.0)
            .on_death(Box::new(move |_e, _w| hit2.set(true)));
        w.add(e);
        w.advance(5.0);
        assert!(hit.get());
    }

    #[test]
    fn world_removes_dead_flag_entities() {
        let mut w = World::new(5, 5);
        let idx = w.add(Entity::new(vec!["x".into()]).pos(1.0, 1.0));
        w.entities[idx].alive = false;
        w.advance(0.1);
        assert!(w.entities.is_empty());
    }

    #[test]
    fn world_render_composites() {
        let mut w = World::new(2, 1);
        w.add(Entity::new(vec!["a".into()]).pos(0.0, 0.0).with_depth(10));
        let buf = w.render();
        assert_eq!(buf.chars[0][0], 'a');
    }
}
