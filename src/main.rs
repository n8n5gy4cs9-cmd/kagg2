use macroquad::prelude::*;

use kagga2::app::{AppEvent, AppState, Menu, menu_event};
use kagga2::assets::Assets;
use kagga2::gfx::board_scene::BoardScene;
use kagga2::gfx::render3d::Renderer;
use kagga2::gfx::sound::Mixer;
use kagga2::gfx::space_scene::SpaceScene;
use kagga2::gfx::starfield::Starfield;
use kagga2::gfx::viewer::Viewer;
use kagga2::gfx::{pixfont, screens};
use kagga2::sim::SectorId;

const BOOT_SECONDS: f64 = 1.2;

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Kagga2".to_owned(),
            window_width: 1280,
            window_height: 720,
            window_resizable: true,
            ..Default::default()
        },
        // Planet and sky meshes are bigger than the 10k-vertex default draw call.
        draw_call_vertex_capacity: 60_000,
        draw_call_index_capacity: 120_000,
        ..Default::default()
    }
}

/// Read the keyboard for the current state and return the resulting event, if any.
fn state_input(state: AppState, menu: &mut Menu, boot_started: f64) -> Option<AppEvent> {
    let confirm = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space);
    match state {
        AppState::Boot => {
            (confirm || get_time() - boot_started > BOOT_SECONDS).then_some(AppEvent::BootDone)
        }
        AppState::MainMenu => {
            if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                menu.step(1);
            }
            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                menu.step(-1);
            }
            let (mx, my) = mouse_position();
            for i in 0..menu.items().len() {
                if screens::menu_item_rect(i).contains(vec2(mx, my)) && menu.select(i) {
                    if is_mouse_button_pressed(MouseButton::Left) {
                        return menu.activate().and_then(menu_event);
                    }
                    break;
                }
            }
            if is_key_pressed(KeyCode::Enter) {
                return menu.activate().and_then(menu_event);
            }
            is_key_pressed(KeyCode::Escape).then_some(AppEvent::QuitRequested)
        }
        AppState::Briefing { .. } => {
            if is_key_pressed(KeyCode::Escape) {
                Some(AppEvent::ToMenu)
            } else {
                confirm.then_some(AppEvent::BriefingDone)
            }
        }
        // Space flight is T05; boarding is still a placeholder until T18. Esc leaves, B boards, F9 wins.
        AppState::Space { .. } => {
            if is_key_pressed(KeyCode::Escape) {
                Some(AppEvent::ToMenu)
            } else if is_key_pressed(KeyCode::F9) {
                Some(AppEvent::MissionEnded { won: true })
            } else {
                None
            }
        }
        // The boarding scene reads its own keys (Esc inside it aborts back to the ship).
        AppState::Boarding { .. } => None,
        AppState::Debrief { .. } => confirm.then_some(AppEvent::DebriefDone),
        AppState::Quit => None,
    }
}

/// Command-line options for development (`--viewer`, `--space`, `--shot`).
#[derive(Default)]
struct Cli {
    viewer: Option<Option<String>>,
    space: Option<SectorId>,
    shot: Option<String>,
    chase: bool,
    look: Option<Vec3>,
    pos: Option<Vec3>,
    target: bool,
    throttle: Option<f32>,
    frames: u32,
    tab: Option<usize>,
    fight: bool,
    dock: bool,
    fire: bool,
    board: Option<u32>,
    size: u32,
}

fn parse_vec3(s: &str) -> Option<Vec3> {
    let v: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    (v.len() == 3).then(|| vec3(v[0], v[1], v[2]))
}

fn parse_cli() -> Cli {
    let mut cli = Cli {
        frames: 45,
        size: 2,
        ..Default::default()
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let next = args.get(i + 1).filter(|a| !a.starts_with("--")).cloned();
        match args[i].as_str() {
            "--viewer" => cli.viewer = Some(next.clone()),
            "--space" => {
                cli.space = Some(
                    next.as_deref()
                        .and_then(SectorId::parse)
                        .unwrap_or(SectorId::MarsOrbit),
                )
            }
            "--shot" => cli.shot = next.clone(),
            "--chase" => cli.chase = true,
            "--target" => cli.target = true,
            "--fight" => cli.fight = true,
            "--dock" => cli.dock = true,
            "--fire" => cli.fire = true,
            "--board" => cli.board = next.as_deref().and_then(|n| n.parse().ok()).or(Some(1)),
            "--size" => cli.size = next.as_deref().and_then(|n| n.parse().ok()).unwrap_or(2),
            "--tab" => cli.tab = next.as_deref().and_then(|n| n.parse().ok()),
            "--look" => cli.look = next.as_deref().and_then(parse_vec3),
            "--pos" => cli.pos = next.as_deref().and_then(parse_vec3),
            "--throttle" => cli.throttle = next.as_deref().and_then(|n| n.parse().ok()),
            "--frames" => cli.frames = next.as_deref().and_then(|n| n.parse().ok()).unwrap_or(45),
            _ => {}
        }
        if next.is_some()
            && !["--chase", "--target", "--fight", "--dock", "--fire"].contains(&args[i].as_str())
        {
            i += 1;
        }
        i += 1;
    }
    cli
}

fn save_shot(path: &str) {
    let mut img = get_screen_data();
    for px in img.bytes.chunks_exact_mut(4) {
        px[3] = 255;
    }
    img.export_png(path);
}

#[macroquad::main(window_conf)]
async fn main() {
    let cli = parse_cli();
    clear_background(Color::new(0.02, 0.02, 0.05, 1.0));
    draw_text("Loading assets...", 40.0, 60.0, 32.0, WHITE);
    next_frame().await;

    let assets = match Assets::load().await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };
    let mut renderer = match Renderer::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("fatal: cannot build shaders: {e}");
            return;
        }
    };

    let mut mixer = Mixer::new().await;
    let mut music_wanted = true;
    let mut vol_toast = 0.0f32;
    let mut forced: Option<AppEvent> = None;
    let mut board: Option<BoardScene> = None;
    let mut grabbed = false;
    let mut state = AppState::Boot;
    let mut menu = Menu::main();
    let mut stars = Starfield::new(300);
    let boot_started = get_time();
    let mut scene: Option<SpaceScene> = None;
    let mut viewer = cli
        .viewer
        .as_ref()
        .map(|name| Viewer::new(&assets, name.as_deref()));
    let mut frame = 0u32;

    let start_scene = |sector: SectorId, cli: &Cli| {
        let mut sc = SpaceScene::new(sector, 1);
        if let (Some(p), Some(l)) = (cli.pos, cli.look) {
            sc.debug_place(p, l);
        } else if let Some(l) = cli.look {
            sc.debug_place(Vec3::ZERO, l);
        }
        sc.set_chase(cli.chase);
        if let Some(t) = cli.throttle {
            sc.debug_throttle(t);
        }
        if cli.target {
            sc.cycle_target_for_shot();
        }
        if let Some(t) = cli.tab {
            sc.debug_tab(t);
        }
        if cli.fight {
            sc.debug_fight();
        }
        if cli.dock {
            sc.debug_dock();
        }
        if cli.fire {
            sc.debug_autofire();
        }
        sc
    };
    if let Some(t) = cli.board {
        use kagga2::board::Theme;
        let theme = [
            Theme::Corvette,
            Theme::Station,
            Theme::PrisonBarge,
            Theme::DreadnoughtBridge,
        ][(t.max(1) as usize - 1) % 4];
        let mut b = BoardScene::new(kagga2::gfx::board_scene::BoardInfo {
            theme,
            size: cli.size,
            label: "Test Ship".into(),
            target: kagga2::sim::EntityId {
                index: 0,
                generation: 0,
            },
            seed: 11,
        });
        b.auto_start = true;
        board = Some(b);
        state = AppState::Boarding { mission: 1 };
    }
    if let Some(sector) = cli.space {
        scene = Some(start_scene(sector, &cli));
        state = AppState::Space { mission: 1 };
    }

    while state != AppState::Quit {
        let dt = get_frame_time();
        frame += 1;
        if let Some(v) = viewer.as_mut() {
            v.update(&assets, dt);
            if is_key_pressed(KeyCode::Escape) {
                break;
            }
            clear_background(BLACK);
            v.draw(&assets, &mut renderer);
        } else {
            stars.update(dt);
            let mut input_event = state_input(state, &mut menu, boot_started);
            if matches!(state, AppState::Space { .. })
                && is_key_pressed(KeyCode::B)
                && let Some(sc) = scene.as_ref()
                && let Some(info) = sc.boardable()
            {
                board = Some(BoardScene::new(info));
                input_event = Some(AppEvent::BeginBoarding);
            }
            if let Some(event) = input_event.or(forced.take()) {
                let before = state;
                state = state.next(event);
                match (before, state) {
                    (AppState::Briefing { .. }, AppState::Space { mission }) => {
                        scene = Some(start_scene(SectorId::for_mission(mission), &cli));
                    }
                    (_, AppState::Space { .. }) => {}
                    (_, _) => {
                        if !matches!(state, AppState::Boarding { .. }) {
                            scene = None;
                        }
                    }
                }
            }

            clear_background(Color::new(0.02, 0.02, 0.05, 1.0));
            let in_space = matches!(state, AppState::Space { .. });
            if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::Equal) {
                let d = if is_key_pressed(KeyCode::Equal) {
                    0.1
                } else {
                    -0.1
                };
                mixer.vol.master = (mixer.vol.master + d).clamp(0.0, 1.0);
                mixer.refresh();
                vol_toast = 1.5;
            }
            if is_key_pressed(KeyCode::F11) {
                music_wanted = !music_wanted;
            }
            if in_space && music_wanted {
                mixer.start_music();
            } else if mixer.music_on() {
                mixer.stop_music();
            }
            let in_board = matches!(state, AppState::Boarding { .. });
            let want_grab = in_board && board.as_ref().is_some_and(|b| b.wants_grab());
            if want_grab != grabbed {
                set_cursor_grab(want_grab);
                show_mouse(!want_grab);
                grabbed = want_grab;
            }
            if in_board {
                if let Some(b) = board.as_mut() {
                    b.update(dt, &assets, Some(&mixer));
                    b.draw(&assets, &mut renderer);
                    if let Some(res) = b.result().cloned() {
                        if let Some(sc) = scene.as_mut() {
                            sc.apply_boarding(&b.info, &res);
                        }
                        forced = Some(if res.outcome == kagga2::board::Outcome::Died {
                            AppEvent::MissionEnded { won: false }
                        } else {
                            AppEvent::EndBoarding
                        });
                        board = None;
                    }
                }
            } else if in_space {
                if let Some(sc) = scene.as_mut() {
                    sc.update(dt, Some(&mixer));
                    sc.draw(&assets, &mut renderer);
                    if let Some(won) = sc.outcome() {
                        forced = Some(AppEvent::MissionEnded { won });
                    }
                }
            } else {
                stars.draw();
            }
            match state {
                AppState::Boot => screens::draw_boot(),
                AppState::MainMenu => screens::draw_main_menu(&menu),
                AppState::Briefing { mission } => screens::draw_briefing(mission),
                AppState::Space { .. } => {}
                AppState::Boarding { .. } => {}
                AppState::Debrief { mission, won } => screens::draw_debrief(mission, won),
                AppState::Quit => {}
            }
        }
        vol_toast = (vol_toast - dt).max(0.0);
        if vol_toast > 0.0 {
            pixfont::draw(
                &assets,
                &format!("VOLUME {:.0}%", mixer.vol.master * 100.0),
                screen_width() - 190.0,
                screen_height() - 30.0,
                2.0,
                WHITE,
            );
        }
        if get_fps() > 0 && std::env::var_os("KAGGA_FPS").is_some() {
            pixfont::draw(
                &assets,
                &format!("{} FPS", get_fps()),
                screen_width() - 110.0,
                8.0,
                2.0,
                YELLOW,
            );
        }
        if let Some(path) = &cli.shot
            && frame >= cli.frames
        {
            save_shot(path);
            break;
        }
        next_frame().await;
    }
}
