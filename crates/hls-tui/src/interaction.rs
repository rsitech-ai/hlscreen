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
pub enum WorkstationChartWindow {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    SixtyMinutes,
}

impl WorkstationChartWindow {
    pub const ALL: [Self; 5] = [
        Self::OneMinute,
        Self::FiveMinutes,
        Self::FifteenMinutes,
        Self::ThirtyMinutes,
        Self::SixtyMinutes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::ThirtyMinutes => "30m",
            Self::SixtyMinutes => "60m",
        }
    }

    pub fn candle_limit(self) -> usize {
        match self {
            Self::OneMinute => 1,
            Self::FiveMinutes => 5,
            Self::FifteenMinutes => 15,
            Self::ThirtyMinutes => 30,
            Self::SixtyMinutes => 60,
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkstationCommandTarget {
    Filter,
    Preset,
    Sort,
}

impl WorkstationCommandTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Filter => "filter",
            Self::Preset => "preset",
            Self::Sort => "sort",
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Self::Filter => "where",
            Self::Preset => "preset",
            Self::Sort => "sort",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkstationCommand {
    target: WorkstationCommandTarget,
    input: String,
}

impl WorkstationCommand {
    fn new(target: WorkstationCommandTarget) -> Self {
        Self {
            target,
            input: String::new(),
        }
    }

    pub fn target(&self) -> WorkstationCommandTarget {
        self.target
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn prompt(&self) -> &'static str {
        self.target.prompt()
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
    CycleFilter,
    CyclePreset,
    CycleSort,
    CycleChartWindow,
    CommandChar(char),
    CommandBackspace,
    SubmitCommand,
    CancelCommand,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkstationUiState {
    selected: usize,
    view: WorkstationView,
    density: WorkstationDensity,
    chart_window: WorkstationChartWindow,
    command: Option<WorkstationCommand>,
    command_error: Option<String>,
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
            chart_window: WorkstationChartWindow::FifteenMinutes,
            command: None,
            command_error: None,
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

    pub fn chart_window(&self) -> WorkstationChartWindow {
        self.chart_window
    }

    pub fn command(&self) -> Option<&WorkstationCommand> {
        self.command.as_ref()
    }

    pub fn command_error(&self) -> Option<&str> {
        self.command_error.as_deref()
    }

    pub fn set_command_error(&mut self, error: String) {
        self.command_error = Some(error);
    }

    pub fn clear_command_error(&mut self) {
        self.command_error = None;
    }

    pub fn close_command(&mut self) {
        self.command = None;
        self.command_error = None;
    }

    pub fn take_command(&mut self) -> Option<WorkstationCommand> {
        self.command_error = None;
        self.command.take()
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
            WorkstationAction::CycleChartWindow => self.chart_window = self.chart_window.next(),
            WorkstationAction::CycleFilter => {
                self.command = Some(WorkstationCommand::new(WorkstationCommandTarget::Filter));
                self.command_error = None;
            }
            WorkstationAction::CyclePreset => {
                self.command = Some(WorkstationCommand::new(WorkstationCommandTarget::Preset));
                self.command_error = None;
            }
            WorkstationAction::CycleSort => {
                self.command = Some(WorkstationCommand::new(WorkstationCommandTarget::Sort));
                self.command_error = None;
            }
            WorkstationAction::CommandChar(ch) => {
                if let Some(command) = &mut self.command {
                    command.input.push(ch);
                    self.command_error = None;
                }
            }
            WorkstationAction::CommandBackspace => {
                if let Some(command) = &mut self.command {
                    command.input.pop();
                    self.command_error = None;
                }
            }
            WorkstationAction::SubmitCommand => {}
            WorkstationAction::CancelCommand => self.close_command(),
            WorkstationAction::Quit => self.quit_requested = true,
        }

        if row_count == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(row_count - 1);
        }
    }
}
