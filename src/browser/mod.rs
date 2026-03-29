use std::io;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEventKind};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

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
    pub(crate) workspace: Option<std::path::PathBuf>,
    pub(crate) search_term: String,
    pub(crate) search_results: Option<Vec<std::path::PathBuf>>,
    /// Flat list of all audio files under the workspace; populated on first search keystroke.
    workspace_files: Option<Vec<std::path::PathBuf>>,
}

impl BrowserState {
    pub(crate) fn new(dir: std::path::PathBuf, workspace: Option<std::path::PathBuf>) -> io::Result<Self> {
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

        Ok(Self { cwd: dir, entries, cursor, workspace, search_term: String::new(), search_results: None, workspace_files: None })
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

    /// Returns the path of the currently highlighted audio file, or `None` if
    /// the cursor is on a directory or non-audio entry.
    pub(crate) fn highlighted_audio_path(&self) -> Option<std::path::PathBuf> {
        if let Some(ref results) = self.search_results {
            results.get(self.cursor).cloned()
        } else {
            self.entries.get(self.cursor).and_then(|e| {
                if e.kind == EntryKind::Audio { Some(e.path.clone()) } else { None }
            })
        }
    }
}

/// Compute a human-readable relative path from `base` to `target`, using `./` and `../` notation.
fn relative_path(base: &std::path::Path, target: &std::path::Path) -> String {
    if let Ok(rel) = target.strip_prefix(base) {
        let s = rel.display().to_string();
        if s.is_empty() { "./".to_string() } else { format!("./{s}") }
    } else {
        let base_comps: Vec<_> = base.components().collect();
        let target_comps: Vec<_> = target.components().collect();
        let common = base_comps.iter().zip(target_comps.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let up = base_comps.len() - common;
        let mut parts = vec!["..".to_string(); up];
        parts.extend(target_comps[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
        parts.join("/")
    }
}

fn collect_workspace_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        let mut children: Vec<_> = read.filter_map(|e| e.ok()).collect();
        children.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
        for entry in children {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if is_audio(&path) {
                files.push(path);
            }
        }
    }
    files
}

fn run_search(term: &str, files: &[std::path::PathBuf], workspace: &std::path::Path) -> Vec<std::path::PathBuf> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &std::path::PathBuf)> = files
        .iter()
        .filter_map(|p| {
            let display = p.strip_prefix(workspace).unwrap_or(p).display().to_string();
            matcher.fuzzy_match(&display, term).map(|score| (score, p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, p)| p.clone()).collect()
}

pub(crate) enum BrowserResult {
    Selected(std::path::PathBuf),
    WorkspaceSet(std::path::PathBuf),
    WorkspaceCleared,
    ReturnToPlayer,
    Quit,
}

pub(crate) fn render_browser(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &BrowserState,
    deck_slot: usize,
) {
    let bg         = Color::Rgb(20, 20, 38);
    let border_fg  = Color::Rgb(40, 60, 100);
    let border_style = Style::default().fg(border_fg).bg(bg);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Top bar: search field (workspace set) or workspace prompt.
    // When workspace is set, a right-side hint reminds the user how to clear it.
    if state.workspace.is_some() {
        let hint = "': unset  ";
        let hint_len = hint.chars().count() as u16;
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(hint_len)])
            .split(chunks[0]);

        let search_line = if state.search_term.is_empty() {
            Line::from(vec![
                ratatui::text::Span::styled(" search: ", Style::default().fg(Color::Rgb(100, 140, 220)).bg(bg)),
                ratatui::text::Span::styled("type to search", Style::default().fg(Color::Rgb(60, 60, 80)).bg(bg)),
            ])
        } else {
            Line::from(vec![
                ratatui::text::Span::styled(" search: ", Style::default().fg(Color::Rgb(100, 140, 220)).bg(bg)),
                ratatui::text::Span::styled(state.search_term.clone(), Style::default().fg(Color::White).bg(bg)),
                ratatui::text::Span::styled("█", Style::default().fg(Color::Rgb(100, 140, 220)).bg(bg)),
            ])
        };
        frame.render_widget(Paragraph::new(search_line).style(Style::default().bg(bg)), top_cols[0]);
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::Rgb(60, 80, 120)).bg(bg)),
            top_cols[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(" Press @ to set this directory as your search workspace")
                .style(Style::default().fg(Color::Rgb(60, 80, 120)).bg(bg)),
            chunks[0],
        );
    };

    // List: search results or directory entries.
    let items: Vec<ListItem> = if let Some(ref results) = state.search_results {
        let ws = state.workspace.as_deref().unwrap();
        results.iter().map(|p| {
            let label = p.strip_prefix(ws).unwrap_or(p).display().to_string();
            ListItem::new(label).style(Style::default().fg(Color::Yellow))
        }).collect()
    } else {
        state.entries.iter().map(|e| {
            let (label, color) = match e.kind {
                EntryKind::Dir   => (format!("{}/", e.name), Color::Rgb(80, 110, 180)),
                EntryKind::Audio => (e.name.clone(), Color::Yellow),
                EntryKind::Other => (e.name.clone(), Color::Rgb(60, 60, 80)),
            };
            ListItem::new(label).style(Style::default().fg(color))
        }).collect()
    };

    // Title: workspace root (bright) + relative path from workspace to cwd (dim).
    // Falls back to full cwd when no workspace is set.
    let path_title = if let Some(ws) = &state.workspace {
        let rel_str = relative_path(ws, &state.cwd);
        Line::from(vec![
            ratatui::text::Span::styled(
                format!(" @: {} ", ws.display()),
                Style::default().fg(Color::Yellow).bg(bg),
            ),
            ratatui::text::Span::styled(
                format!("[{}] ", rel_str),
                Style::default().fg(Color::Rgb(80, 80, 60)).bg(bg),
            ),
        ])
    } else {
        Line::from(ratatui::text::Span::styled(
            format!(" {} ", state.cwd.display()),
            Style::default().fg(Color::Yellow).bg(bg),
        ))
    };

    let result_count_title = state.search_results.as_ref().map(|r| {
        Line::from(ratatui::text::Span::styled(
            format!(" {} results ", r.len()),
            Style::default().fg(Color::Rgb(100, 140, 220)).bg(bg),
        )).alignment(Alignment::Left)
    });

    let mut block = Block::default()
        .title(path_title.alignment(Alignment::Left))
        .title(Line::from(ratatui::text::Span::styled(format!(" deck {} ", deck_slot + 1), Style::default().fg(Color::Yellow).bg(bg))).alignment(Alignment::Right))
        .border_style(border_style)
        .style(Style::default().bg(bg))
        .borders(Borders::ALL);
    if let Some(t) = result_count_title {
        // Second left title sits below the path title — used during search to show match count.
        block = block.title_bottom(t);
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 50, 0)).add_modifier(Modifier::BOLD));

    let mut list_state = ListState::default().with_selected(Some(state.cursor));
    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    let hint = if state.search_results.is_some() {
        "↑/↓  Enter: load   Bksp: edit search   Esc: clear search   #: preview   q: quit"
    } else {
        "↑/↓  Enter  ←/Bksp: up   Esc: back   #: preview   q: quit"
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::Rgb(40, 60, 100)).bg(bg)),
        chunks[2],
    );
}

pub(crate) fn handle_browser_key(
    state: &mut BrowserState,
    key: crossterm::event::KeyEvent,
) -> io::Result<Option<BrowserResult>> {
    if key.kind != KeyEventKind::Press { return Ok(None); }
    match key.code {
        KeyCode::Up => {
            if state.search_results.is_some() {
                if state.cursor > 0 { state.cursor -= 1; }
            } else {
                state.move_up();
            }
            Ok(None)
        }
        KeyCode::Down => {
            if let Some(ref results) = state.search_results {
                if state.cursor + 1 < results.len() { state.cursor += 1; }
            } else {
                state.move_down();
            }
            Ok(None)
        }
        KeyCode::Enter => {
            if let Some(ref results) = state.search_results {
                return Ok(results.get(state.cursor).cloned().map(BrowserResult::Selected));
            }
            if let Some(entry) = state.entries.get(state.cursor) {
                match entry.kind {
                    EntryKind::Dir => {
                        let path = entry.path.clone();
                        let workspace = state.workspace.clone();
                        *state = BrowserState::new(path, workspace)?;
                        Ok(None)
                    }
                    EntryKind::Audio => Ok(Some(BrowserResult::Selected(entry.path.clone()))),
                    EntryKind::Other => Ok(None),
                }
            } else {
                Ok(None)
            }
        }
        KeyCode::Backspace | KeyCode::Left => {
            if state.workspace.is_some() && !state.search_term.is_empty() {
                state.search_term.pop();
                if state.search_term.is_empty() {
                    state.search_results = None;
                } else {
                    let ws = state.workspace.as_deref().unwrap();
                    let files = state.workspace_files.get_or_insert_with(|| collect_workspace_files(ws));
                    state.search_results = Some(run_search(&state.search_term, files, ws));
                }
                state.cursor = 0;
            } else if let Some(parent) = state.cwd.parent().map(|p| p.to_path_buf()) {
                let workspace = state.workspace.clone();
                *state = BrowserState::new(parent, workspace)?;
            }
            Ok(None)
        }
        KeyCode::Char('@') => {
            state.workspace = Some(state.cwd.clone());
            state.workspace_files = None; // invalidate cached file list for new workspace
            Ok(Some(BrowserResult::WorkspaceSet(state.cwd.clone())))
        }
        KeyCode::Char('\'') => {
            state.workspace = None;
            state.workspace_files = None;
            state.search_term.clear();
            state.search_results = None;
            state.cursor = 0;
            Ok(Some(BrowserResult::WorkspaceCleared))
        }
        KeyCode::Char('q') if state.search_term.is_empty() => Ok(Some(BrowserResult::Quit)),
        KeyCode::Char('z') | KeyCode::Esc => {
            if !state.search_term.is_empty() {
                state.search_term.clear();
                state.search_results = None;
                state.cursor = 0;
                Ok(None)
            } else {
                Ok(Some(BrowserResult::ReturnToPlayer))
            }
        }
        KeyCode::Char(c) if state.workspace.is_some() && c != '@' && c != '\'' => {
            state.search_term.push(c);
            let ws = state.workspace.as_deref().unwrap();
            let files = state.workspace_files.get_or_insert_with(|| collect_workspace_files(ws));
            state.search_results = Some(run_search(&state.search_term, files, ws));
            state.cursor = 0;
            Ok(None)
        }
        _ => Ok(None),
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
            workspace: None,
            search_term: String::new(),
            search_results: None,
            workspace_files: None,
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
