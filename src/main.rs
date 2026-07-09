//! Point d'entrée : coquille crossterm + boucle principale (seul module terminal-aware).

use std::io::{Stdout, Write};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color as TColor, Print, SetForegroundColor};
use crossterm::{cursor, execute, queue, terminal};

use asciimeadow::engine::{Buffer, Color, World};
use asciimeadow::scene;
use asciimeadow::spawn::{step, Spawner};
use rand::rngs::StdRng;
use rand::SeedableRng;

const FPS: u32 = 20;

pub struct Args {
    pub seed: Option<u64>,
    pub fps: u32,
    pub day_length: f64,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut seed: Option<u64> = None;
    let mut fps: u32 = FPS;
    let mut day_length: f64 = 90.0;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                let v = args.get(i).ok_or("--seed requires a value")?;
                seed = Some(v.parse::<u64>().map_err(|_| "--seed must be an integer")?);
            }
            "--fps" => {
                i += 1;
                let v = args.get(i).ok_or("--fps requires a value")?;
                fps = v.parse::<u32>().map_err(|_| "--fps must be an integer")?;
            }
            "--day-length" => {
                i += 1;
                let v = args.get(i).ok_or("--day-length requires a value")?;
                day_length = v
                    .parse::<f64>()
                    .map_err(|_| "--day-length must be a number")?;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }
    if fps < 1 {
        return Err("--fps must be >= 1".into());
    }
    if day_length <= 0.0 {
        return Err("--day-length must be > 0".into());
    }
    Ok(Args { seed, fps, day_length })
}

/// Couleur logique -> couleur crossterm. Pas de brun natif : DarkYellow.
fn term_color(c: Color) -> TColor {
    match c {
        Color::White => TColor::White,
        Color::Green => TColor::Green,
        Color::Brown => TColor::DarkYellow,
        Color::Yellow => TColor::Yellow,
        Color::Red => TColor::Red,
        Color::Cyan => TColor::Cyan,
        Color::Blue => TColor::Blue,
        Color::Magenta => TColor::Magenta,
        Color::Black => TColor::Black,
    }
}

/// Rétablit le terminal quoi qu'il arrive (sortie normale, `q`, Ctrl+C, panic).
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// Affichage double-buffer : ne réécrit que les cellules changées.
struct Display {
    out: Stdout,
    prev: Option<Buffer>,
}

impl Display {
    fn new() -> Self {
        Display {
            out: std::io::stdout(),
            prev: None,
        }
    }

    fn force_repaint(&mut self) {
        self.prev = None;
    }

    fn draw(&mut self, buf: &Buffer) -> std::io::Result<()> {
        for y in 0..buf.height {
            for x in 0..buf.width {
                let ch = buf.chars[y][x];
                let col = buf.colors[y][x];
                // Cellule à redessiner si taille différente ou contenu changé.
                let changed = match &self.prev {
                    Some(p) if p.width == buf.width && p.height == buf.height => {
                        p.chars[y][x] != ch || p.colors[y][x] != col
                    }
                    _ => true,
                };
                if changed {
                    queue!(
                        self.out,
                        cursor::MoveTo(x as u16, y as u16),
                        SetForegroundColor(term_color(col)),
                        Print(ch)
                    )?;
                }
            }
        }
        self.out.flush()?;
        self.prev = Some(buf.clone());
        Ok(())
    }
}

fn build_world(
    width: usize,
    height: usize,
    seed: Option<u64>,
    day_length: f64,
) -> (World, Spawner) {
    let rng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };
    // Garde-fou : un terminal peut transitoirement rapporter une dimension 0 ;
    // un monde 0×0 ferait paniquer la construction de la scène.
    let mut world = World::with_rng(width.max(1), height.max(1), rng);
    scene::build_meadow(&mut world, day_length);
    let mut spawner = Spawner::new();
    scene::register_spawners(&mut spawner);
    (world, spawner)
}

fn run(args: Args) -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    // Armer le guard AVANT tout autre appel terminal faillible : si `execute!`
    // ci-dessous échoue, le mode raw doit quand même être rétabli à la sortie.
    let _guard = TerminalGuard;
    let mut out = std::io::stdout();
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;

    let (mut cols, mut rows) = terminal::size()?;
    let (mut world, mut spawner) =
        build_world(cols as usize, rows as usize, args.seed, args.day_length);

    let mut disp = Display::new();
    let dt = 1.0 / args.fps as f64;
    let frame = std::time::Duration::from_secs_f64(dt);
    let mut paused = false;

    loop {
        // La fenêtre de poll fait office de cadence (comme le timeout curses).
        if event::poll(frame)? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('p') | KeyCode::Char('P') => paused = !paused,
                    KeyCode::Char('r') | KeyCode::Char('R') => disp.force_repaint(),
                    _ => {}
                },
                Event::Resize(w, h) => {
                    cols = w;
                    rows = h;
                    let rebuilt =
                        build_world(cols as usize, rows as usize, args.seed, args.day_length);
                    world = rebuilt.0;
                    spawner = rebuilt.1;
                    disp.force_repaint();
                }
                _ => {}
            }
        }
        if !paused {
            step(&mut world, &spawner, dt);
        }
        disp.draw(&world.render())?;
    }
    Ok(())
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("asciimeadow: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(args) {
        eprintln!("asciimeadow: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn day_length_defaults_to_90() {
        let a = parse_args(&args(&[])).unwrap();
        assert_eq!(a.day_length, 90.0);
        assert_eq!(a.fps, 20);
        assert_eq!(a.seed, None);
    }

    #[test]
    fn day_length_parsed() {
        let a = parse_args(&args(&["--day-length", "30"])).unwrap();
        assert_eq!(a.day_length, 30.0);
    }

    #[test]
    fn seed_and_fps_parsed() {
        let a = parse_args(&args(&["--seed", "7", "--fps", "15"])).unwrap();
        assert_eq!(a.seed, Some(7));
        assert_eq!(a.fps, 15);
    }

    #[test]
    fn rejects_bad_fps_and_day_length() {
        assert!(parse_args(&args(&["--fps", "0"])).is_err());
        assert!(parse_args(&args(&["--day-length", "0"])).is_err());
        assert!(parse_args(&args(&["--seed", "x"])).is_err());
        assert!(parse_args(&args(&["--unknown"])).is_err());
    }

    #[test]
    fn term_color_covers_all_palette() {
        use asciimeadow::engine::COLOR_NAMES;
        for c in COLOR_NAMES {
            let _ = term_color(c);
        }
    }
}
