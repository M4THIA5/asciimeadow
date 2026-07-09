//! Scène : construction du pré, factories comportement, factories créatures, spawners.

use crate::art;
use crate::engine::{self, Behavior, Color, Entity, World};
use crate::environment::Environment;
use crate::spawn::{Spawner, Target};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

/// Nombre de rangs de la bande terrestre : ~¼ de l'écran, min 6, plafonné à la
/// moitié de la hauteur (protège les petits terminaux, laisse la place au ciel).
pub fn ground_rows(world: &World) -> i32 {
    let h = world.height as i32;
    std::cmp::min(std::cmp::max(6, h / 4), std::cmp::max(1, h / 2))
}

/// Largeur max (en glyphes) parmi les lignes d'une frame.
fn max_line_width(frame: &str) -> i32 {
    frame.split('\n').map(|l| l.chars().count()).max().unwrap_or(0) as i32
}

pub fn ground_top(world: &World) -> i32 {
    world.height as i32 - ground_rows(world)
}

/// Variante d'arbre selon la taille du monde (le resize resélectionne).
pub fn tree_art(world: &World) -> (&'static str, &'static str) {
    if world.height >= 28 && world.width >= 40 {
        (art::TREE_LARGE, art::TREE_LARGE_MASK)
    } else {
        (art::TREE_SMALL, art::TREE_SMALL_MASK)
    }
}

pub fn tree_origin(world: &World) -> (i32, i32) {
    let (frame, _) = tree_art(world);
    let tree_w = max_line_width(frame);
    let tree_h = frame.split('\n').count() as i32;
    let ox = world.width as i32 / 2 - tree_w / 2;
    let oy = ground_top(world) - tree_h;
    (ox, oy)
}

/// Variante de nuage selon la taille du monde (grand nuage si large).
pub fn cloud_art(world: &World) -> (&'static str, &'static str) {
    if world.height >= 24 && world.width >= 60 {
        (art::CLOUD_LARGE, art::CLOUD_LARGE_MASK)
    } else {
        (art::CLOUD_SMALL, art::CLOUD_SMALL_MASK)
    }
}

pub fn ground_frames(width: usize, rows: usize) -> Vec<String> {
    // Chaque rang cycle un motif de GRASS_ROWS, décalé horizontalement de `i`
    // glyphes pour éviter une répétition verticale visible quand la bande dépasse
    // 4 lignes.
    let base: Vec<String> = (0..rows)
        .map(|i| {
            let p = art::GRASS_ROWS[i % art::GRASS_ROWS.len()];
            let plen = p.chars().count();
            let reps = width / plen + 2;
            let repeated: Vec<char> = p.repeat(reps).chars().collect();
            let start = i % plen; // décalage horizontal par rang
            repeated[start..start + width].iter().collect()
        })
        .collect();
    let shifted: Vec<String> = base
        .iter()
        .map(|r| {
            let chars: Vec<char> = r.chars().collect();
            let mut s: String = chars[1..].iter().collect();
            s.push(chars[0]);
            s
        })
        .collect();
    vec![base.join("\n"), shifted.join("\n")]
}

/// Pas d'env => on considère qu'il fait jour (compat : prairie diurne).
pub fn is_day(world: &World) -> bool {
    match &world.env {
        None => true,
        Some(env) => !env.is_night(),
    }
}
pub fn is_night(world: &World) -> bool {
    match &world.env {
        None => false,
        Some(env) => env.is_night(),
    }
}

pub fn build_meadow(world: &mut World, day_length: f64) {
    // Environnement global (horloge jour/nuit + météo), rng dérivé du rng maître.
    let env_seed = world.rng.next_u64();
    world.env = Some(Environment::new(day_length, StdRng::seed_from_u64(env_seed)));

    // Arbre — centré, base au sol, variante selon la taille du terminal.
    let (tf, tm) = tree_art(world);
    let (tox, toy) = tree_origin(world);
    world.add(
        Entity::new(vec![tf.to_string()])
            .with_mask(vec![tm.to_string()])
            .pos(tox as f64, toy as f64)
            .with_depth(engine::DEPTH_TREE)
            .with_color(Color::Green)
            .with_name("tree"),
    );

    // Sol — pleine largeur, herbe dense ondulante, derrière les animaux du sol.
    let gframes = ground_frames(world.width, ground_rows(world) as usize);
    world.add(
        Entity::new(gframes)
            .pos(0.0, ground_top(world) as f64)
            .with_depth(engine::DEPTH_GRASS)
            .with_frame_rate(2.0)
            .with_color(Color::Green)
            .with_name("ground"),
    );

    // Fleurs — dispersées sur toute la bande d'herbe.
    let gy = ground_top(world);
    let count = std::cmp::max(3, world.width / 12);
    for _ in 0..count {
        let fx = world.rng.gen_range(0..=world.width as i32 - 1) as f64;
        let fy = world.rng.gen_range(gy..=world.height as i32 - 1) as f64;
        let flower = art::FLOWERS[world.rng.gen_range(0..art::FLOWERS.len())];
        let palette = [Color::Red, Color::Yellow, Color::Magenta, Color::White];
        let color = palette[world.rng.gen_range(0..palette.len())];
        world.add(
            Entity::new(vec![flower.to_string()])
                .pos(fx, fy)
                .with_depth(engine::DEPTH_FOREGROUND)
                .with_color(color)
                .with_name("flower"),
        );
    }
}

/// Crée une entité qui traverse l'écran depuis un côté aléatoire.
#[allow(clippy::too_many_arguments)]
fn cross_factory(
    world: &mut World,
    frames: &[String],
    depth: i32,
    speed: f64,
    y: f64,
    color: Color,
    name: &str,
    mask: Option<&[String]>,
    frame_rate: f64,
) -> Entity {
    let from_left = world.rng.gen::<f64>() < 0.5;
    let (x, dx, used): (f64, f64, Vec<String>) = if from_left {
        let w0 = max_line_width(&frames[0]) as f64;
        (-w0, speed, frames.to_vec())
    } else {
        (
            world.width as f64,
            -speed,
            frames.iter().map(|f| engine::flip_horizontal(f)).collect(),
        )
    };
    let used_mask: Option<Vec<String>> = mask.map(|m| {
        if from_left {
            m.to_vec()
        } else {
            m.iter().map(|s| engine::flip_horizontal(s)).collect()
        }
    });
    let mut e = Entity::new(used)
        .pos(x, y)
        .vel(dx, 0.0)
        .with_depth(depth)
        .with_frame_rate(frame_rate)
        .with_color(color)
        .with_name(name);
    if let Some(m) = used_mask {
        e = e.with_mask(m);
    }
    e
}

fn celestial_x(world: &World, frame: &str) -> f64 {
    (world.width as i32 - max_line_width(frame) - 1) as f64
}

pub fn spawn_bird(world: &mut World) -> Option<Entity> {
    let y = world.rng.gen_range(1..=std::cmp::max(2, world.height as i32 / 3)) as f64;
    let speed = world.rng.gen_range(8.0..14.0);
    let frames: Vec<String> = art::BIRD.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::BIRD_MASK.iter().map(|s| s.to_string()).collect();
    Some(cross_factory(
        world,
        &frames,
        engine::DEPTH_SKY_CREATURE,
        speed,
        y,
        Color::White,
        "bird",
        Some(&masks),
        4.0,
    ))
}

pub fn spawn_cloud(world: &mut World) -> Option<Entity> {
    let (frame, mask) = cloud_art(world);
    let y = world.rng.gen_range(0..=std::cmp::max(1, world.height as i32 / 4)) as f64;
    let speed = world.rng.gen_range(1.5..3.0);
    let frames = vec![frame.to_string()];
    let masks = vec![mask.to_string()];
    Some(cross_factory(
        world,
        &frames,
        engine::DEPTH_CLOUD,
        speed,
        y,
        Color::White,
        "cloud",
        Some(&masks),
        0.0,
    ))
}

pub fn spawn_butterfly(world: &mut World) -> Option<Entity> {
    let y = ground_top(world) as f64 - world.rng.gen_range(2..=6) as f64;
    let speed = world.rng.gen_range(3.0..6.0);
    let frames: Vec<String> = art::BUTTERFLY.iter().map(|s| s.to_string()).collect();
    let bottom = ground_top(world) as f64 - 1.0;
    let vy = world.rng.gen_range(3.0..6.0);
    let e = cross_factory(
        world,
        &frames,
        engine::DEPTH_SKY_CREATURE,
        speed,
        y,
        Color::Magenta,
        "butterfly",
        None,
        6.0,
    )
    .with_behavior(Behavior::Zigzag { top: 1.0, bottom, vy })
    .with_behavior(Behavior::EnvCull { day: true });
    Some(e)
}

pub fn spawn_owl(world: &mut World) -> Option<Entity> {
    let (tox, toy) = tree_origin(world);
    let (tf, _) = tree_art(world);
    let tree_w = max_line_width(tf);
    let tree_h = tf.split('\n').count() as i32;
    let owl_w = max_line_width(art::OWL[0]);
    let x = tox + tree_w / 2 - owl_w / 2;
    let y = toy + std::cmp::max(1, tree_h / 2 - 3); // centré dans la canopée
    let frames: Vec<String> = art::OWL.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::OWL_MASK.iter().map(|s| s.to_string()).collect();
    Some(
        Entity::new(frames)
            .with_mask(masks)
            .pos(x as f64, y as f64)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_frame_rate(0.4)
            .with_color(Color::Brown)
            .with_name("owl")
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_bee(world: &mut World) -> Option<Entity> {
    let cx = (world.width / 2) as f64;
    let (_, toy) = tree_origin(world);
    let cy = toy as f64 + 1.0;
    let radius = world.rng.gen_range(3.0..6.0);
    let ang_speed = world.rng.gen_range(2.0..4.0);
    let frames: Vec<String> = art::BEE.iter().map(|s| s.to_string()).collect();
    Some(
        Entity::new(frames)
            .pos(cx, cy)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_frame_rate(10.0)
            .with_color(Color::Yellow)
            .with_name("bee")
            .with_behavior(Behavior::Orbit { cx, cy, radius, ang_speed, a: 0.0 })
            .with_behavior(Behavior::EnvCull { day: true }),
    )
}

pub fn spawn_sun(world: &mut World) -> Option<Entity> {
    Some(
        Entity::new(vec![art::SUN.to_string()])
            .with_mask(vec![art::SUN_MASK.to_string()])
            .pos(celestial_x(world, art::SUN), 0.0)
            .with_depth(engine::DEPTH_SUN)
            .with_color(Color::Yellow)
            .with_name("sun")
            .with_behavior(Behavior::EnvCull { day: true }),
    )
}

pub fn spawn_moon(world: &mut World) -> Option<Entity> {
    Some(
        Entity::new(vec![art::MOON.to_string()])
            .with_mask(vec![art::MOON_MASK.to_string()])
            .pos(celestial_x(world, art::MOON), 0.0)
            .with_depth(engine::DEPTH_SUN)
            .with_color(Color::White)
            .with_name("moon")
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_star(world: &mut World) -> Option<Entity> {
    let x = world.rng.gen_range(0..=world.width as i32 - 1) as f64;
    let y = world.rng.gen_range(0..=world.height as i32 / 3) as f64;
    let ch = art::STAR_CHARS[world.rng.gen_range(0..art::STAR_CHARS.len())];
    let color = if world.rng.gen::<bool>() { Color::White } else { Color::Yellow };
    Some(
        Entity::new(vec![ch.to_string()])
            .pos(x, y)
            .with_depth(engine::DEPTH_SUN)
            .with_color(color)
            .with_name("star")
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_firefly(world: &mut World) -> Option<Entity> {
    let (tox, toy) = tree_origin(world);
    let (tf, _) = tree_art(world);
    let tw = max_line_width(tf);
    let gy = ground_top(world);
    let lo = std::cmp::max(0, tox - 3);
    let hi = std::cmp::min(world.width as i32 - 1, tox + tw + 3);
    let x = world.rng.gen_range(lo..=hi) as f64;
    let y = world.rng.gen_range(toy + 2..=gy) as f64;
    let frame_rate = world.rng.gen_range(2.0..4.0);
    let vy = world.rng.gen_range(1.0..2.5);
    let frames: Vec<String> = art::FIREFLY.iter().map(|s| s.to_string()).collect();
    Some(
        Entity::new(frames)
            .pos(x, y)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_frame_rate(frame_rate)
            .with_color(Color::Yellow)
            .with_name("firefly")
            .with_behavior(Behavior::Zigzag { top: toy as f64, bottom: gy as f64, vy })
            .with_behavior(Behavior::EnvCull { day: false }),
    )
}

pub fn spawn_apple(world: &mut World) -> Option<Entity> {
    let (tox, toy) = tree_origin(world);
    let (tf, _) = tree_art(world);
    let tw = max_line_width(tf);
    let x = world.rng.gen_range(tox + 1..=tox + tw - 2) as f64;
    let is_apple = world.rng.gen::<f64>() < 0.6;
    let (frame, color) = if is_apple {
        (art::APPLE, Color::Red)
    } else {
        (art::LEAF, Color::Green)
    };
    Some(
        Entity::new(vec![frame.to_string()])
            .pos(x, toy as f64 + 1.0)
            .vel(0.0, 0.1)
            .with_depth(engine::DEPTH_TREE_CREATURE)
            .with_color(color)
            .with_name("apple")
            .with_behavior(Behavior::Fall {
                gravity: 12.0,
                ground_y: ground_top(world) as f64,
                first: true,
            }),
    )
}

pub fn spawn_raindrop(world: &mut World) -> Option<Entity> {
    let snap = world.env_snapshot();
    if !snap.raining {
        return None;
    }
    let dx = snap.wind_dx;
    let ch = if dx > 0.0 {
        art::RAIN_CHARS[2] // '\' penché à droite
    } else if dx < 0.0 {
        art::RAIN_CHARS[1] // '/' penché à gauche
    } else {
        art::RAIN_CHARS[0] // '|' vertical
    };
    let x = world.rng.gen_range(0..=world.width as i32 - 1) as f64;
    let dy = world.rng.gen_range(18.0..26.0);
    let color = if world.rng.gen::<bool>() { Color::Cyan } else { Color::Blue };
    Some(
        Entity::new(vec![ch.to_string()])
            .pos(x, 0.0)
            .vel(dx, dy)
            .with_depth(engine::DEPTH_FOREGROUND)
            .with_color(color)
            .with_name("rain"),
    )
}

pub fn spawn_wind_leaf(world: &mut World) -> Option<Entity> {
    let snap = world.env_snapshot();
    if !snap.windy || snap.raining {
        return None;
    }
    let dx = snap.wind_dx * world.rng.gen_range(1.5..2.5);
    let x = if dx >= 0.0 { -1.0 } else { world.width as f64 };
    let y = world.rng.gen_range(0..=std::cmp::max(0, ground_top(world) - 1)) as f64;
    let ch = art::WIND_CHARS[world.rng.gen_range(0..art::WIND_CHARS.len())];
    let dy = world.rng.gen_range(-1.0..1.0);
    let life = world.rng.gen_range(3.0..6.0);
    Some(
        Entity::new(vec![ch.to_string()])
            .pos(x, y)
            .vel(dx, dy)
            .with_depth(engine::DEPTH_SKY_CREATURE)
            .with_color(Color::Green)
            .with_name("wind_leaf")
            .with_behavior(Behavior::Lifespan { seconds: life, t: 0.0 }),
    )
}

pub fn spawn_lightning(world: &mut World) -> Option<Entity> {
    let snap = world.env_snapshot();
    if !snap.storming {
        return None;
    }
    let bolt_w = max_line_width(art::LIGHTNING);
    let x = world.rng.gen_range(0..=std::cmp::max(0, world.width as i32 - bolt_w)) as f64;
    let y = world.rng.gen_range(0..=world.height as i32 / 4) as f64;
    Some(
        Entity::new(vec![art::LIGHTNING.to_string()])
            .with_mask(vec![art::LIGHTNING_MASK.to_string()])
            .pos(x, y)
            .with_depth(engine::DEPTH_CLOUD)
            .with_color(Color::Yellow)
            .with_name("lightning")
            .with_behavior(Behavior::Lifespan { seconds: 0.3, t: 0.0 }),
    )
}

/// Tire la rangée-plancher (les « pieds ») d'un animal terrestre dans la bande,
/// pour un sprite de hauteur `sprite_h`, et le `depth` de perspective associé.
///
/// L'animal tient entièrement dans la bande (le haut du sprite reste ≥ `ground_top`).
/// Plus haut dans la bande ⇒ plus loin ⇒ `depth` plus grand ⇒ peint avant ⇒ derrière.
/// `depth` est borné à `DEPTH_GRASS - 1` : les animaux restent toujours devant l'herbe.
fn ground_slot(world: &mut World, sprite_h: i32) -> (i32, i32) {
    let top = ground_top(world);
    let bottom = world.height as i32 - 1;
    // Borne basse des pieds : garde le corps entier dans la bande. Si le sprite est
    // plus haut que la bande, on retombe sur `bottom` (plage non vide).
    let lo = std::cmp::min(top + sprite_h - 1, bottom);
    let feet = world.rng.gen_range(lo..=bottom);
    let rank = bottom - feet; // 0 = tout en bas (devant), croît vers le haut (derrière)
    let depth = std::cmp::min(engine::DEPTH_GROUND_ANIMAL + rank, engine::DEPTH_GRASS - 1);
    (feet, depth)
}

fn ground_hopper(
    world: &mut World,
    frames: &[String],
    color: Color,
    name: &str,
    amplitude: f64,
    speed: f64,
    masks: Option<&[String]>,
) -> Entity {
    let height = frames[0].split('\n').count() as i32;
    let (feet, depth) = ground_slot(world, height);
    let y = (feet - (height - 1)) as f64;
    let mut e = cross_factory(
        world, frames, depth, speed, y, color, name, masks, 6.0,
    );
    e.opaque = true; // le corps masque l'herbe derrière lui
    // Hop pose le bas du sprite à `ground_y - 1` ; `feet + 1` place les pieds au
    // repos exactement sur `feet`.
    e.with_behavior(Behavior::Hop {
        ground_y: (feet + 1) as f64,
        amplitude,
        period: 0.4,
        t: 0.0,
    })
}

fn ground_walker(
    world: &mut World,
    frames: &[String],
    color: Color,
    name: &str,
    speed: f64,
    masks: Option<&[String]>,
    frame_rate: f64,
) -> Entity {
    let height = frames[0].split('\n').count() as i32;
    let (feet, depth) = ground_slot(world, height);
    let y = (feet - (height - 1)) as f64; // bas du sprite posé sur `feet`
    let mut e = cross_factory(
        world, frames, depth, speed, y, color, name, masks, frame_rate,
    );
    e.opaque = true;
    e
}

pub fn spawn_rabbit(world: &mut World) -> Option<Entity> {
    let frames: Vec<String> = art::RABBIT.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::RABBIT_MASK.iter().map(|s| s.to_string()).collect();
    let speed = world.rng.gen_range(6.0..9.0);
    Some(ground_hopper(world, &frames, Color::White, "rabbit", 2.0, speed, Some(&masks)))
}

pub fn spawn_fox(world: &mut World) -> Option<Entity> {
    let frames: Vec<String> = art::FOX.iter().map(|s| s.to_string()).collect();
    let masks: Vec<String> = art::FOX_MASK.iter().map(|s| s.to_string()).collect();
    let speed = world.rng.gen_range(5.0..8.0);
    Some(ground_walker(world, &frames, Color::Red, "fox", speed, Some(&masks), 4.0))
}

pub fn spawn_hedgehog(world: &mut World) -> Option<Entity> {
    let frames = vec![art::HEDGEHOG.to_string()];
    let masks = vec![art::HEDGEHOG_MASK.to_string()];
    let speed = world.rng.gen_range(2.0..3.0);
    Some(ground_walker(world, &frames, Color::Brown, "hedgehog", speed, Some(&masks), 0.0))
}

pub fn spawn_mouse(world: &mut World) -> Option<Entity> {
    let frames = vec![art::MOUSE.to_string()];
    let masks = vec![art::MOUSE_MASK.to_string()];
    let speed = world.rng.gen_range(4.0..6.0);
    Some(ground_walker(world, &frames, Color::White, "mouse", speed, Some(&masks), 0.0))
}

pub fn spawn_snail(world: &mut World) -> Option<Entity> {
    let frames = vec![art::SNAIL.to_string()];
    let speed = world.rng.gen_range(0.8..1.5);
    Some(ground_walker(world, &frames, Color::Yellow, "snail", speed, None, 0.0))
}

fn day_target_1(w: &World) -> i32 {
    if is_day(w) { 1 } else { 0 }
}
fn day_target_3(w: &World) -> i32 {
    if is_day(w) { 3 } else { 0 }
}
fn night_target_1(w: &World) -> i32 {
    if is_night(w) { 1 } else { 0 }
}
fn night_target_6(w: &World) -> i32 {
    if is_night(w) { 6 } else { 0 }
}
fn star_target(w: &World) -> i32 {
    if is_night(w) {
        std::cmp::max(3, w.width as i32 / 8)
    } else {
        0
    }
}
fn rain_target(w: &World) -> i32 {
    let snap = w.env_snapshot();
    if !snap.raining {
        return 0;
    }
    let base = std::cmp::max(6, w.width as i32 / 3);
    if snap.storming {
        base * 2 // l'orage intensifie la pluie
    } else {
        base
    }
}
fn wind_leaf_target(w: &World) -> i32 {
    let snap = w.env_snapshot();
    if snap.windy && !snap.raining {
        4
    } else {
        0
    }
}
fn lightning_target(w: &World) -> i32 {
    if w.env_snapshot().storming {
        1
    } else {
        0
    }
}

pub fn register_spawners(spawner: &mut Spawner) {
    // Corps célestes (gatés jour/nuit)
    spawner.register("sun", spawn_sun, Target::Dynamic(day_target_1), 1.0);
    spawner.register("moon", spawn_moon, Target::Dynamic(night_target_1), 1.0);
    spawner.register("star", spawn_star, Target::Dynamic(star_target), 2.0);
    // Résidents de l'arbre / créatures gatées
    spawner.register("owl", spawn_owl, Target::Dynamic(night_target_1), 1.0);
    spawner.register("bee", spawn_bee, Target::Dynamic(day_target_3), 0.8);
    spawner.register("firefly", spawn_firefly, Target::Dynamic(night_target_6), 1.5);
    // Ciel
    spawner.register("cloud", spawn_cloud, Target::Fixed(3), 0.3);
    spawner.register("bird", spawn_bird, Target::Fixed(5), 0.5);
    spawner.register("butterfly", spawn_butterfly, Target::Dynamic(day_target_3), 0.4);
    // Objets qui tombent
    spawner.register("apple", spawn_apple, Target::Fixed(2), 0.2);
    // Sol
    spawner.register("rabbit", spawn_rabbit, Target::Fixed(2), 0.3);
    spawner.register("fox", spawn_fox, Target::Fixed(1), 0.1);
    spawner.register("hedgehog", spawn_hedgehog, Target::Fixed(1), 0.15);
    spawner.register("mouse", spawn_mouse, Target::Fixed(2), 0.3);
    spawner.register("snail", spawn_snail, Target::Fixed(1), 0.1);
    // Météo
    spawner.register("rain", spawn_raindrop, Target::Dynamic(rain_target), 25.0);
    spawner.register("wind_leaf", spawn_wind_leaf, Target::Dynamic(wind_leaf_target), 3.0);
    spawner.register("lightning", spawn_lightning, Target::Dynamic(lightning_target), 1.5);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art;
    use crate::engine::{self, World};
    use crate::environment::Environment;

    #[test]
    fn ground_rows_scales_and_clamps() {
        assert_eq!(ground_rows(&World::new(40, 20)), 6); // max(6, 20/4=5) => 6
        assert_eq!(ground_rows(&World::new(80, 40)), 10); // 40/4 = 10
        assert_eq!(ground_rows(&World::new(20, 8)), 4); // plafond height/2 = 4
    }

    #[test]
    fn ground_top_leaves_band() {
        let w = World::new(40, 20);
        assert_eq!(ground_top(&w), 14); // bande = ground_rows = 6, 20 - 6
    }

    #[test]
    fn tree_art_selects_variant_by_size() {
        let small = World::new(80, 24); // hauteur < 28 => petit
        let large = World::new(100, 35);
        let narrow = World::new(39, 35); // trop étroit => petit
        // Comparaison par contenu : TREE_SMALL et TREE_LARGE diffèrent, donc
        // l'égalité de valeur identifie sans ambiguïté la variante retournée.
        // (Pas `std::ptr::eq` : l'adresse de deux `&'static str` identiques
        // n'est pas garantie stable selon le découpage en unités de codegen.)
        assert_eq!(tree_art(&small).0, art::TREE_SMALL);
        assert_eq!(tree_art(&large).0, art::TREE_LARGE);
        assert_eq!(tree_art(&narrow).0, art::TREE_SMALL);
    }

    #[test]
    fn tree_is_centered() {
        let w = World::new(60, 24);
        let (ox, _) = tree_origin(&w);
        let (frame, _) = tree_art(&w);
        let tree_w = max_line_width(frame);
        let center = ox as f64 + tree_w as f64 / 2.0;
        assert!((center - 30.0).abs() <= 1.0);
    }

    #[test]
    fn large_tree_fits_on_screen() {
        let w = World::new(100, 35);
        let (ox, oy) = tree_origin(&w);
        assert!(oy >= 0);
        assert!(ox >= 0);
    }

    #[test]
    fn ground_frames_fill_all_rows() {
        let frames = ground_frames(30, 7);
        assert_eq!(frames.len(), 2);
        for f in &frames {
            let lines: Vec<&str> = f.split('\n').collect();
            assert_eq!(lines.len(), 7);
            for line in lines {
                assert_eq!(line.chars().count(), 30);
                assert!(!line.trim().is_empty());
            }
        }
    }

    #[test]
    fn build_meadow_adds_scenery() {
        let mut w = World::seeded(60, 24, 0);
        build_meadow(&mut w, 90.0);
        let names: std::collections::HashSet<&str> =
            w.entities.iter().filter_map(|e| e.name.as_deref()).collect();
        assert!(names.contains("tree"));
        assert!(names.contains("ground"));
    }

    #[test]
    fn build_meadow_installs_environment() {
        let mut w = World::seeded(60, 24, 0);
        build_meadow(&mut w, 90.0);
        assert!(w.env.is_some());
    }

    #[test]
    fn ground_spans_full_width() {
        let mut w = World::seeded(50, 20, 0);
        build_meadow(&mut w, 90.0);
        let ground = w.entities.iter().find(|e| e.name.as_deref() == Some("ground")).unwrap();
        assert_eq!(ground.width(), 50);
    }

    #[test]
    fn ground_band_at_grass_depth() {
        let mut w = World::seeded(50, 20, 0);
        build_meadow(&mut w, 90.0);
        let ground = w.entities.iter().find(|e| e.name.as_deref() == Some("ground")).unwrap();
        assert_eq!(ground.depth, engine::DEPTH_GRASS);
    }

    #[test]
    fn flowers_spread_across_band() {
        let mut w = World::seeded(60, 24, 5);
        build_meadow(&mut w, 90.0);
        let gy = ground_top(&w) as f64;
        let flowers: Vec<_> = w.entities.iter().filter(|e| e.name.as_deref() == Some("flower")).collect();
        assert!(!flowers.is_empty());
        assert!(flowers.iter().all(|e| gy <= e.y && e.y <= (w.height as f64 - 1.0)));
    }

    #[test]
    fn cross_factory_sets_direction_and_depth() {
        let mut w = World::seeded(60, 24, 1);
        let b = spawn_bird(&mut w).unwrap();
        assert_eq!(b.name.as_deref(), Some("bird"));
        assert!(b.dx != 0.0);
        assert_eq!(b.depth, engine::DEPTH_SKY_CREATURE);
        assert!(b.color_mask.is_some());
    }

    #[test]
    fn owl_is_resident_and_perched() {
        for (wd, ht) in [(60, 24), (100, 35)] {
            let mut w = World::seeded(wd, ht, 3);
            let o = spawn_owl(&mut w).unwrap();
            assert_eq!(o.dx, 0.0);
            assert_eq!(o.dy, 0.0);
            let (_, toy) = tree_origin(&w);
            let (frame, _) = tree_art(&w);
            let tree_h = frame.split('\n').count() as i32;
            assert!(o.y as i32 > toy && o.y as i32 <= toy + tree_h / 2 + 1);
            assert!(o.color_mask.is_some());
        }
    }

    #[test]
    fn owl_cull_kills_when_day_returns() {
        let mut w = World::new(80, 24);
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0); // nuit
        w.env = Some(env);
        let mut o = spawn_owl(&mut w).unwrap();
        o.advance(0.1, w.env_snapshot());
        assert!(o.alive); // la nuit : le hibou vit
        w.env.as_mut().unwrap().t = 0.0; // bascule jour
        o.advance(0.1, w.env_snapshot());
        assert!(!o.alive); // culled quand la nuit se ferme
    }

    #[test]
    fn butterfly_and_bee_are_day_gated_movers() {
        let mut w = World::seeded(80, 24, 2);
        let bf = spawn_butterfly(&mut w).unwrap();
        assert_eq!(bf.name.as_deref(), Some("butterfly"));
        assert_eq!(bf.behaviors.len(), 2); // zigzag + env cull
        let bee = spawn_bee(&mut w).unwrap();
        assert_eq!(bee.name.as_deref(), Some("bee"));
        assert_eq!(bee.behaviors.len(), 2); // orbit + env cull
    }

    #[test]
    fn is_day_night_track_env() {
        let mut day = World::new(80, 24);
        day.env = Some(Environment::seeded(100.0, 0)); // phase 0 => jour
        assert!(is_day(&day));
        assert!(!is_night(&day));
        let mut night = World::new(80, 24);
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0); // phase 0.5 => nuit
        night.env = Some(env);
        assert!(is_night(&night));
        assert!(!is_day(&night));
        let no_env = World::new(80, 24);
        assert!(is_day(&no_env)); // pas d'env => jour
        assert!(!is_night(&no_env));
    }

    #[test]
    fn apple_spawns_in_canopy_and_falls() {
        let mut w = World::seeded(60, 24, 0);
        let a = spawn_apple(&mut w).unwrap();
        assert_eq!(a.name.as_deref(), Some("apple"));
        assert_eq!(a.behaviors.len(), 1); // fall
        let (_, toy) = tree_origin(&w);
        let (frame, _) = tree_art(&w);
        let h = frame.split('\n').count() as f64;
        assert!(toy as f64 <= a.y && a.y <= toy as f64 + h);
    }

    #[test]
    fn rain_spawns_only_when_raining() {
        use crate::environment::Weather;
        let mut w = World::new(80, 24);
        w.env = Some(Environment::seeded(100.0, 0));
        w.env.as_mut().unwrap().weather = Weather::Clear;
        assert!(spawn_raindrop(&mut w).is_none());
        w.env.as_mut().unwrap().weather = Weather::Rain;
        let e = spawn_raindrop(&mut w).unwrap();
        assert_eq!(e.name.as_deref(), Some("rain"));
        assert!(e.dy > 0.0);
    }

    #[test]
    fn lightning_spawns_only_when_storming() {
        use crate::environment::Weather;
        let mut w = World::new(80, 24);
        w.env = Some(Environment::seeded(100.0, 0));
        w.env.as_mut().unwrap().weather = Weather::Rain;
        assert!(spawn_lightning(&mut w).is_none());
        w.env.as_mut().unwrap().weather = Weather::Storm;
        assert!(spawn_lightning(&mut w).is_some());
    }

    #[test]
    fn ground_slot_keeps_body_in_band_and_orders_depth() {
        let mut w = World::seeded(60, 24, 7);
        let top = ground_top(&w);
        let bottom = w.height as i32 - 1;
        let sprite_h = 2;
        for _ in 0..50 {
            let (feet, depth) = ground_slot(&mut w, sprite_h);
            // pieds dans la bande, corps entier dans la bande
            assert!(feet <= bottom, "pieds sous l'écran");
            assert!(feet - (sprite_h - 1) >= top, "corps au-dessus de la bande");
            // depth dans la fenêtre de perspective, toujours devant l'herbe
            assert!(depth >= engine::DEPTH_GROUND_ANIMAL);
            assert!(depth < engine::DEPTH_GRASS);
            // plus haut (feet plus petit) => plus loin => depth plus grand
            let rank = bottom - feet;
            let expected = std::cmp::min(engine::DEPTH_GROUND_ANIMAL + rank, engine::DEPTH_GRASS - 1);
            assert_eq!(depth, expected);
        }
    }

    #[test]
    fn walkers_feet_within_band() {
        let mut w = World::seeded(60, 24, 3);
        let top = ground_top(&w);
        for factory in [spawn_fox, spawn_hedgehog, spawn_mouse] {
            let e = factory(&mut w).unwrap();
            let feet = e.y as i32 + e.height() as i32 - 1; // rangée du bas du sprite
            assert!(top <= e.y as i32, "corps au-dessus de la bande");
            assert!(feet < w.height as i32, "pieds sous l'écran");
            assert!(e.depth >= engine::DEPTH_GROUND_ANIMAL && e.depth < engine::DEPTH_GRASS);
        }
    }

    #[test]
    fn ground_animals_are_opaque_and_masked() {
        let mut w = World::seeded(60, 24, 3);
        for factory in [spawn_rabbit, spawn_fox, spawn_hedgehog, spawn_mouse] {
            let e = factory(&mut w).unwrap();
            assert!(e.opaque);
            assert!(e.color_mask.is_some());
        }
        assert!(spawn_snail(&mut w).unwrap().opaque); // snail opaque mais sans masque
    }

    use crate::spawn::Spawner;

    fn spec_target(sp: &Spawner, name: &str, w: &World) -> i32 {
        sp.specs.iter().find(|s| s.name == name).unwrap().target.resolve(w)
    }

    fn day_world() -> World {
        let mut w = World::seeded(80, 24, 0);
        w.env = Some(Environment::seeded(100.0, 0)); // phase 0 => jour
        w
    }
    fn night_world() -> World {
        let mut w = World::seeded(80, 24, 0);
        let mut env = Environment::seeded(100.0, 0);
        env.update(50.0); // phase 0.5 => nuit
        w.env = Some(env);
        w
    }

    #[test]
    fn owl_target_night_only() {
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        assert_eq!(spec_target(&sp, "owl", &day_world()), 0);
        assert_eq!(spec_target(&sp, "owl", &night_world()), 1);
    }

    #[test]
    fn day_creatures_target_zero_at_night() {
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        let night = night_world();
        assert_eq!(spec_target(&sp, "bee", &night), 0);
        assert_eq!(spec_target(&sp, "butterfly", &night), 0);
    }

    #[test]
    fn moon_target_night_sun_target_day() {
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        let (day, night) = (day_world(), night_world());
        assert_eq!(spec_target(&sp, "sun", &day), 1);
        assert_eq!(spec_target(&sp, "sun", &night), 0);
        assert_eq!(spec_target(&sp, "moon", &night), 1);
        assert_eq!(spec_target(&sp, "moon", &day), 0);
    }

    #[test]
    fn register_spawners_populates_day_world() {
        let mut w = World::seeded(80, 24, 0);
        build_meadow(&mut w, 90.0); // env de jour (phase 0)
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        for _ in 0..200 {
            sp.tick(&mut w, 0.1);
        }
        let names: std::collections::HashSet<&str> =
            w.entities.iter().filter_map(|e| e.name.as_deref()).collect();
        assert!(names.contains("bee")); // créature de jour
        assert!(names.contains("sun")); // soleil day-gated
        assert!(!names.contains("owl")); // nocturne absent de jour
    }

    #[test]
    fn moon_replaces_sun_at_night_in_full_step() {
        let mut w = World::seeded(80, 24, 0);
        build_meadow(&mut w, 90.0);
        let dl = w.env.as_ref().unwrap().day_length;
        w.env.as_mut().unwrap().update(dl * 0.5); // bascule nuit
        let mut sp = Spawner::new();
        register_spawners(&mut sp);
        for _ in 0..60 {
            sp.tick(&mut w, 0.1); // tick seul : ne fait pas avancer l'horloge
        }
        let names: std::collections::HashSet<&str> =
            w.entities.iter().filter_map(|e| e.name.as_deref()).collect();
        assert!(names.contains("moon"));
        assert!(!names.contains("sun"));
    }
}
