use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;

pub(crate) const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "wav", "aac", "opus", "m4a"];

pub(crate) fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum EntryKind {
    Dir,
    Audio,
    Other,
}

pub(crate) struct BrowserEntry {
    pub(crate) name: String,
    pub(crate) path: std::path::PathBuf,
    pub(crate) kind: EntryKind,
}

pub(crate) struct BrowserState {
    pub(crate) cwd: std::path::PathBuf,
    pub(crate) entries: Vec<BrowserEntry>,
    pub(crate) cursor: usize,
}

impl BrowserState {
    pub(crate) fn new(dir: std::path::PathBuf) -> io::Result<Self> {
        let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
        let mut entries = Vec::new();

        if dir.parent().is_some() {
            entries.push(BrowserEntry {
                name: "..".to_string(),
                path: dir.parent().unwrap().to_path_buf(),
                kind: EntryKind::Dir,
            });
        }

        let mut raw: Vec<_> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
        raw.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());

        let mut dirs  = Vec::new();
        let mut audio = Vec::new();
        let mut other = Vec::new();
        for entry in raw {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                dirs.push(BrowserEntry { name, path, kind: EntryKind::Dir });
            } else if is_audio(&path) {
                audio.push(BrowserEntry { name, path, kind: EntryKind::Audio });
            } else {
                other.push(BrowserEntry { name, path, kind: EntryKind::Other });
            }
        }
        entries.extend(dirs);
        entries.extend(audio);
        entries.extend(other);

        // Start on the first selectable entry that isn't `..` so Enter navigates
        // into content rather than immediately going back up. `..` is reachable via Up.
        let cursor = entries
            .iter()
            .position(|e| Self::is_selectable(&e.kind) && e.name != "..")
            .unwrap_or(0);

        Ok(Self { cwd: dir, entries, cursor })
    }

    pub(crate) fn is_selectable(kind: &EntryKind) -> bool {
        matches!(kind, EntryKind::Dir | EntryKind::Audio)
    }

    pub(crate) fn move_down(&mut self) {
        let next = (self.cursor + 1..self.entries.len())
            .find(|&i| Self::is_selectable(&self.entries[i].kind));
        if let Some(i) = next {
            self.cursor = i;
        }
    }

    pub(crate) fn move_up(&mut self) {
        let prev = (0..self.cursor)
            .rev()
            .find(|&i| Self::is_selectable(&self.entries[i].kind));
        if let Some(i) = prev {
            self.cursor = i;
        }
    }
}

pub(crate) enum BrowserResult {
    Selected(std::path::PathBuf),
    ReturnToPlayer,
    Quit,
}

pub(crate) fn run_browser(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_dir: std::path::PathBuf,
) -> io::Result<(BrowserResult, std::path::PathBuf)> {
    let mut state = BrowserState::new(start_dir)?;

    loop {
        let cursor = state.cursor;
        let cwd_display = state.cwd.display().to_string();

        let items: Vec<ListItem> = state
            .entries
            .iter()
            .map(|e| {
                let (label, color) = match e.kind {
                    EntryKind::Dir => {
                        let label = format!("{}/", e.name);
                        (label, Color::Yellow)
                    }
                    EntryKind::Audio => (e.name.clone(), Color::Green),
                    EntryKind::Other => (e.name.clone(), Color::DarkGray),
                };
                ListItem::new(label).style(Style::default().fg(color))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" deck — {} ", cwd_display))
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut list_state = ListState::default().with_selected(Some(cursor));

        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(area);

            frame.render_stateful_widget(list, chunks[0], &mut list_state);

            frame.render_widget(
                Paragraph::new("↑/↓: navigate   Enter: open   ←/Bksp: up dir   Esc: back to player   q: quit")
                    .style(Style::default().fg(Color::DarkGray)),
                chunks[1],
            );
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok((BrowserResult::Quit, state.cwd));
                }
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Up => state.move_up(),
                    KeyCode::Down => state.move_down(),
                    KeyCode::Enter => {
                        if let Some(entry) = state.entries.get(state.cursor) {
                            match entry.kind {
                                EntryKind::Dir => {
                                    let path = entry.path.clone();
                                    state = BrowserState::new(path)?;
                                }
                                EntryKind::Audio => {
                                    let cwd = state.cwd.clone();
                                    return Ok((BrowserResult::Selected(entry.path.clone()), cwd));
                                }
                                EntryKind::Other => {}
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Left => {
                        if let Some(parent) = state.cwd.parent().map(|p| p.to_path_buf()) {
                            state = BrowserState::new(parent)?;
                        }
                    }
                    KeyCode::Char('q') => return Ok((BrowserResult::Quit, state.cwd)),
                    KeyCode::Char('z') => return Ok((BrowserResult::ReturnToPlayer, state.cwd)),
                    KeyCode::Esc => return Ok((BrowserResult::ReturnToPlayer, state.cwd)),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_state(kinds: &[EntryKind]) -> BrowserState {
        BrowserState {
            cwd: PathBuf::from("/test"),
            entries: kinds
                .iter()
                .enumerate()
                .map(|(i, k)| BrowserEntry {
                    name: format!("entry{i}"),
                    path: PathBuf::from(format!("/test/entry{i}")),
                    kind: k.clone(),
                })
                .collect(),
            cursor: 0,
        }
    }

    #[test]
    fn test_is_audio_known_extensions() {
        assert!(is_audio(&PathBuf::from("track.flac")));
        assert!(is_audio(&PathBuf::from("track.mp3")));
        assert!(is_audio(&PathBuf::from("track.ogg")));
        assert!(is_audio(&PathBuf::from("track.wav")));
    }

    #[test]
    fn test_is_audio_case_insensitive() {
        assert!(is_audio(&PathBuf::from("track.FLAC")));
        assert!(is_audio(&PathBuf::from("track.Mp3")));
    }

    #[test]
    fn test_is_audio_non_audio() {
        assert!(!is_audio(&PathBuf::from("readme.txt")));
        assert!(!is_audio(&PathBuf::from("noextension")));
        assert!(!is_audio(&PathBuf::from("image.png")));
    }

    #[test]
    fn test_dirs_and_audio_are_selectable() {
        assert!(BrowserState::is_selectable(&EntryKind::Dir));
        assert!(BrowserState::is_selectable(&EntryKind::Audio));
        assert!(!BrowserState::is_selectable(&EntryKind::Other));
    }

    #[test]
    fn test_cursor_down_skips_other() {
        // [Audio, Other, Audio] — down from 0 should land on 2
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Other, EntryKind::Audio]);
        state.cursor = 0;
        state.move_down();
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_cursor_up_skips_other() {
        // [Audio, Other, Audio] — up from 2 should land on 0
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Other, EntryKind::Audio]);
        state.cursor = 2;
        state.move_up();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_cursor_down_does_not_pass_end() {
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Audio]);
        state.cursor = 1;
        state.move_down();
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_cursor_up_does_not_pass_start() {
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Audio]);
        state.cursor = 0;
        state.move_up();
        assert_eq!(state.cursor, 0);
    }
}
