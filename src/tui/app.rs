use std::io;
use std::path::PathBuf;
use std::time::Duration;

use tuirealm::listener::EventListenerCfg;
use tuirealm::terminal::{CrosstermTerminalAdapter, TerminalAdapter, TerminalBridge};
use tuirealm::{Application, NoUserEvent, PollStrategy, Update};

use super::dashboard::Dashboard;
use super::types::{AppId, AppMsg};

/// Top-level TUI application: wraps the tuirealm `Application` and
/// `TerminalBridge` and drives the event-poll / render loop.
pub struct TuiApp<T>
where
    T: TerminalAdapter,
{
    application: Application<AppId, AppMsg, NoUserEvent>,
    quit: bool,
    redraw: bool,
    pub terminal: TerminalBridge<T>,
}

impl TuiApp<CrosstermTerminalAdapter> {
    /// Constructs a `TuiApp` configured for crossterm and performs the initial
    /// media scan.
    pub fn new(target: PathBuf) -> io::Result<Self> {
        let listener = EventListenerCfg::default()
            .crossterm_input_listener(Duration::from_millis(50), 10)
            .poll_timeout(Duration::from_millis(50))
            .tick_interval(Duration::from_millis(50));

        let mut application: Application<AppId, AppMsg, NoUserEvent> = Application::init(listener);
        let _ = application.mount(AppId::Dashboard, Box::new(Dashboard::new(target)), vec![]);
        let _ = application.active(&AppId::Dashboard);

        Ok(Self {
            application,
            quit: false,
            redraw: true,
            terminal: TerminalBridge::init_crossterm()
                .map_err(|error| io::Error::other(error.to_string()))?,
        })
    }
}

impl<T> TuiApp<T>
where
    T: TerminalAdapter,
{
    /// Draws the current state to the terminal.
    fn view(&mut self) -> io::Result<()> {
        self.terminal
            .draw(|frame| {
                self.application
                    .view(&AppId::Dashboard, frame, frame.area())
            })
            .map(|_| ())
            .map_err(|error| io::Error::other(error.to_string()))
    }

    /// Runs the event loop until the user quits, then restores the terminal.
    pub fn run(&mut self) -> io::Result<()> {
        self.view()?;
        self.redraw = false;

        while !self.quit {
            match self.application.tick(PollStrategy::Once) {
                Ok(messages) if !messages.is_empty() => {
                    for message in messages {
                        let mut next = Some(message);
                        while next.is_some() {
                            next = self.update(next);
                        }
                    }
                }
                Err(_) => self.quit = true,
                _ => {}
            }

            if self.redraw && !self.quit {
                self.view()?;
                self.redraw = false;
            }
        }

        self.terminal
            .restore()
            .map_err(|error| io::Error::other(error.to_string()))
    }
}

impl<T> Update<AppMsg> for TuiApp<T>
where
    T: TerminalAdapter,
{
    fn update(&mut self, msg: Option<AppMsg>) -> Option<AppMsg> {
        if let Some(message) = msg {
            self.redraw = true;
            match message {
                AppMsg::Quit => {
                    self.quit = true;
                    None
                }
                AppMsg::Redraw => None,
            }
        } else {
            None
        }
    }
}
