use macroquad::prelude::*;

use kagg2::app::{AppEvent, AppState, Menu, MenuAction, menu_event};
use kagg2::assets::Assets;
use kagg2::gfx::board_scene::BoardScene;
use kagg2::gfx::keys::Bind;
use kagg2::gfx::render3d::Renderer;
use kagg2::gfx::settings_ui::SettingsUi;
use kagg2::gfx::sound::Mixer;
use kagg2::gfx::space_scene::SpaceScene;
use kagg2::gfx::starfield::Starfield;
use kagg2::gfx::viewer::Viewer;
use kagg2::gfx::{pixfont, screens};
use kagg2::save::{self, SaveGame, Settings};
use kagg2::sim::SectorId;
use kagg2::sim::mission::Carry;
use kagg2::story::Ending;

const BOOT_SECONDS: f64 = 1.2;

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: Conf {
            window_title: "Kagg2".to_owned(),
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
fn state_input(
    state: AppState,
    menu: &mut Menu,
    boot_started: f64,
    campaign: &SaveGame,
    has_save: bool,
    help_page: &mut usize,
) -> Option<AppEvent> {
    let activate = |menu: &Menu| match menu.activate() {
        Some(MenuAction::Continue) if has_save => Some(AppEvent::Continue {
            mission: campaign.mission,
        }),
        other => other.and_then(menu_event),
    };
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
                        return activate(menu);
                    }
                    break;
                }
            }
            if is_key_pressed(KeyCode::Enter) {
                return activate(menu);
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
            } else if is_key_pressed(KeyCode::F9) && std::env::var_os("KAGGA_DEV").is_some() {
                // developer skip: KAGGA_DEV=1
                Some(AppEvent::MissionEnded { won: true })
            } else {
                None
            }
        }
        // The boarding scene reads its own keys (Esc inside it aborts back to the ship).
        AppState::Boarding { .. } => None,
        AppState::Debrief { .. } => confirm.then_some(AppEvent::DebriefDone),
        AppState::Settings => None,
        AppState::Help => {
            let n = kagg2::gfx::help::pages().len();
            if is_key_pressed(KeyCode::Right) {
                *help_page = (*help_page + 1) % n;
            }
            if is_key_pressed(KeyCode::Left) {
                *help_page = (*help_page + n - 1) % n;
            }
            is_key_pressed(KeyCode::Escape).then_some(AppEvent::ToMenu)
        }
        AppState::Ending { chosen } => {
            if chosen == 0 {
                if is_key_pressed(KeyCode::Key1) {
                    Some(AppEvent::EndingChosen(Ending::BreakArmada.code()))
                } else if is_key_pressed(KeyCode::Key2) {
                    Some(AppEvent::EndingChosen(Ending::TakeThrone.code()))
                } else {
                    None
                }
            } else {
                confirm.then_some(AppEvent::DebriefDone)
            }
        }
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
    /// AI shield-brothers for `--board` (T33).
    coop: usize,
    /// Mission script to play (T34): `--script file.k2m`.
    script: Option<String>,
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
            "--coop" => cli.coop = next.as_deref().and_then(|n| n.parse().ok()).unwrap_or(1),
            "--script" => cli.script = next.clone(),
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
    let mut cli = parse_cli();
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

    let mut settings = save::load_settings();
    let mut campaign = save::load_game().unwrap_or_else(SaveGame::new_game);
    let mut has_save = save::load_game().is_some();
    let mut settings_ui = SettingsUi::default();
    let mut help_page = 0usize;
    let mut loss_reason: &'static str = "";
    let mut mixer = Mixer::new().await;
    mixer.vol.master = settings.volume;
    mixer.refresh();
    let mut music_wanted = settings.music;
    let mut vol_toast = 0.0f32;
    let mut forced: Option<AppEvent> = None;
    let mut board: Option<BoardScene> = None;
    let mut grabbed = false;
    let mut state = AppState::Boot;
    let mut menu = Menu::main(has_save);
    let mut stars = Starfield::new(300);
    let boot_started = get_time();
    let mut scene: Option<SpaceScene> = None;
    let mut viewer = cli
        .viewer
        .as_ref()
        .map(|name| Viewer::new(&assets, name.as_deref()));
    let mut frame = 0u32;
    let (mut frame_ms_sum, mut frame_ms_max) = (0.0f64, 0.0f64);

    let start_scene = |sector: SectorId, cli: &Cli, settings: &Settings| {
        let mut sc = SpaceScene::new(sector, 1);
        sc.keys = settings.keys.clone();
        sc.invert_y = settings.invert_y;
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
        use kagg2::board::Theme;
        let theme = [
            Theme::Corvette,
            Theme::Station,
            Theme::PrisonBarge,
            Theme::DreadnoughtBridge,
        ][(t.max(1) as usize - 1) % 4];
        let mut b = BoardScene::new(kagg2::gfx::board_scene::BoardInfo {
            theme,
            size: cli.size,
            label: "Test Ship".into(),
            target: kagg2::sim::EntityId {
                index: 0,
                generation: 0,
            },
            seed: 11,
            force: None,
        });
        b.auto_start = true;
        b.partners = cli.coop.min(2);
        board = Some(b);
        state = AppState::Boarding { mission: 1 };
    }
    for (path, res) in kagg2::sim::script::load_dir(&save::missions_dir()) {
        match res {
            Ok(id) => eprintln!("mod mission {id}: {}", path.display()),
            Err(e) => eprintln!("mod mission {}: {e}", path.display()),
        }
    }
    let mut script_mission = None;
    if let Some(path) = &cli.script {
        match kagg2::sim::script::load_file(std::path::Path::new(path)) {
            Ok(id) => {
                cli.space = Some(SectorId::for_mission(id));
                script_mission = Some(id);
            }
            Err(e) => eprintln!("cannot load script {path}: {e}"),
        }
    }
    if let Some(sector) = cli.space {
        let mut sc = start_scene(sector, &cli, &settings);
        if let Some(id) = script_mission {
            sc.start_mission(id, &Carry::default(), settings.difficulty.scale());
        }
        scene = Some(sc);
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
            if state == AppState::Settings && settings_ui.update(&mut settings) {
                if let Err(e) = save::store_settings(&settings) {
                    eprintln!("cannot save settings: {e}");
                }
                mixer.vol.master = settings.volume;
                mixer.refresh();
                music_wanted = settings.music;
                forced = Some(AppEvent::ToMenu);
            }
            let mut input_event = state_input(
                state,
                &mut menu,
                boot_started,
                &campaign,
                has_save,
                &mut help_page,
            );
            if matches!(state, AppState::Space { .. })
                && let Some(sc) = scene.as_ref()
                && is_key_pressed(sc.keys.get(Bind::Board))
                && let Some(info) = sc.boardable()
            {
                let mut b = BoardScene::new(info);
                b.difficulty = settings.difficulty.scale();
                b.sens = settings.mouse_sens;
                b.invert_y = settings.invert_y;
                board = Some(b);
                input_event = Some(AppEvent::BeginBoarding);
            }
            if let Some(event) = input_event.or(forced.take()) {
                let before = state;
                match (event, before) {
                    (AppEvent::NewGame, _) => {
                        campaign = SaveGame::new_game();
                        help_page = 0;
                    }
                    (AppEvent::MissionEnded { won }, AppState::Space { mission })
                    | (AppEvent::MissionEnded { won }, AppState::Boarding { mission }) => {
                        if let Some(sc) = scene.as_ref() {
                            if won && let Some(c) = Carry::from_world(&sc.world) {
                                campaign.carry = c;
                            }
                            if !sc.loss_reason().is_empty() {
                                loss_reason = sc.loss_reason();
                            }
                        }
                        if won {
                            loss_reason = "";
                            campaign.mission = (mission + 1).min(kagg2::app::MISSION_COUNT);
                            has_save = true;
                            if let Err(e) = save::store_game(&campaign) {
                                eprintln!("cannot save game: {e}");
                            }
                        }
                    }
                    (AppEvent::EndingChosen(c), _) => {
                        campaign.ending = c;
                        if let Err(e) = save::store_game(&campaign) {
                            eprintln!("cannot save game: {e}");
                        }
                    }
                    _ => {}
                }
                state = state.next(event);
                if state == AppState::MainMenu && before != AppState::MainMenu {
                    menu = Menu::main(has_save);
                }
                if state == AppState::Settings {
                    settings_ui = SettingsUi::default();
                }
                match (before, state) {
                    (AppState::Briefing { .. }, AppState::Space { mission }) => {
                        let mut sc = start_scene(SectorId::for_mission(mission), &cli, &settings);
                        sc.start_mission(mission, &campaign.carry, settings.difficulty.scale());
                        scene = Some(sc);
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
                settings.volume = mixer.vol.master;
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
                        forced = Some(if res.outcome == kagg2::board::Outcome::Died {
                            loss_reason = "YOUR BOARDING PARTY WAS LOST";
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
                AppState::Debrief { mission, won } => {
                    screens::draw_debrief(mission, won, loss_reason)
                }
                AppState::Settings => settings_ui.draw(&settings),
                AppState::Help => screens::draw_help(help_page),
                AppState::Ending { chosen } => screens::draw_ending(chosen),
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
        if frame > 10 {
            frame_ms_sum += f64::from(dt) * 1000.0;
            frame_ms_max = frame_ms_max.max(f64::from(dt) * 1000.0);
        }
        if let Some(path) = &cli.shot
            && frame >= cli.frames
        {
            // frame timing for the T31 performance check (vsync caps the average near 16.7 ms)
            let n = f64::from(frame.saturating_sub(10)).max(1.0);
            eprintln!(
                "frames {frame}: avg {:.2} ms, worst {:.2} ms",
                frame_ms_sum / n,
                frame_ms_max
            );
            save_shot(path);
            break;
        }
        next_frame().await;
    }
    if let Err(e) = save::store_settings(&settings) {
        eprintln!("cannot save settings: {e}");
    }
}
