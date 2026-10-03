//! Pure application state machine and menu model (no rendering, no input devices).

/// Number of campaign missions (see `STORY.md`).
pub const MISSION_COUNT: u8 = 10;

/// Top-level screens of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Boot,
    MainMenu,
    Briefing { mission: u8 },
    Space { mission: u8 },
    Boarding { mission: u8 },
    Debrief { mission: u8, won: bool },
    Quit,
}

/// Things that can happen to the app; the only way to change [`AppState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    BootDone,
    NewGame,
    BriefingDone,
    BeginBoarding,
    EndBoarding,
    MissionEnded {
        won: bool,
    },
    /// Leave the debrief: next mission after a win, retry after a loss.
    DebriefDone,
    ToMenu,
    QuitRequested,
}

impl AppState {
    /// Apply an event. Events that make no sense in the current state are ignored,
    /// except `QuitRequested`, which always quits.
    pub fn next(self, event: AppEvent) -> AppState {
        use AppEvent as E;
        use AppState as S;
        match (self, event) {
            (_, E::QuitRequested) => S::Quit,
            (S::Boot, E::BootDone) => S::MainMenu,
            (S::MainMenu, E::NewGame) => S::Briefing { mission: 1 },
            (S::Briefing { mission }, E::BriefingDone) => S::Space { mission },
            (S::Space { mission }, E::BeginBoarding) => S::Boarding { mission },
            (S::Boarding { mission }, E::EndBoarding) => S::Space { mission },
            (S::Space { mission } | S::Boarding { mission }, E::MissionEnded { won }) => {
                S::Debrief { mission, won }
            }
            (S::Debrief { mission, won }, E::DebriefDone) => {
                if !won {
                    S::Briefing { mission }
                } else if mission >= MISSION_COUNT {
                    S::MainMenu
                } else {
                    S::Briefing {
                        mission: mission + 1,
                    }
                }
            }
            (S::Boot | S::Quit, E::ToMenu) => self,
            (_, E::ToMenu) => S::MainMenu,
            (state, _) => state,
        }
    }
}

/// Entries of the main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    Continue,
    Settings,
    Quit,
}

/// One selectable line in a menu.
#[derive(Debug, Clone, Copy)]
pub struct MenuItem {
    pub label: &'static str,
    pub action: MenuAction,
    pub enabled: bool,
}

/// Keyboard/mouse driven vertical menu; disabled items are skipped.
#[derive(Debug, Clone)]
pub struct Menu {
    items: Vec<MenuItem>,
    selected: usize,
}

impl Menu {
    /// The main menu. Continue (T28) and Settings (T28) are disabled until they exist.
    pub fn main() -> Menu {
        let item = |label, action, enabled| MenuItem {
            label,
            action,
            enabled,
        };
        Menu {
            items: vec![
                item("New Game", MenuAction::NewGame, true),
                item("Continue", MenuAction::Continue, false),
                item("Settings", MenuAction::Settings, false),
                item("Quit", MenuAction::Quit, true),
            ],
            selected: 0,
        }
    }

    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Move the selection by `delta` (wrapping), skipping disabled items.
    pub fn step(&mut self, delta: i32) {
        let len = self.items.len() as i32;
        if len == 0 || delta == 0 {
            return;
        }
        let dir = delta.signum();
        let mut i = self.selected as i32;
        for _ in 0..len {
            i = (i + dir).rem_euclid(len);
            if self.items[i as usize].enabled {
                self.selected = i as usize;
                return;
            }
        }
    }

    /// Select item `index` if it exists and is enabled; returns whether it changed.
    pub fn select(&mut self, index: usize) -> bool {
        match self.items.get(index) {
            Some(item) if item.enabled => {
                self.selected = index;
                true
            }
            _ => false,
        }
    }

    /// Action of the selected item.
    pub fn activate(&self) -> Option<MenuAction> {
        self.items
            .get(self.selected)
            .filter(|item| item.enabled)
            .map(|item| item.action)
    }
}

/// Map a menu action to the app event it triggers (if any).
pub fn menu_event(action: MenuAction) -> Option<AppEvent> {
    match action {
        MenuAction::NewGame => Some(AppEvent::NewGame),
        MenuAction::Quit => Some(AppEvent::QuitRequested),
        MenuAction::Continue | MenuAction::Settings => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_to_menu_to_first_briefing() {
        let s = AppState::Boot.next(AppEvent::BootDone);
        assert_eq!(s, AppState::MainMenu);
        assert_eq!(s.next(AppEvent::NewGame), AppState::Briefing { mission: 1 });
    }

    #[test]
    fn board_and_return_keeps_mission() {
        let s = AppState::Space { mission: 4 };
        let b = s.next(AppEvent::BeginBoarding);
        assert_eq!(b, AppState::Boarding { mission: 4 });
        assert_eq!(b.next(AppEvent::EndBoarding), s);
    }

    #[test]
    fn win_advances_loss_retries_final_win_returns_to_menu() {
        let won = AppState::Space { mission: 3 }.next(AppEvent::MissionEnded { won: true });
        assert_eq!(
            won.next(AppEvent::DebriefDone),
            AppState::Briefing { mission: 4 }
        );
        let lost = AppState::Boarding { mission: 3 }.next(AppEvent::MissionEnded { won: false });
        assert_eq!(
            lost.next(AppEvent::DebriefDone),
            AppState::Briefing { mission: 3 }
        );
        let end = AppState::Space {
            mission: MISSION_COUNT,
        }
        .next(AppEvent::MissionEnded { won: true })
        .next(AppEvent::DebriefDone);
        assert_eq!(end, AppState::MainMenu);
    }

    #[test]
    fn nonsense_events_are_ignored_and_quit_always_works() {
        assert_eq!(
            AppState::MainMenu.next(AppEvent::BriefingDone),
            AppState::MainMenu
        );
        assert_eq!(AppState::Boot.next(AppEvent::ToMenu), AppState::Boot);
        assert_eq!(
            AppState::Space { mission: 2 }.next(AppEvent::ToMenu),
            AppState::MainMenu
        );
        assert_eq!(AppState::Boot.next(AppEvent::QuitRequested), AppState::Quit);
        assert_eq!(AppState::Quit.next(AppEvent::NewGame), AppState::Quit);
    }

    #[test]
    fn menu_skips_disabled_items_and_wraps() {
        let mut m = Menu::main();
        assert_eq!(m.activate(), Some(MenuAction::NewGame));
        m.step(1);
        assert_eq!(m.activate(), Some(MenuAction::Quit));
        m.step(1);
        assert_eq!(m.activate(), Some(MenuAction::NewGame));
        m.step(-1);
        assert_eq!(m.activate(), Some(MenuAction::Quit));
    }

    #[test]
    fn menu_select_rejects_disabled_and_out_of_range() {
        let mut m = Menu::main();
        assert!(!m.select(1));
        assert!(!m.select(99));
        assert!(m.select(3));
        assert_eq!(m.selected(), 3);
        assert_eq!(menu_event(MenuAction::Settings), None);
        assert_eq!(menu_event(MenuAction::Quit), Some(AppEvent::QuitRequested));
    }
}
