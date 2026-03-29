use std::collections::HashSet;
use std::path::PathBuf;

use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{AttrValue, Attribute, Props};
use tuirealm::ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent, State};

use crate::media::{clean_media, scan_media};
use crate::utils::format_bytes;

use super::types::{ACTIONS, AppMsg, UiAction};

// ── Modes ─────────────────────────────────────────────────────────────────────

/// Which pane is currently focused.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Focus {
    Contacts,
    Actions,
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

/// The single tuirealm component that makes up the entire TUI.
///
/// It owns the scan state, handles keyboard events, and draws the full layout
/// on every render cycle.
pub struct Dashboard {
    props: Props,
    target: PathBuf,
    report: Option<crate::media::ScanReport>,
    status: String,

    // Contact list state
    /// Labels of all contacts (index 0 = "All contacts" sentinel).
    contact_labels: Vec<String>,
    /// Contacts whose checkbox is ticked. Empty set means top-level cursor only.
    selected_contacts: HashSet<String>,
    /// Cursor position in the contact list (0 = "All contacts").
    contact_cursor: usize,

    // Action list state
    selected_action: usize,
    confirm_clean: bool,
    pending_clean: bool,
    pending_restart: bool,
    /// Defaults to `true` (Yes) for the restart interactive popup.
    restart_selection: bool,

    focus: Focus,
}

impl Dashboard {
    pub fn new(target: PathBuf) -> Self {
        let mut dashboard = Self {
            props: Props::default(),
            target,
            report: None,
            status: String::new(),
            contact_labels: vec!["All contacts".to_string()],
            selected_contacts: HashSet::new(),
            contact_cursor: 0,
            selected_action: 0,
            confirm_clean: false,
            pending_clean: false,
            pending_restart: false,
            restart_selection: true,
            focus: Focus::Contacts,
        };
        dashboard.refresh();
        dashboard
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Returns `true` when no contacts are explicitly checked, meaning the
    /// "All contacts" row is implicitly active.
    fn all_selected(&self) -> bool {
        self.selected_contacts.is_empty()
    }

    /// Human-readable description of the current selection.
    fn selection_summary(&self) -> String {
        if self.all_selected() {
            "All contacts selected".to_string()
        } else {
            let count = self.selected_contacts.len();
            format!(
                "{} contact{} selected",
                count,
                if count == 1 { "" } else { "s" }
            )
        }
    }

    /// Files that belong to the current contact selection, used for
    /// preview/clean operations.
    fn files_for_selection(&self) -> Vec<crate::media::MediaEntry> {
        let Some(report) = &self.report else {
            return vec![];
        };

        if self.all_selected() {
            return report.files.clone();
        }

        // Filter files to those attributed to a selected contact.
        // We use the `files` array now stored directly on each `ContactBreakdown`.
        let mut result = Vec::new();
        for cb in &report.contact_breakdown {
            if self.selected_contacts.contains(&cb.label) {
                result.extend(cb.files.iter().cloned());
            }
        }
        result
    }

    fn size_for_selection(&self) -> (usize, u64) {
        let files = self.files_for_selection();
        let size: u64 = files.iter().map(|f| f.size).sum();
        (files.len(), size)
    }

    fn rebuild_contact_labels(&mut self) {
        let mut labels = vec!["All contacts".to_string()];
        if let Some(report) = &self.report {
            labels.extend(report.contact_breakdown.iter().map(|cb| cb.label.clone()));
        }
        self.contact_labels = labels;
        // Clamp cursor.
        if self.contact_cursor >= self.contact_labels.len() {
            self.contact_cursor = self.contact_labels.len().saturating_sub(1);
        }
        // Remove stale selections.
        let valid: HashSet<String> = self.contact_labels.iter().skip(1).cloned().collect();
        self.selected_contacts.retain(|s| valid.contains(s));
    }

    // ── Actions ───────────────────────────────────────────────────────────────

    pub fn refresh(&mut self) {
        self.confirm_clean = false;
        match scan_media(&self.target) {
            Ok(report) => {
                self.status = if report.total_files == 0 {
                    "No WhatsApp media files found.".to_string()
                } else {
                    format!(
                        "Scanned {} file(s) totalling {}.",
                        report.total_files,
                        format_bytes(report.total_size)
                    )
                };
                self.report = Some(report);
            }
            Err(error) => {
                self.report = None;
                self.status = format!("Scan failed: {}", error);
            }
        }
        self.rebuild_contact_labels();
    }

    pub fn preview(&mut self) {
        self.confirm_clean = false;
        self.pending_restart = false;
        let (count, size) = self.size_for_selection();
        if count == 0 {
            self.status = "Nothing to delete.".to_string();
        } else {
            self.status = format!(
                "Preview: {} file(s) would be deleted, freeing {}. ({})",
                count,
                format_bytes(size),
                self.selection_summary()
            );
        }
    }

    pub fn run_selected_action(&mut self) {
        match ACTIONS[self.selected_action] {
            UiAction::Rescan => self.refresh(),
            UiAction::PreviewClean => self.preview(),
            UiAction::Clean => {
                let files = self.files_for_selection();
                if files.is_empty() {
                    self.status = "Nothing to delete.".to_string();
                    self.confirm_clean = false;
                    return;
                }
                if !self.confirm_clean {
                    self.confirm_clean = true;
                    self.status = format!(
                        "Press Enter again to delete {} file(s) and free {}. ({})",
                        files.len(),
                        format_bytes(files.iter().map(|f| f.size).sum()),
                        self.selection_summary()
                    );
                    return;
                }

                let outcome = clean_media(&self.target, &files);
                self.confirm_clean = false;
                self.status = format!(
                    "Deleted {}/{} file(s), freed {}{}{}",
                    outcome.deleted_files,
                    outcome.total_files,
                    format_bytes(outcome.freed_bytes),
                    if outcome.repaired_orphans > 0 {
                        format!(", repaired {} orphaned records", outcome.repaired_orphans)
                    } else {
                        String::new()
                    },
                    if outcome.errors > 0 {
                        format!(", {} delete errors", outcome.errors)
                    } else {
                        String::new()
                    }
                );
                self.refresh();
                if outcome.db_updated {
                    self.pending_restart = true;
                    self.restart_selection = true;
                } else {
                    self.status.push_str(
                        " — Database update skipped. Close WhatsApp if references look stale.",
                    );
                }
            }
        }
    }

    pub fn move_contact_cursor(&mut self, delta: isize) {
        let last = self.contact_labels.len() as isize - 1;
        self.contact_cursor = (self.contact_cursor as isize + delta).clamp(0, last) as usize;
    }

    pub fn toggle_contact_selection(&mut self) {
        if self.contact_cursor == 0 {
            // "All contacts" row — clear individual selections (= select all).
            self.selected_contacts.clear();
            return;
        }
        let label = self.contact_labels[self.contact_cursor].clone();
        if self.selected_contacts.contains(&label) {
            self.selected_contacts.remove(&label);
        } else {
            self.selected_contacts.insert(label);
        }
    }

    pub fn move_action_cursor(&mut self, delta: isize) {
        let last = ACTIONS.len() as isize - 1;
        let next = (self.selected_action as isize + delta).clamp(0, last);
        self.selected_action = next as usize;
        self.confirm_clean = false;
    }

    // ── Draw helpers ──────────────────────────────────────────────────────────

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let (sel_count, sel_size) = self.size_for_selection();
        let sel_info = if sel_count > 0 {
            format!("{}  {}", sel_count, format_bytes(sel_size))
        } else {
            "—".to_string()
        };

        let text = vec![
            Line::from(vec![
                Span::styled(
                    "wmc",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  WhatsApp Media Cleaner"),
            ]),
            Line::from(vec![
                Span::styled("Selection: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}  ·  {}", self.selection_summary(), sel_info)),
            ]),
            Line::from(vec![
                Span::styled("Keys: ", Style::default().fg(Color::DarkGray)),
                Span::styled("↑↓", Style::default().fg(Color::Yellow)),
                Span::raw(" navigate  "),
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::raw(" select  "),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::raw(" switch panel  "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" run  "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" quit"),
            ]),
        ];
        let header = Paragraph::new(text)
            .block(Block::default().borders(Borders::BOTTOM))
            .wrap(Wrap { trim: true });
        frame.render_widget(header, area);
    }

    fn draw_contacts(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus == Focus::Contacts {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let items: Vec<ListItem> = self
            .contact_labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                if i == 0 {
                    // "All contacts" row
                    let checked = self.selected_contacts.is_empty();
                    let marker = if checked { "●" } else { "○" };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{} ", marker),
                            Style::default().fg(if checked {
                                Color::Green
                            } else {
                                Color::DarkGray
                            }),
                        ),
                        Span::styled(
                            label.as_str(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]))
                } else {
                    let checked = self.selected_contacts.contains(label.as_str());
                    let (marker, marker_color) = if checked {
                        ("[✓]", Color::Green)
                    } else {
                        ("[ ]", Color::DarkGray)
                    };

                    // Look up size info for this contact.
                    let size_str = self
                        .report
                        .as_ref()
                        .and_then(|r| {
                            r.contact_breakdown
                                .iter()
                                .find(|cb| cb.label == *label)
                                .map(|cb| {
                                    format!("  {} / {}", cb.file_count, format_bytes(cb.total_size))
                                })
                        })
                        .unwrap_or_default();

                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{} ", marker), Style::default().fg(marker_color)),
                        Span::raw(label.as_str()),
                        Span::styled(size_str, Style::default().fg(Color::DarkGray)),
                    ]))
                }
            })
            .collect();

        let mut state = ListState::default().with_selected(Some(self.contact_cursor));
        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Contacts ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn draw_actions(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focus == Focus::Actions {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let items: Vec<ListItem> = ACTIONS
            .iter()
            .enumerate()
            .map(|(index, action)| {
                let is_destructive = *action == UiAction::Clean;
                let is_selected = index == self.selected_action;
                let confirm_pending = is_destructive && self.confirm_clean && is_selected;

                let label = if confirm_pending {
                    format!("{} ← press Enter again to confirm", action.label())
                } else {
                    format!("{}  [{}]", action.label(), action.shortcut())
                };

                let style = if is_destructive && self.focus == Focus::Actions {
                    Style::default().fg(Color::LightRed)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let mut state = ListState::default().with_selected(if self.focus == Focus::Actions {
            Some(self.selected_action)
        } else {
            None
        });

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Actions ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let status = Paragraph::new(self.status.clone())
            .block(Block::default().borders(Borders::TOP))
            .wrap(Wrap { trim: true });
        frame.render_widget(status, area);
    }

    fn draw_deleting_popup(&self, frame: &mut Frame, area: Rect) {
        let popup_area = {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(5),
                    Constraint::Percentage(40),
                ])
                .split(area);
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                ])
                .split(vert[1])[1]
        };

        let (count, _) = self.size_for_selection();
        let text = format!("Deleting {} file(s)...\n\nPlease wait.", count);

        let popup = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Cleaning ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(Clear, popup_area);
        frame.render_widget(popup, popup_area);
    }

    fn draw_restart_popup(&self, frame: &mut Frame, area: Rect) {
        let popup_area = {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(35),
                    Constraint::Length(7),
                    Constraint::Percentage(35),
                ])
                .split(area);
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(vert[1])[1]
        };

        let active_style = Style::default().fg(Color::Black).bg(Color::Cyan);
        let inactive_style = Style::default().fg(Color::DarkGray);

        let yes_style = if self.restart_selection {
            active_style
        } else {
            inactive_style
        };
        let no_style = if !self.restart_selection {
            active_style
        } else {
            inactive_style
        };

        let text = vec![
            Line::from("Database updated successfully."),
            Line::from("Restart WhatsApp now to reflect changes?"),
            Line::from(""),
            Line::from(vec![
                Span::raw("   "),
                Span::styled("  Yes  ", yes_style),
                Span::raw("      "),
                Span::styled("  No   ", no_style),
                Span::raw("   "),
            ]),
        ];

        let popup = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Restart WhatsApp ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(Clear, popup_area);
        frame.render_widget(popup, popup_area);
    }

    fn draw_error_popup(&self, frame: &mut Frame, area: Rect) {
        let popup_area = {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(5),
                    Constraint::Percentage(40),
                ])
                .split(area);
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(15),
                    Constraint::Percentage(70),
                    Constraint::Percentage(15),
                ])
                .split(vert[1])[1]
        };

        let popup = Paragraph::new("Unable to scan WhatsApp media. Make sure WhatsApp is installed and the path is correct.")
            .block(
                Block::default()
                    .title(" Scan Error ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(Clear, popup_area);
        frame.render_widget(popup, popup_area);
    }
}

// ── tuirealm trait implementations ───────────────────────────────────────────

impl MockComponent for Dashboard {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Outer layout: header / body / status
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // header
                Constraint::Min(8),    // body
                Constraint::Length(3), // status bar
            ])
            .split(area);

        self.draw_header(frame, outer[0]);

        // Body: contacts on left, actions on right
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(outer[1]);

        self.draw_contacts(frame, body[0]);
        self.draw_actions(frame, body[1]);

        self.draw_status(frame, outer[2]);

        if self.report.is_none() {
            self.draw_error_popup(frame, area);
        } else if self.pending_clean {
            self.draw_deleting_popup(frame, area);
        } else if self.pending_restart {
            self.draw_restart_popup(frame, area);
        }
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<AppMsg, NoUserEvent> for Dashboard {
    fn on(&mut self, event: Event<NoUserEvent>) -> Option<AppMsg> {
        if self.pending_restart {
            return match event {
                // Confirm selection
                Event::Keyboard(KeyEvent {
                    code: Key::Enter, ..
                }) => {
                    self.pending_restart = false;
                    if self.restart_selection {
                        crate::media::cleaner::restart_whatsapp();
                        self.status = "WhatsApp restarted.".to_string();
                    } else {
                        self.status = "WhatsApp restart skipped.".to_string();
                    }
                    Some(AppMsg::Redraw)
                }
                // Toggle selection
                Event::Keyboard(KeyEvent {
                    code: Key::Left, ..
                })
                | Event::Keyboard(KeyEvent { code: Key::Up, .. })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('k'),
                    ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('h'),
                    ..
                }) => {
                    self.restart_selection = true;
                    Some(AppMsg::Redraw)
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Right, ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Down, ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('j'),
                    ..
                })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('l'),
                    ..
                }) => {
                    self.restart_selection = false;
                    Some(AppMsg::Redraw)
                }
                // Cancel
                Event::Keyboard(KeyEvent { code: Key::Esc, .. })
                | Event::Keyboard(KeyEvent {
                    code: Key::Char('q'),
                    ..
                }) => {
                    self.pending_restart = false;
                    self.status = "WhatsApp restart skipped.".to_string();
                    Some(AppMsg::Redraw)
                }
                Event::WindowResize(_, _) => Some(AppMsg::Redraw),
                _ => None,
            };
        }

        match event {
            // Quit
            Event::Keyboard(KeyEvent { code: Key::Esc, .. })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            }) => Some(AppMsg::Quit),

            // Tab — toggle focus between Contacts and Actions panes
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => {
                self.focus = match self.focus {
                    Focus::Contacts => Focus::Actions,
                    Focus::Actions => Focus::Contacts,
                };
                self.confirm_clean = false;
                Some(AppMsg::Redraw)
            }

            // Up / k
            Event::Keyboard(KeyEvent { code: Key::Up, .. })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                ..
            }) => {
                match self.focus {
                    Focus::Contacts => self.move_contact_cursor(-1),
                    Focus::Actions => self.move_action_cursor(-1),
                }
                Some(AppMsg::Redraw)
            }

            // Down / j
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                ..
            }) => {
                match self.focus {
                    Focus::Contacts => self.move_contact_cursor(1),
                    Focus::Actions => self.move_action_cursor(1),
                }
                Some(AppMsg::Redraw)
            }

            // Space — toggle contact selection (only in Contacts pane)
            Event::Keyboard(KeyEvent {
                code: Key::Char(' '),
                modifiers,
            }) if modifiers == KeyModifiers::NONE => {
                if self.focus == Focus::Contacts {
                    self.toggle_contact_selection();
                    Some(AppMsg::Redraw)
                } else {
                    None
                }
            }

            // r — rescan
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers,
            }) if modifiers == KeyModifiers::NONE => {
                self.refresh();
                Some(AppMsg::Redraw)
            }

            // p — preview
            Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                modifiers,
            }) if modifiers == KeyModifiers::NONE => {
                self.preview();
                Some(AppMsg::Redraw)
            }

            // Enter — run selected action (Actions pane); or switch to Actions
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if self.focus == Focus::Contacts {
                    // Auto-delete the selected contacts
                    let (count, _) = self.size_for_selection();
                    if count > 0 {
                        self.status = format!("Deleting {} file(s)... Please wait.", count);
                        self.pending_clean = true;
                    } else {
                        self.status = "Nothing to delete.".to_string();
                    }
                } else {
                    self.run_selected_action();
                }
                Some(AppMsg::Redraw)
            }

            // Tick — process background state changes like pending clean
            Event::Tick => {
                if self.pending_clean {
                    self.pending_clean = false;
                    self.selected_action = 2; // UiAction::Clean
                    self.confirm_clean = true; // Auto-confirm
                    self.run_selected_action();
                    Some(AppMsg::Redraw)
                } else {
                    None
                }
            }

            Event::WindowResize(_, _) => Some(AppMsg::Redraw),
            _ => None,
        }
    }
}
