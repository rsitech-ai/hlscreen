#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkstationView {
    Overview,
    Flow,
    Quality,
    Metadata,
    Explain,
}

impl WorkstationView {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Flow,
        Self::Quality,
        Self::Metadata,
        Self::Explain,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Flow => "flow",
            Self::Quality => "quality",
            Self::Metadata => "metadata",
            Self::Explain => "explain",
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkstationDensity {
    Compact,
    Balanced,
    Dense,
}

impl WorkstationDensity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Balanced => "balanced",
            Self::Dense => "dense",
        }
    }

    pub fn visible_rows(self) -> usize {
        match self {
            Self::Compact => 8,
            Self::Balanced => 15,
            Self::Dense => 30,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Compact => Self::Balanced,
            Self::Balanced => Self::Dense,
            Self::Dense => Self::Compact,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkstationAction {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    NextView,
    PreviousView,
    ToggleDensity,
    ToggleHelp,
    TogglePause,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkstationUiState {
    selected: usize,
    view: WorkstationView,
    density: WorkstationDensity,
    help_open: bool,
    paused: bool,
    quit_requested: bool,
}

impl Default for WorkstationUiState {
    fn default() -> Self {
        Self {
            selected: 0,
            view: WorkstationView::Overview,
            density: WorkstationDensity::Balanced,
            help_open: false,
            paused: false,
            quit_requested: false,
        }
    }
}

impl WorkstationUiState {
    pub fn selected_index(&self, row_count: usize) -> Option<usize> {
        if row_count == 0 {
            None
        } else {
            Some(self.selected.min(row_count - 1))
        }
    }

    pub fn view(&self) -> WorkstationView {
        self.view
    }

    pub fn density(&self) -> WorkstationDensity {
        self.density
    }

    pub fn help_open(&self) -> bool {
        self.help_open
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    pub fn visible_row_limit(&self) -> usize {
        self.density.visible_rows()
    }

    pub fn apply(&mut self, action: WorkstationAction, row_count: usize) {
        match action {
            WorkstationAction::Up => self.selected = self.selected.saturating_sub(1),
            WorkstationAction::Down => {
                if row_count > 0 {
                    self.selected = (self.selected + 1).min(row_count - 1);
                }
            }
            WorkstationAction::PageUp => self.selected = self.selected.saturating_sub(5),
            WorkstationAction::PageDown => {
                if row_count > 0 {
                    self.selected = (self.selected + 5).min(row_count - 1);
                }
            }
            WorkstationAction::Home => self.selected = 0,
            WorkstationAction::End => {
                if row_count > 0 {
                    self.selected = row_count - 1;
                }
            }
            WorkstationAction::NextView => self.view = self.view.next(),
            WorkstationAction::PreviousView => self.view = self.view.previous(),
            WorkstationAction::ToggleDensity => self.density = self.density.next(),
            WorkstationAction::ToggleHelp => self.help_open = !self.help_open,
            WorkstationAction::TogglePause => self.paused = !self.paused,
            WorkstationAction::Quit => self.quit_requested = true,
        }

        if row_count == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(row_count - 1);
        }
    }
}
