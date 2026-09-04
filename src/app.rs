use crate::config::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Pick,
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKey {
    Digit(u8),
    Left,
    Right,
    Enter,
    Escape,
}

/// What the platform layer should do after a state change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    HideAll,
    ShowPick { candidate: String },
    ShowLocked { cinema: String },
    RefreshPick { candidate: String },
}

#[derive(Debug)]
pub struct Logic {
    pub mode: Mode,
    pub style: Style,
    pub candidate: Option<String>,
    pub cinema: Option<String>,
    /// After keyboard navigation, ignore hover until the platform reports a real mouse move.
    pub follow_pointer: bool,
}

impl Logic {
    pub fn new(style: Style) -> Self {
        Self {
            mode: Mode::Idle,
            style,
            candidate: None,
            cinema: None,
            follow_pointer: true,
        }
    }

    pub fn on_toggle(&mut self, cursor_display: Option<String>) -> Action {
        match self.mode {
            Mode::Idle => self.enter_pick(cursor_display),
            Mode::Pick | Mode::Locked => self.dismiss(),
        }
    }

    pub fn enter_pick(&mut self, cursor_display: Option<String>) -> Action {
        let Some(candidate) = cursor_display else {
            return Action::None;
        };
        self.mode = Mode::Pick;
        self.cinema = None;
        self.follow_pointer = true;
        self.candidate = Some(candidate.clone());
        Action::ShowPick { candidate }
    }

    pub fn dismiss(&mut self) -> Action {
        self.mode = Mode::Idle;
        self.candidate = None;
        self.cinema = None;
        self.follow_pointer = true;
        Action::HideAll
    }

    #[allow(dead_code)]
    pub fn allow_pointer(&mut self) {
        self.follow_pointer = true;
    }

    pub fn on_pointer(&mut self, display: &str) -> Action {
        if self.mode != Mode::Pick || !self.follow_pointer {
            return Action::None;
        }
        if self.candidate.as_deref() == Some(display) {
            return Action::None;
        }
        self.candidate = Some(display.to_string());
        Action::RefreshPick {
            candidate: display.to_string(),
        }
    }

    pub fn on_click(&mut self, display: &str) -> Action {
        match self.mode {
            Mode::Idle => Action::None,
            Mode::Pick => self.lock(display),
            Mode::Locked => self.dismiss(),
        }
    }

    pub fn lock(&mut self, cinema: &str) -> Action {
        self.mode = Mode::Locked;
        self.cinema = Some(cinema.to_string());
        self.candidate = None;
        self.follow_pointer = true;
        Action::ShowLocked {
            cinema: cinema.to_string(),
        }
    }

    pub fn on_pick_key(&mut self, key: PickKey, displays: &[String]) -> Action {
        if self.mode != Mode::Pick {
            return Action::None;
        }
        match key {
            PickKey::Escape => self.dismiss(),
            PickKey::Enter => {
                let cinema = self.candidate.clone().or_else(|| displays.first().cloned());
                match cinema {
                    Some(id) => self.lock(&id),
                    None => Action::None,
                }
            }
            PickKey::Digit(n) => {
                let idx = n.saturating_sub(1) as usize;
                match displays.get(idx) {
                    Some(id) => {
                        self.follow_pointer = false;
                        self.candidate = Some(id.clone());
                        Action::RefreshPick {
                            candidate: id.clone(),
                        }
                    }
                    None => Action::None,
                }
            }
            PickKey::Left | PickKey::Right => {
                if displays.is_empty() {
                    return Action::None;
                }
                let current = self
                    .candidate
                    .as_ref()
                    .and_then(|id| displays.iter().position(|d| d == id))
                    .unwrap_or(0);
                let next = if key == PickKey::Right {
                    (current + 1) % displays.len()
                } else {
                    (current + displays.len() - 1) % displays.len()
                };
                let id = displays[next].clone();
                self.follow_pointer = false;
                self.candidate = Some(id.clone());
                Action::RefreshPick { candidate: id }
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_style(&mut self, style: Style) -> Action {
        if self.style == style {
            return Action::None;
        }
        self.style = style;
        match self.mode {
            Mode::Idle => Action::None,
            Mode::Pick => self
                .candidate
                .clone()
                .map(|candidate| Action::RefreshPick { candidate })
                .unwrap_or(Action::None),
            Mode::Locked => self
                .cinema
                .clone()
                .map(|cinema| Action::ShowLocked { cinema })
                .unwrap_or(Action::None),
        }
    }

    #[allow(dead_code)]
    pub fn restore_after_hotplug(&mut self, displays: &[String]) -> Action {
        match self.mode {
            Mode::Idle => Action::HideAll,
            Mode::Pick => {
                if let Some(id) = &self.candidate {
                    if displays.iter().any(|d| d == id) {
                        return Action::ShowPick {
                            candidate: id.clone(),
                        };
                    }
                }
                self.enter_pick(displays.first().cloned())
            }
            Mode::Locked => {
                if let Some(id) = &self.cinema {
                    if displays.iter().any(|d| d == id) {
                        return Action::ShowLocked { cinema: id.clone() };
                    }
                }
                self.dismiss()
            }
        }
    }
}
