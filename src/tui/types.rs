/// The three actions available in the TUI action list.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum UiAction {
    Rescan,
    PreviewClean,
    Clean,
}

impl UiAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rescan => "Rescan media",
            Self::PreviewClean => "Preview clean",
            Self::Clean => "Delete all media",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Self::Rescan => "r",
            Self::PreviewClean => "p",
            Self::Clean => "enter",
        }
    }
}

/// Ordered list of all available actions (drives the action list widget).
pub const ACTIONS: [UiAction; 3] = [UiAction::Rescan, UiAction::PreviewClean, UiAction::Clean];

/// Component IDs used by the tuirealm `Application`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum AppId {
    Dashboard,
}

/// Messages passed through the tuirealm message bus.
#[derive(Clone, Eq, PartialEq)]
pub enum AppMsg {
    Redraw,
    Quit,
}
