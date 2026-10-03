//! Pure application state machine and menu model (no rendering, no input devices).

/// Number of campaign missions (see `STORY.md`).
pub const MISSION_COUNT: u8 = 10;

/// Top-level screens of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Boot,
    MainMenu,
    Briefing {
        mission: u8,
    },
    Space {
        mission: u8,
    },
    Boarding {
        mission: u8,
    },
    Debrief {
        mission: u8,
        won: bool,
    },
    Settings,
    Help,
    /// Campaign finished; `chosen` is `Ending::code` (0 = not chosen yet).
    Ending {
        chosen: u8,
    },
    Quit,
}

/// Things that can happen to the app; the only way to change [`AppState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    BootDone,
    NewGame,
    /// Resume the saved campaign at this mission.
    Continue {
        mission: u8,
    },
    OpenSettings,
    OpenHelp,
    EndingChosen(u8),
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
            (S::MainMenu, E::Continue { mission }) => S::Briefing {
                mission: mission.clamp(1, MISSION_COUNT),
            },
            (S::MainMenu, E::OpenSettings) => S::Settings,
            (S::MainMenu, E::OpenHelp) => S::Help,
            (S::Ending { chosen: 0 }, E::EndingChosen(c)) if c != 0 => S::Ending { chosen: c },
            (S::Ending { chosen }, E::DebriefDone) => {
                if chosen == 0 {
                    self
                } else {
                    S::MainMenu
                }
            }
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
                    S::Ending { chosen: 0 }
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
    Help,
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
    /// The main menu; Continue is enabled when a save game exists.
    pub fn main(has_save: bool) -> Menu {
        let item = |label, action, enabled| MenuItem {
            label,
            action,
            enabled,
        };
        Menu {
            items: vec![
                item("New Game", MenuAction::NewGame, true),
                item("Continue", MenuAction::Continue, has_save),
                item("Settings", MenuAction::Settings, true),
                item("Help", MenuAction::Help, true),
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
        MenuAction::Settings => Some(AppEvent::OpenSettings),
        MenuAction::Help => Some(AppEvent::OpenHelp),
        // Continue needs the saved mission number, so the caller builds that event.
        MenuAction::Continue => None,
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
        assert_eq!(end, AppState::Ending { chosen: 0 });
        assert_eq!(end.next(AppEvent::DebriefDone), end, "must choose first");
        let chosen = end.next(AppEvent::EndingChosen(2));
        assert_eq!(chosen, AppState::Ending { chosen: 2 });
        assert_eq!(chosen.next(AppEvent::EndingChosen(1)), chosen);
        assert_eq!(chosen.next(AppEvent::DebriefDone), AppState::MainMenu);
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
        let mut m = Menu::main(false);
        assert_eq!(m.activate(), Some(MenuAction::NewGame));
        m.step(1);
        assert_eq!(m.activate(), Some(MenuAction::Settings));
        m.step(-1);
        m.step(-1);
        assert_eq!(m.activate(), Some(MenuAction::Quit));
        let mut with_save = Menu::main(true);
        with_save.step(1);
        assert_eq!(with_save.activate(), Some(MenuAction::Continue));
    }

    #[test]
    fn menu_select_rejects_disabled_and_out_of_range() {
        let mut m = Menu::main(false);
        assert!(!m.select(1));
        assert!(!m.select(99));
        assert!(m.select(4));
        assert_eq!(m.selected(), 4);
        assert_eq!(
            menu_event(MenuAction::Settings),
            Some(AppEvent::OpenSettings)
        );
        assert_eq!(menu_event(MenuAction::Help), Some(AppEvent::OpenHelp));
        assert_eq!(menu_event(MenuAction::Quit), Some(AppEvent::QuitRequested));
    }

    #[test]
    fn continue_resumes_the_saved_mission_and_screens_return_to_menu() {
        let s = AppState::MainMenu.next(AppEvent::Continue { mission: 6 });
        assert_eq!(s, AppState::Briefing { mission: 6 });
        assert_eq!(
            AppState::MainMenu.next(AppEvent::Continue { mission: 99 }),
            AppState::Briefing {
                mission: MISSION_COUNT
            }
        );
        assert_eq!(
            AppState::MainMenu.next(AppEvent::OpenSettings),
            AppState::Settings
        );
        assert_eq!(
            AppState::Settings.next(AppEvent::ToMenu),
            AppState::MainMenu
        );
        assert_eq!(AppState::MainMenu.next(AppEvent::OpenHelp), AppState::Help);
        assert_eq!(AppState::Help.next(AppEvent::ToMenu), AppState::MainMenu);
    }
}
