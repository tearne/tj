use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    DisableMouseCapture, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;

use rodio::stream::DeviceSinkBuilder;
use rodio::Player;

mod audio;
mod browser;
mod cache;
mod config;
mod deck;
mod render;
mod tags;

use audio::{decode_audio, scrub_audio, play_click_tone, FilterSource, PitchSource, PreviewOutput, TrackingSource, WaveformData, SeekHandle, FADE_SAMPLES};
use browser::{BrowserResult, BrowserState, handle_browser_key, render_browser};
use cache::{cache_path, hash_mono, Cache, detect_bpm};
use config::{load_config, Action, KeyBinding};
use deck::do_time_jump;
use deck::{
    anchor_beat_grid_to_cue, apply_offset_step, cache_entry_for_deck, compute_spectrum,
    compute_tap_bpm_offset, Deck, DeckAudio, NudgeMode, Notification, NotificationStyle,
    NOTIFICATION_TIMEOUT, PALETTE_SCHEMES, TagEditorState, TAG_FIELD_LABELS,
};
use render::{
    extract_tick_viewport, halfblock_art, info_line_empty, DEFAULT_ZOOM_IDX,
    info_line_for_deck, notification_line_empty, notification_line_for_deck,
    overview_empty, overview_for_deck, render_detail_empty, render_detail_waveform,
    render_shared_tick_row,
    render_tag_editor, SharedDetailRenderer, ZOOM_LEVELS,
};
use tags::{propose_rename_stem, read_cover_art, read_tags_for_editor, read_track_name};

fn cleanup_terminal() {
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(PopKeyboardEnhancementFlags).and_then(|s| s.execute(DisableMouseCapture)).and_then(|s| s.execute(LeaveAlternateScreen));
}

fn panic_log_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".config/deck/panic.log")
}

fn main() {
    color_eyre::install().expect("color_eyre initialisation should succeed at startup");

    // Chain a file-writing hook around color_eyre's hook so panics are preserved
    // even when the terminal is in raw mode and stderr is invisible.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let log_path = panic_log_path();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        let msg = format!(
            "[{timestamp}] thread '{thread_name}' {info}\n",
        );
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&log_path, &msg);
        cleanup_terminal();
        prev_hook(info);
    }));

    let args: Vec<String> = std::env::args().collect();

    let arg = args.get(1).map(std::path::PathBuf::from);
    let start = arg.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

    // Set up terminal once — shared by browser and player.
    let setup = (|| -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        io::stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?
            .execute(PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES))?;
        Terminal::new(CrosstermBackend::new(io::stdout()))
    })();
    let mut terminal = match setup {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Terminal error: {e}");
            std::process::exit(1);
        }
    };

    // Load cache early so we can read last_browser_path before the browser opens.
    let cache_file = cache_path();
    let mut cache = Cache::load(cache_file);

    // Compute the initial browser directory:
    //   CLI dir arg  → that directory (overrides last-visited for this first open only)
    //   CLI file arg → the file's parent directory
    //   no arg       → last visited path from cache (if it still exists), else CWD
    let mut browser_dir: std::path::PathBuf = if arg.as_deref().map(|p| p.is_dir()).unwrap_or(false) {
        start.clone()
    } else if start.is_file() {
        start.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| start.clone())
    } else {
        cache.last_browser_path()
            .filter(|p| p.exists())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
    };

    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            cleanup_terminal();
            eprintln!("Audio output error: {e}");
            std::process::exit(1);
        }
    };
    let mixer = handle.mixer();

    let initial_load: Option<PendingLoad> = if start.is_file() {
        Some(start_load(&start))
    } else {
        None
    };
    if let Err(e) = tui_loop(&mut terminal, initial_load, &mut cache, &mut browser_dir, &mixer) {
        cleanup_terminal();
        eprintln!("TUI error: {e}");
        std::process::exit(1);
    }

    cleanup_terminal();
}

struct PendingLoad {
    filename: String,
    path:     PathBuf,
    rx:       mpsc::Receiver<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>,
    decoded:  Arc<AtomicUsize>,
    total:    Arc<AtomicUsize>,
}

fn start_load(path: &Path) -> PendingLoad {
    let path_str = path.to_string_lossy().to_string();
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path_str)
        .to_string();
    let decoded = Arc::new(AtomicUsize::new(0));
    let total   = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<Result<(Vec<f32>, Vec<f32>, u32, u16), String>>();
    {
        let decoded_for_thread = Arc::clone(&decoded);
        let total_for_thread   = Arc::clone(&total);
        thread::spawn(move || {
            let _ = tx.send(decode_audio(&path_str, decoded_for_thread, total_for_thread).map_err(|e| e.to_string()));
        });
    }
    PendingLoad { filename, path: path.to_path_buf(), rx, decoded, total }
}

fn build_deck(
    path:            &Path,
    filename:        String,
    mono:            Vec<f32>,
    stereo:          Vec<f32>,
    sample_rate:     u32,
    channels:        u16,
    mixer:           &rodio::mixer::Mixer,
    cache:           &Cache,
    pfl_active_deck: Arc<AtomicUsize>,
    deck_slot:       usize,
) -> Deck {
    use std::sync::atomic::{AtomicBool, AtomicI8, AtomicI32, AtomicI64, AtomicU8, AtomicU32};
    let track_name  = read_track_name(&path.to_string_lossy());
    let rename_hint = {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem.is_empty() {
            None
        } else {
            let proposed = propose_rename_stem(path);
            if proposed == stem { None } else { Some(proposed) }
        }
    };
    let total_duration = mono.len() as f64 / sample_rate as f64;
    let mono           = Arc::new(mono);
    let waveform       = Arc::new(WaveformData::compute(Arc::clone(&mono), sample_rate));

    let samples        = Arc::new(stereo);
    let position       = Arc::new(AtomicUsize::new(0));
    let fade_remaining = Arc::new(AtomicI64::new(0));
    let fade_len       = Arc::new(AtomicI64::new(FADE_SAMPLES));
    let pending_target  = Arc::new(AtomicUsize::new(usize::MAX));
    let flush_pitch     = Arc::new(AtomicBool::new(false));
    let seek_handle = SeekHandle {
        samples: Arc::clone(&samples),
        position: Arc::clone(&position),
        fade_remaining: Arc::clone(&fade_remaining),
        fade_len: Arc::clone(&fade_len),
        pending_target: Arc::clone(&pending_target),
        sample_rate,
        channels,
        flush_pitch: Arc::clone(&flush_pitch),
    };

    let filter_offset_shared = Arc::new(AtomicI32::new(0));
    let filter_state_reset   = Arc::new(AtomicBool::new(false));
    let pfl_level            = Arc::new(AtomicU8::new(0));
    let deck_volume_atomic   = Arc::new(AtomicU32::new(1.0f32.to_bits()));
    let gain_linear          = Arc::new(AtomicU32::new(1.0f32.to_bits()));
    let filter_poles         = Arc::new(AtomicU8::new(2));
    let pitch_semitones      = Arc::new(AtomicI8::new(0));
    let player = Player::connect_new(mixer);
    player.append(PitchSource::new(
        FilterSource::new(
            TrackingSource::new(
                samples, position, fade_remaining, fade_len, pending_target, sample_rate, channels,
            ),
            Arc::clone(&filter_offset_shared),
            Arc::clone(&filter_state_reset),
            Arc::clone(&pfl_level),
            pfl_active_deck,
            deck_slot,
            Arc::clone(&deck_volume_atomic),
            Arc::clone(&gain_linear),
            Arc::clone(&filter_poles),
        ),
        Arc::clone(&pitch_semitones),
        Arc::clone(&flush_pitch),
    ));
    player.pause();

    let (bpm_tx, bpm_rx) = mpsc::channel::<(String, f32, i64, bool)>();
    {
        let mono_bg = Arc::clone(&mono);
        let entries = cache.entries_snapshot();
        thread::spawn(move || {
            let hash = hash_mono(&mono_bg);
            // is_fresh=false → applied immediately and marks bpm_established=true (confirmed).
            // is_fresh=true  → applied immediately only when bpm_established is false, leaves it false (unconfirmed).
            let (bpm, offset_ms, is_fresh) = if let Some(entry) = entries.get(&hash) {
                let snapped = (entry.offset_ms as f64 / 10.0).round() as i64 * 10;
                let period  = (60_000.0 / entry.bpm as f64 / 10.0).round() as i64 * 10;
                let snapped = snapped.rem_euclid(period);
                (entry.bpm, snapped, false)
            } else {
                // No cache entry: use 120 as a placeholder; leave bpm_established false so the UI
                // signals that the BPM has not been confirmed.
                (120.0f32, 0i64, true)
            };
            let _ = bpm_tx.send((hash, bpm, offset_ms, is_fresh));
        });
    }

    let mut deck = Deck::new(
        filename,
        path.to_path_buf(),
        track_name,
        total_duration,
        rename_hint,
        DeckAudio {
            player,
            seek_handle,
            mono,
            waveform,
            sample_rate,
            filter_offset_shared,
            filter_state_reset,
            filter_poles,
            pfl_level,
            deck_volume_atomic,
            gain_linear,
            pitch_semitones,
        },
        bpm_rx,
    );
    deck.cover_art = read_cover_art(path);
    deck
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    initial_load: Option<PendingLoad>,
    cache: &mut Cache,
    browser_dir: &mut std::path::PathBuf,
    mixer: &rodio::mixer::Mixer,
) -> io::Result<()> {
    // Per-deck display values computed each frame from current deck state.
    struct DeckRenderState {
        display_samp:     f64,
        display_pos_samp: usize,
        analysing:        bool,
        spinner_active:   bool,
        beat_on:          bool,
        warning_active:   bool,
        warn_beat_on:     bool,
    }
    let (keymap, display_cfg, config_notice) = load_config();
    let mut global_notification: Option<Notification> = None;
    if let Some(msg) = config_notice {
        global_notification = Some(Notification {
            message: msg,
            style: NotificationStyle::Success,
            expires: Instant::now() + NOTIFICATION_TIMEOUT,
        });
    }
    let mut decks: [Option<Deck>; 2] = [None, None];
    let mut pending_loads: [Option<PendingLoad>; 2] = [initial_load, None];
    if pending_loads[0].is_none() && global_notification.is_none() {
        global_notification = Some(Notification {
            message: "No track loaded — press z to open the file browser".to_string(),
            style: NotificationStyle::Info,
            expires: Instant::now() + Duration::from_secs(60),
        });
    }
    const DET_MIN: u16 = 3;
    let mut audio_latency_ms: i64 = ((cache.get_latency() as f64 / 10.0).round() as i64 * 10).clamp(0, 250);
    let mut scheme_idx: usize = 0;
    let mut art_bright_idx: u8 = cache.get_art_bright_idx();
    let mut zoom_idx: usize = DEFAULT_ZOOM_IDX;
    let mut vinyl_mode: bool = cache.get_vinyl_mode();
    let shared_renderer = SharedDetailRenderer::new(zoom_idx);
    let mut detail_height: usize = display_cfg.detail_height.max(DET_MIN as usize);
    let mut frame_count: usize = 0;
    let mut last_render = Instant::now();
    let mut help_open = false;
    let mut browser_state: Option<(BrowserState, usize)> = None; // (state, target deck slot)
    let mut preview_output: Option<PreviewOutput> = None;
    let mut max_det_h: usize = usize::MAX;
    let pfl_active_deck = Arc::new(AtomicUsize::new(usize::MAX));
    let mut space_held = false;
    // After a chord fires, suppress further Space-Press events until at least one frame
    // passes with no Space activity. Crossterm decodes Kitty key-repeats as Press events,
    // so without this guard the repeat stream re-arms space_held immediately after the
    // post-chord reset, leaving the modifier stuck until the repeats stop — which never
    // happens via a Release event (those also don't arrive in crossterm 0.29 + Kitty).
    let mut space_repeat_suppressed = false;
    let mut space_saw_event_this_frame = false;
    let mut pending_quit: Option<Instant> = None;
    let mut browser_blocked: Option<(Instant, usize)> = None; // (expiry, target deck slot)
    let mut bpm_ramp_started: Option<Instant> = None;
    let mut bpm_ramp_last: Option<Instant> = None;
    // When Esc dismisses an overlay, suppress the next Quit action for a short window.
    // This absorbs the duplicate Esc event that Kitty injects after the first one.
    let mut suppress_quit_until: Option<Instant> = None;

    'tui: loop {
        frame_count += 1;

        // Clear the repeat-suppression latch once a full frame passes with no Space events,
        // indicating the key has been physically released.
        if space_repeat_suppressed && !space_saw_event_this_frame {
            space_repeat_suppressed = false;
        }
        space_saw_event_this_frame = false;

        // Frame timing — computed once and shared by both decks.
        let dc = shared_renderer.cols.load(Ordering::Relaxed);
        let zoom_secs = ZOOM_LEVELS[zoom_idx];
        let col_secs = if dc > 0 { zoom_secs as f64 / dc as f64 } else { 0.033 };

        // Frame budget: one half-column of scroll time, clamped to a sane range.
        // Sleep is deferred to the END of the loop so variable draw/write time is absorbed
        // automatically — the sleep shrinks to compensate for a slow terminal flush.
        // When the tag editor is open, bypass the waveform-derived budget entirely so text
        // navigation and input are never throttled by zoom level.
        let tag_editor_open = decks.iter().flatten().any(|d| d.tag_editor.is_some());
        let frame_dur = if tag_editor_open {
            Duration::from_millis(16)
        } else {
            Duration::from_secs_f64(col_secs / 2.0)
                .max(Duration::from_millis(8))
                .min(Duration::from_millis(50))
        };

        let frame_start = Instant::now();
        let elapsed = frame_start.duration_since(last_render).as_secs_f64()
            // Cap at 4 columns per frame. Must exceed the minimum frame_dur (8ms) at every zoom
            // level — a tighter cap causes systematic drift and periodic large-drift snapping.
            .min(col_secs * 4.0);
        last_render = frame_start;

        // Expire global notification.
        if global_notification.as_ref().map_or(false, |n| frame_start >= n.expires) {
            global_notification = None;
        }
        // Complete any pending loads.
        for slot in 0..2 {
            if pending_loads[slot].is_none() { continue; }
            let recv = pending_loads[slot].as_ref().unwrap().rx.try_recv();
            match recv {
                Ok(Ok((mono, stereo, sample_rate, channels))) => {
                    let pending = pending_loads[slot].take().unwrap();
                    let new_deck = build_deck(&pending.path, pending.filename, mono, stereo, sample_rate, channels, mixer, cache, Arc::clone(&pfl_active_deck), slot);
                    shared_renderer.set_deck(slot, Arc::clone(&new_deck.audio.waveform), new_deck.audio.seek_handle.channels, new_deck.audio.sample_rate);
                    decks[slot] = Some(new_deck);
                    if let Some(ref mut d) = decks[slot] {
                        d.display.palette = if slot == 0 { PALETTE_SCHEMES[scheme_idx].1 } else { PALETTE_SCHEMES[scheme_idx].2 };
                    }
                }
                Ok(Err(e)) => {
                    global_notification = Some(Notification {
                        message: format!("Load failed: {e}"),
                        style: NotificationStyle::Error,
                        expires: Instant::now() + NOTIFICATION_TIMEOUT,
                    });
                    pending_loads[slot] = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => { pending_loads[slot] = None; }
            }
        }

        // Service both decks: BPM results, position, metronome, tap timeout, spectrum.
        for slot in 0..2 {
            service_deck_frame(slot, &mut decks, col_secs, frame_dur, elapsed, mixer, &shared_renderer, cache, audio_latency_ms, vinyl_mode);
        }

        // Compute render state for both decks.
        let render: [Option<DeckRenderState>; 2] = std::array::from_fn(|slot| {
            let d = decks[slot].as_ref()?;
            // Latency correction only applies during playback — when paused there is
            // no buffer fill ahead, so the raw position is the heard position.
            let latency_correction = if d.audio.player.is_paused() { 0.0 } else { audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0 };
            let display_samp = (d.display.smooth_display_samp - latency_correction).max(0.0);
            let display_pos_samp = display_samp as usize;
            let pos_interleaved  = display_pos_samp * d.audio.seek_handle.channels as usize;
            if slot == 0 {
                shared_renderer.display_pos_a.store(pos_interleaved, Ordering::Relaxed);
            } else {
                shared_renderer.display_pos_b.store(pos_interleaved, Ordering::Relaxed);
            }
            let spinner_active = d.tempo.analysis_hash.is_none();
            let analysing      = vinyl_mode || spinner_active || !d.tempo.bpm_established;
            let beat_period    = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
            let flash_window   = beat_period.mul_f64(0.15);
            let smooth_pos_ns  = (display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                - d.tempo.offset_ms as i128 * 1_000_000;
            let phase          = smooth_pos_ns.rem_euclid(beat_period.as_nanos() as i128);
            let beat_on        = phase < flash_window.as_nanos() as i128;
            let audio_pos_samp = d.audio.seek_handle.position.load(Ordering::Relaxed)
                / d.audio.seek_handle.channels as usize;
            let pos_dur        = Duration::from_secs_f64(audio_pos_samp as f64 / d.audio.sample_rate as f64);
            let remaining_secs = d.total_duration - pos_dur.as_secs_f64();
            let warning_active = !d.audio.player.is_paused()
                && remaining_secs < display_cfg.warning_threshold_secs as f64;
            let beat_index     = smooth_pos_ns.div_euclid(beat_period.as_nanos() as i128);
            let warn_beat_on   = warning_active && (beat_index % 2 == 0);
            Some(DeckRenderState { display_samp, display_pos_samp, analysing, spinner_active, beat_on, warning_active, warn_beat_on })
        });

        shared_renderer.zoom_at.store(zoom_idx, Ordering::Relaxed);
        let buf_a = Arc::clone(&*shared_renderer.shared_a.lock().unwrap());
        let buf_b = Arc::clone(&*shared_renderer.shared_b.lock().unwrap());
        let scrub_spc_a = buf_a.samples_per_col;
        let scrub_spc_b = buf_b.samples_per_col;

        // Take both decks out so the draw closure can mutate them.
        let mut d0 = decks[0].take();
        let mut d1 = decks[1].take();

        // Compute loading labels for slots that have a pending load but no deck.
        let loading_label: [Option<String>; 2] = std::array::from_fn(|slot| {
            let p = pending_loads[slot].as_ref()?;
            let done  = p.decoded.load(Ordering::Relaxed);
            let total = p.total.load(Ordering::Relaxed);
            let pct   = if total > 0 { format!(" {}%", (done * 100 / total).min(100)) } else { String::new() };
            Some(format!("Loading {}…{}", p.filename, pct))
        });

        terminal.draw(|frame| {
            let area = frame.area();
            let inner = area;

            // Compression order as the terminal shrinks:
            //   1. Detail waveforms compress evenly: detail_height → DET_MIN
            //   2. Overview waveforms compress evenly: OV_MAX → OV_MIN
            //   3. No further compression — elements fall off the bottom
            //
            // Row heights are pre-computed and sum exactly to inner.height so the
            // cassowary solver never receives an infeasible system and proportionally
            // shrinks things it shouldn't.
            const OV_MAX:  u16 = 3;
            const OV_MIN:  u16 = 2;
            let det_max = detail_height as u16;
            let ih = inner.height;
            let fixed = 7_u16; // global + detail-info + shared-tick + notif×2 + info×2

            // Cap detail_height to what the current terminal can actually display,
            // so HeightIncrease never outruns the screen.
            max_det_h = (ih.saturating_sub(fixed + OV_MIN * 2) / 2) as usize;

            // Compute a unified pool for each waveform type so both decks always
            // get the same height (no sequential-allocation asymmetry).
            // Phase 1: detail compresses; overviews stay at OV_MAX.
            // Phase 2: overviews compress; detail stays at DET_MIN.
            // Phase 3: items fall off bottom (heights stay at minimums).
            let total_variable = ih.saturating_sub(fixed);
            let det_full = det_max * 2;
            let ov_full  = OV_MAX * 2;

            let (both_det, both_ov) = if total_variable >= det_full + ov_full {
                (det_full, ov_full)
            } else if total_variable >= DET_MIN * 2 + ov_full {
                (total_variable - ov_full, ov_full)
            } else if total_variable >= DET_MIN * 2 + OV_MIN * 2 {
                (DET_MIN * 2, total_variable - DET_MIN * 2)
            } else {
                let d = total_variable.min(DET_MIN * 2);
                (d, total_variable.saturating_sub(d))
            };

            // Clamp to minimums: the pool calculation drives compression through
            // the normal phase range; below minimum, take_h handles falloff.
            let effective_det_h = (both_det / 2).max(DET_MIN).min(det_max);
            let effective_ov_h  = (both_ov  / 2).clamp(OV_MIN, OV_MAX);

            // Allocate rows top-to-bottom using take_exact for all waveform rows:
            // each waveform shows at its computed height or disappears entirely.
            // This prevents partial heights below the minimum (e.g. a 3-row
            // detail area where the tick rows leave only 1 waveform row).
            let mut rem = ih;
            // take: allocate up to n rows (partial ok — used for 1-row fixed items).
            // take_consume: show at full height or not at all, but always consume
            //   up to n rows so freed space cannot cause lower items to reappear.
            let take         = |rem: &mut u16, n: u16| -> u16 { let h = (*rem).min(n); *rem -= h; h };
            let take_consume = |rem: &mut u16, n: u16| -> u16 {
                let actual = if *rem >= n { n } else { 0 };
                *rem = rem.saturating_sub(n);
                actual
            };
            let hh = [
                take(&mut rem, 1),                      // 0: global bar
                take(&mut rem, 1),                      // 1: detail info bar
                take_consume(&mut rem, effective_det_h), // 2: detail A
                take(&mut rem, 1),                      // 3: shared tick row
                take_consume(&mut rem, effective_det_h), // 4: detail B
                take(&mut rem, 1),                      // 5: notif A
                take(&mut rem, 1),                      // 6: info A
                take_consume(&mut rem, effective_ov_h),  // 7: overview A
                take(&mut rem, 1),                      // 8: notif B
                take(&mut rem, 1),                      // 9: info B
                take_consume(&mut rem, effective_ov_h),  // 10: overview B
                rem,                                    // 11: spacer (leftover)
            ];

            let c = Layout::default()
                .direction(Direction::Vertical)
                .constraints(hh.map(Constraint::Length))
                .split(inner);
            let (area_detail_info, area_detail_a, area_tick,
                 area_detail_b, area_notif_a, area_info_a, area_overview_a,
                 area_notif_b, area_info_b, area_overview_b,
                 area_global) = (c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[10], c[0]);

            // Update renderer dimensions from layout.
            {
                let w = area_detail_a.width as usize;
                let h = area_detail_a.height as usize;
                if w > 0 && h > 0 {
                    shared_renderer.cols.store(w, Ordering::Relaxed);
                    shared_renderer.rows.store(h, Ordering::Relaxed);
                }
            }

            // Update tempo and cue state for background buffer rendering.
            // In vinyl mode: suppress ticks (analysing=true) and cue column.
            for (slot, deck) in [(0usize, d0.as_ref()), (1, d1.as_ref())] {
                let (base_bpm, offset_ms, analysing, cue_sample) = deck.map(|d| {
                    let analysing = vinyl_mode || d.tempo.analysis_hash.is_none() || !d.tempo.bpm_established;
                    let cue = if vinyl_mode { None } else { d.cue_sample };
                    (d.tempo.base_bpm, d.tempo.offset_ms, analysing, cue)
                }).unwrap_or((0.0, 0, true, None));
                shared_renderer.store_tempo(slot, base_bpm, offset_ms, analysing);
                shared_renderer.store_cue(slot, cue_sample);
            }

            // Detail info bar
            {
                let nudge_label = match d0.as_ref().or(d1.as_ref()).map(|d| d.nudge_mode) {
                    Some(NudgeMode::Warp) => "  [WARP]",
                    _ => "  [JUMP]",
                };
                let spc_label   = if space_held { "  [SPC]" } else { "" };
                let vinyl_label = if vinyl_mode { "  [VINYL]" } else { "  [BEAT]" };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        format!("  zoom:{}s  lat:{}ms{}{}{}", zoom_secs, audio_latency_ms, nudge_label, vinyl_label, spc_label),
                        Style::default().fg(Color::DarkGray),
                    ))),
                    area_detail_info,
                );
            }

            let label_style = Style::default().fg(Color::Rgb(40, 60, 100));
            let notif_bg    = Style::default().bg(Color::Rgb(20, 20, 38));

            // Extract tick viewport slices for the shared tick row.
            let tick_w = area_tick.width as usize;
            let tick_centre = ((tick_w as f64 * display_cfg.playhead_position as f64 / 100.0) as usize)
                .clamp(0, tick_w.saturating_sub(1));
            let pos_a = render[0].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
            let pos_b = render[1].as_ref().map(|rs| rs.display_pos_samp).unwrap_or(0);
            let tick_a = extract_tick_viewport(&buf_a, pos_a, tick_centre, tick_w);
            let tick_b = extract_tick_viewport(&buf_b, pos_b, tick_centre, tick_w);
            render_shared_tick_row(frame, area_tick, &tick_a, &tick_b);

            // ---- Deck 1 ----
            if let (Some(deck), Some(rs)) = (&mut d0, &render[0]) {
                let content = notification_line_for_deck(deck, area_notif_a.width.saturating_sub(2) as usize, vinyl_mode);
                let mut spans = vec![Span::styled("1 ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_a);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_a.width, vinyl_mode);
                frame.render_widget(Paragraph::new(info), area_info_a);
                let (ov, bar_cols, bar_times) = overview_for_deck(deck, area_overview_a, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                deck.display.overview_rect  = area_overview_a;
                deck.display.last_bar_cols  = bar_cols;
                deck.display.last_bar_times = bar_times;
                frame.render_widget(Paragraph::new(ov), area_overview_a);
                render_detail_waveform(frame, &buf_a, deck, area_detail_a, &display_cfg, rs.display_pos_samp, deck.display.palette);
            } else {
                let mut spans = vec![Span::styled("A ", label_style)];
                if let Some(ref s) = loading_label[0] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_a);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_a.width)), area_info_a);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_a, 0)), area_overview_a);
                render_detail_empty(frame, area_detail_a, 0);
            }

            // ---- Deck 2 ----
            if let (Some(deck), Some(rs)) = (&mut d1, &render[1]) {
                let content = notification_line_for_deck(deck, area_notif_b.width.saturating_sub(2) as usize, vinyl_mode);
                let mut spans = vec![Span::styled("2 ", label_style)];
                spans.extend(content.spans);
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_b);
                let info = info_line_for_deck(deck, frame_count, rs.beat_on, rs.spinner_active, label_style, area_info_b.width, vinyl_mode);
                frame.render_widget(Paragraph::new(info), area_info_b);
                let (ov, bar_cols, bar_times) = overview_for_deck(deck, area_overview_b, rs.display_samp, rs.analysing, rs.warning_active, rs.warn_beat_on);
                deck.display.overview_rect  = area_overview_b;
                deck.display.last_bar_cols  = bar_cols;
                deck.display.last_bar_times = bar_times;
                frame.render_widget(Paragraph::new(ov), area_overview_b);
                render_detail_waveform(frame, &buf_b, deck, area_detail_b, &display_cfg, rs.display_pos_samp, deck.display.palette);
            } else {
                let mut spans = vec![Span::styled("B ", label_style)];
                if let Some(ref s) = loading_label[1] {
                    spans.push(Span::styled(s.clone(), Style::default().fg(Color::DarkGray)));
                } else {
                    spans.extend(notification_line_empty().spans);
                }
                frame.render_widget(Paragraph::new(Line::from(spans)).style(notif_bg), area_notif_b);
                frame.render_widget(Paragraph::new(info_line_empty(area_info_b.width)), area_info_b);
                frame.render_widget(Paragraph::new(overview_empty(area_overview_b, 1)), area_overview_b);
                render_detail_empty(frame, area_detail_b, 1);
            }

            // ---- Global status bar ----
            {
                if pending_quit.map_or(false, |e| Instant::now() > e) { pending_quit = None; }
                if browser_blocked.map_or(false, |(e, _)| Instant::now() > e) { browser_blocked = None; global_notification = None; }
                let notification_bar = |msg: &str, expires: Instant, fg: Color, bg: Color, countdown_fg: Color| {
                    let secs = expires.saturating_duration_since(Instant::now()).as_secs();
                    let countdown = format!("[{}]", secs);
                    let w = area_global.width as usize;
                    let inner = w.saturating_sub(countdown.len());
                    let pad = inner.saturating_sub(msg.len()) / 2;
                    let centred = format!("{:pad$}{msg}", "");
                    let fill = inner.saturating_sub(pad + msg.len());
                    let line = Line::from(vec![
                        Span::styled(format!("{centred}{:fill$}", ""), Style::default().fg(fg)),
                        Span::styled(countdown, Style::default().fg(countdown_fg)),
                    ]);
                    (line, Style::default().bg(bg))
                };
                let (global_line, bar_style) = if let Some(quit_expires) = pending_quit {
                    notification_bar("Track is playing — quit?  [y] quit   [Esc/n] cancel", quit_expires,
                        Color::Rgb(255, 180, 180), Color::Rgb(100, 20, 20), Color::Rgb(200, 120, 120))
                } else if let Some(ref n) = global_notification {
                    match n.style {
                        NotificationStyle::Error =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(255, 180, 180), Color::Rgb(100, 20, 20), Color::Rgb(200, 120, 120)),
                        NotificationStyle::Warning =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(255, 220, 120), Color::Rgb(80, 60, 0), Color::Rgb(200, 160, 80)),
                        NotificationStyle::Info =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(160, 200, 255), Color::Rgb(20, 40, 80), Color::Rgb(100, 140, 200)),
                        NotificationStyle::Success =>
                            notification_bar(&n.message, n.expires,
                                Color::Rgb(140, 230, 160), Color::Rgb(10, 60, 30), Color::Rgb(80, 170, 100)),
                    }
                } else {
                    let version = format!(" {} ", env!("CARGO_PKG_VERSION"));
                    let dir     = format!("  {}", browser_dir.display());
                    let w       = area_global.width as usize;
                    let pad     = w.saturating_sub(dir.len() + version.len());
                    let line    = Line::from(vec![
                        Span::styled(dir, Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:pad$}", ""), Style::default()),
                        Span::styled(version, Style::default().fg(Color::DarkGray)),
                    ]);
                    (line, notif_bg)
                };
                frame.render_widget(Paragraph::new(global_line).style(bar_style), area_global);
            }

            // ---- Browser / cover art (spacer row) ----
            if let Some((ref bs, slot)) = browser_state {
                if c[11].height >= 8 {
                    render_browser(frame, c[11], bs, slot);
                } else {
                    // Art area too small — render browser fullscreen.
                    frame.render_widget(ratatui::widgets::Clear, inner);
                    render_browser(frame, inner, bs, slot);
                }
            } else if c[11].height >= 3 && art_bright_idx < 2 {
                let brightness = [1.0f32, 0.35, 0.0][art_bright_idx as usize];
                // 1-row top margin separates art from deck 2 above.
                let vert = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(0)])
                    .split(c[11]);
                let art_row = vert[1];
                // 1-column centre gap between the two panels.
                let art_areas = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1)])
                    .split(art_row);
                for (idx, deck_opt) in [&mut d0, &mut d1].iter_mut().enumerate() {
                    let panel_idx = idx * 2; // indices 0 and 2; index 1 is the gap
                    if let Some(deck) = deck_opt {
                        if let Some(ref bytes) = deck.cover_art {
                            let a = art_areas[panel_idx];
                            let cached = deck.cover_art_cache.get_or_insert_with(|| {
                                (a.width, a.height, art_bright_idx,
                                 halfblock_art(bytes, a.width, a.height, brightness))
                            });
                            if cached.0 != a.width || cached.1 != a.height || cached.2 != art_bright_idx {
                                *cached = (a.width, a.height, art_bright_idx,
                                           halfblock_art(bytes, a.width, a.height, brightness));
                            }
                            frame.render_widget(Paragraph::new(cached.3.clone()), a);
                        }
                    }
                }
            }

            // Tag editor overlay
            for deck_opt in [&d0, &d1] {
                if let Some(deck) = deck_opt {
                    if let Some(ref editor) = deck.tag_editor {
                        render_tag_editor(frame, editor, area);
                    }
                }
            }

            // Help popup
            if help_open {
                const HELP: &str = "\
Space+Z / Space+C    play / pause  Deck 1 / Deck 2
Z / C                load file into Deck 1 / Deck 2
A / Space+A          Deck 1 cue set / cue play   D / Space+D  Deck 2
Space+x / Space+v    Deck 1/2 PFL toggle
1/2  q/w             Deck 1 jump ±4/±8 bars    Space+1/2  q/w  ±1/±4 beats
3/4  e/r             Deck 2 jump ±4/±8 bars    Space+3/4  e/r  ±1/±4 beats
a / z                Deck 1 nudge fwd / bwd     d / c  Deck 2
|                    toggle nudge mode: jump (10ms) / warp (±10%)
j/m  k/,             Deck 1/2 level up/down     Space+j/m  Space+k/,  snap 100%/0%
J/M  K/<             Deck 1/2 gain trim ±1 dB   (range ±12 dB; cached per track)
7/u  i/8             Deck 1/2 filter sweep      Space+7/u  i/8  filter reset
s/x  f/v             Deck 1/2 BPM ±0.1  (vinyl: speed ±0.1%)  S/X  F/V  base BPM ±0.01
@  /  ~              Deck 1/2 tempo reset
b  /  n              Deck 1/2 tap BPM            B / N  metronome
'  /  #              Deck 1/2 BPM redetect       (suppressed in vinyl mode)
!  Q                 Deck 1 offset ±10ms         £  E  Deck 2
[  /  ]              latency ±10ms
-  /  =              zoom in / out
{  /  }              detail height decrease / increase
`                    toggle vinyl mode
¬                    refresh terminal
?                    toggle this help
Esc                  close this / quit";
                let popup_w = 75u16;
                let popup_h = HELP.lines().count() as u16 + 2;
                let px = area.x + area.width.saturating_sub(popup_w) / 2;
                let py = area.y + area.height.saturating_sub(popup_h) / 2;
                let popup_rect = ratatui::layout::Rect {
                    x: px, y: py,
                    width: popup_w.min(area.width),
                    height: popup_h.min(area.height),
                };
                frame.render_widget(ratatui::widgets::Clear, popup_rect);
                frame.render_widget(
                    Paragraph::new(HELP)
                        .block(Block::default().borders(Borders::ALL).title(" Key Bindings "))
                        .style(Style::default().fg(Color::White)),
                    popup_rect,
                );
            }
        })?;

        // Put both decks back after render.
        decks[0] = d0;
        decks[1] = d1;

        // Single event handler — all actions work regardless of which deck is loaded.
        while event::poll(Duration::ZERO)? {
            match event::read()? {
            Event::Mouse(mouse_event) => {
                if browser_state.is_some() { continue; }
                if decks.iter().flatten().any(|d| d.tag_editor.is_some()) { continue; }
                if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
                    let col = mouse_event.column as usize;
                    let row = mouse_event.row as usize;
                    for slot in 0..2 {
                        if let Some(ref d) = decks[slot] {
                            let rect = d.display.overview_rect;
                            if col >= rect.x as usize && col < (rect.x + rect.width) as usize
                                && row >= rect.y as usize && row < (rect.y + rect.height) as usize
                            {
                                let click_col = col - rect.x as usize;
                                let target_secs = if vinyl_mode {
                                    d.total_duration * click_col as f64 / rect.width as f64
                                } else {
                                    d.display.last_bar_cols.iter()
                                        .zip(d.display.last_bar_times.iter())
                                        .filter(|(c, _)| **c <= click_col)
                                        .last()
                                        .map(|(_, t)| *t)
                                        .unwrap_or(0.0)
                                };
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(target_secs);
                                } else {
                                    d.audio.seek_handle.seek_to(target_secs);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Event::Key(key) => {
                // Ctrl-C: unconditional quit.
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    for slot in 0..2 {
                        if let Some(ref d) = decks[slot] {
                            d.audio.player.stop();
                            if let Some(ref hash) = d.tempo.analysis_hash {
                                cache.set(hash.clone(), cache_entry_for_deck(d));
                            }
                        }
                    }
                    cache.save();
                    return Ok(());
                }
                // Tag editor — intercepts all key events when open, before any other handling.
                {
                    let editor_open = decks.iter().flatten().any(|d| d.tag_editor.is_some());
                    if editor_open {
                        if let KeyEventKind::Press = key.kind {
                            for slot in 0..2 {
                                if let Some(ref mut d) = decks[slot] {
                                    if let Some(ref mut editor) = d.tag_editor {
                                        match key.code {
                                            KeyCode::Esc => { d.tag_editor = None; }
                                            KeyCode::Enter => {
                                                let artist_blank = editor.fields[0].0.trim().is_empty();
                                                let title_blank  = editor.fields[1].0.trim().is_empty();
                                                if !artist_blank && !title_blank {
                                                    for (val, cursor) in &mut editor.fields {
                                                        let trimmed = val.trim().to_string();
                                                        *cursor = (*cursor).min(trimmed.chars().count());
                                                        *val = trimmed;
                                                    }
                                                    let new_stem  = editor.preview();
                                                    let extension = editor.extension.clone();
                                                    let needs_rename = new_stem != editor.current_stem;
                                                    let target_path = {
                                                        let parent = d.path.parent()
                                                            .unwrap_or_else(|| std::path::Path::new("."));
                                                        if extension.is_empty() {
                                                            parent.join(&new_stem)
                                                        } else {
                                                            parent.join(format!("{new_stem}.{extension}"))
                                                        }
                                                    };
                                                    if needs_rename && target_path.exists() {
                                                        let filename = target_path
                                                            .file_name()
                                                            .and_then(|n| n.to_str())
                                                            .unwrap_or("")
                                                            .to_string();
                                                        editor.collision_error = Some(format!("already exists: {filename}"));
                                                    } else {
                                                        editor.collision_error = None;
                                                        let fields_snapshot: Vec<(String, usize)> = editor.fields.clone();
                                                        d.tag_editor = None;
                                                        match crate::tags::write_tags(&d.path, &fields_snapshot) {
                                                            Err(e) => {
                                                                d.active_notification = Some(Notification {
                                                                    message: format!("tag write failed: {e}"),
                                                                    style: NotificationStyle::Error,
                                                                    expires: Instant::now() + Duration::from_secs(5),
                                                                });
                                                            }
                                                            Ok(()) => {
                                                                if needs_rename {
                                                                    match std::fs::rename(&d.path, &target_path) {
                                                                        Err(e) => {
                                                                            d.active_notification = Some(Notification {
                                                                                message: format!("rename failed: {e}"),
                                                                                style: NotificationStyle::Error,
                                                                                expires: Instant::now() + Duration::from_secs(5),
                                                                            });
                                                                        }
                                                                        Ok(()) => {
                                                                            if target_path.exists() {
                                                                                d.path = target_path.clone();
                                                                                d.filename = target_path
                                                                                    .file_name()
                                                                                    .and_then(|n| n.to_str())
                                                                                    .unwrap_or("")
                                                                                    .to_string();
                                                                                d.track_name = format!(
                                                                                    "{} \u{2013} {}",
                                                                                    fields_snapshot[1].0,
                                                                                    fields_snapshot[0].0,
                                                                                );
                                                                                d.rename_hint = None;
                                                                                d.rename_offer_started = None;
                                                                                d.active_notification = Some(Notification {
                                                                                    message: format!("\u{2192} {new_stem}"),
                                                                                    style: NotificationStyle::Success,
                                                                                    expires: Instant::now() + NOTIFICATION_TIMEOUT,
                                                                                });
                                                                            } else {
                                                                                d.active_notification = Some(Notification {
                                                                                    message: "rename could not be verified".to_string(),
                                                                                    style: NotificationStyle::Error,
                                                                                    expires: Instant::now() + Duration::from_secs(5),
                                                                                });
                                                                            }
                                                                        }
                                                                    }
                                                                } else {
                                                                    d.track_name = format!(
                                                                        "{} \u{2013} {}",
                                                                        fields_snapshot[1].0,
                                                                        fields_snapshot[0].0,
                                                                    );
                                                                    d.rename_hint = None;
                                                                    d.rename_offer_started = None;
                                                                    d.active_notification = Some(Notification {
                                                                        message: "tags saved".to_string(),
                                                                        style: NotificationStyle::Info,
                                                                        expires: Instant::now() + NOTIFICATION_TIMEOUT,
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            KeyCode::Tab | KeyCode::Down => {
                                                editor.active_field = (editor.active_field + 1) % TAG_FIELD_LABELS.len();
                                            }
                                            KeyCode::BackTab | KeyCode::Up => {
                                                editor.active_field = (editor.active_field + TAG_FIELD_LABELS.len() - 1) % TAG_FIELD_LABELS.len();
                                            }
                                            KeyCode::Left => {
                                                let (_, cursor) = editor.active_field_mut();
                                                if *cursor > 0 { *cursor -= 1; }
                                            }
                                            KeyCode::Right => {
                                                let (text, cursor) = editor.active_field_mut();
                                                let len = text.chars().count();
                                                if *cursor < len { *cursor += 1; }
                                            }
                                            KeyCode::Home => {
                                                let (_, cursor) = editor.active_field_mut();
                                                *cursor = 0;
                                            }
                                            KeyCode::End => {
                                                let (text, cursor) = editor.active_field_mut();
                                                *cursor = text.chars().count();
                                            }
                                            KeyCode::Backspace => {
                                                let (text, cursor) = editor.active_field_mut();
                                                if *cursor > 0 {
                                                    let mut chars: Vec<char> = text.chars().collect();
                                                    chars.remove(*cursor - 1);
                                                    *text = chars.into_iter().collect();
                                                    *cursor -= 1;
                                                }
                                            }
                                            KeyCode::Delete => {
                                                let (text, cursor) = editor.active_field_mut();
                                                let mut chars: Vec<char> = text.chars().collect();
                                                if *cursor < chars.len() {
                                                    chars.remove(*cursor);
                                                    *text = chars.into_iter().collect();
                                                }
                                            }
                                            KeyCode::Char(c) => {
                                                let (text, cursor) = editor.active_field_mut();
                                                let mut chars: Vec<char> = text.chars().collect();
                                                chars.insert(*cursor, c);
                                                *text = chars.into_iter().collect();
                                                *cursor += 1;
                                            }
                                            _ => {}
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        continue; // block all other key handling while editor is open
                    }
                }
                // Browser — intercepts all key events when open.
                if let Some((ref mut bs, target)) = browser_state {
                    let target = target;

                    // # starts (or restarts) preview of the highlighted audio file.
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('#') {
                        if let Some(ref po) = preview_output {
                            if let Some(path) = bs.highlighted_audio_path() {
                                po.play(&path);
                            }
                        }
                        continue;
                    }

                    // Any other key stops an active preview before being handled normally.
                    if let Some(ref po) = preview_output {
                        po.stop();
                    }

                    match handle_browser_key(bs, key)? {
                        Some(BrowserResult::ReturnToPlayer) => {
                            *browser_dir = bs.cwd.clone();
                            cache.set_last_browser_path(browser_dir);
                            cache.save();
                            browser_state = None;
                            preview_output = None;
                        }
                        Some(BrowserResult::Selected(path)) => {
                            *browser_dir = bs.cwd.clone();
                            cache.set_last_browser_path(browser_dir);
                            cache.save();
                            if let Some(ref d) = decks[target] { d.audio.player.stop(); }
                            decks[target] = None;
                            pending_loads[target] = Some(start_load(&path));
                            browser_state = None;
                            preview_output = None;
                        }
                        Some(BrowserResult::WorkspaceSet(path)) => {
                            cache.set_workspace(&path);
                            cache.save();
                        }
                        Some(BrowserResult::WorkspaceCleared) => {
                            cache.browser_workspace = None;
                            cache.save();
                        }
                        Some(BrowserResult::Quit) => {
                            *browser_dir = bs.cwd.clone();
                            cache.set_last_browser_path(browser_dir);
                            for slot in 0..2 { if let Some(ref d) = decks[slot] { d.audio.player.stop(); } }
                            cache.save();
                            return Ok(());
                        }
                        None => {}
                    }
                    continue; // block all player key handling while browser is open
                }
                // Space modifier: track held state for chords.
                if key.code == KeyCode::Char(' ') {
                    space_saw_event_this_frame = true;
                    match key.kind {
                        KeyEventKind::Press | KeyEventKind::Repeat => {
                            if !space_repeat_suppressed { space_held = true; }
                        }
                        KeyEventKind::Release => {
                            space_held = false;
                            space_repeat_suppressed = false;
                        }
                    }
                }
                // Nudge and mode toggle — handled for all key kinds (Release must be detected).
                match key.kind {
                    KeyEventKind::Press
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::NudgeModeToggle) =>
                    {
                        let new_mode = decks.iter().flatten().next()
                            .map(|d| match d.nudge_mode {
                                NudgeMode::Jump => NudgeMode::Warp,
                                NudgeMode::Warp => NudgeMode::Jump,
                            })
                            .unwrap_or(NudgeMode::Jump);
                        for slot in 0..2 {
                            if let Some(ref mut d) = decks[slot] {
                                if d.nudge != 0 {
                                    d.nudge = 0;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                }
                                d.nudge_mode = new_mode;
                            }
                        }
                    }
                    // Cue handlers come before nudge so that Space+nudge-key resolves to
                    // cue (via SpaceChord lookup) rather than nudge.
                    // Press guard requires space_held to avoid firing on bare nudge-key presses.
                    KeyEventKind::Press
                        if space_held && keymap.get(&KeyBinding::SpaceChord(key.code)) == Some(&Action::Deck1CuePlay) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            if let Some(cue_samp) = d.cue_sample {
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                    d.display.smooth_display_samp = cue_samp as f64;
                                } else {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    let target_samp = (cue_samp + latency_samps).min(d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize);
                                    d.audio.seek_handle.seek_to(target_samp as f64 / d.audio.sample_rate as f64);
                                }
                            }
                            space_held = false;
                            space_repeat_suppressed = true;
                        }
                    }
                    KeyEventKind::Press
                        if space_held && keymap.get(&KeyBinding::SpaceChord(key.code)) == Some(&Action::Deck2CuePlay) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            if let Some(cue_samp) = d.cue_sample {
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                    d.display.smooth_display_samp = cue_samp as f64;
                                } else {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    let target_samp = (cue_samp + latency_samps).min(d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize);
                                    d.audio.seek_handle.seek_to(target_samp as f64 / d.audio.sample_rate as f64);
                                }
                            }
                            space_held = false;
                            space_repeat_suppressed = true;
                        }
                    }
                    // Deck 1 nudge
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck1NudgeBackward) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current - 0.010).max(0.0);
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_a);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = -1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 0.9);
                                }
                            }
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck1NudgeForward) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current + 0.010).min(d.total_duration);
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_a);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = 1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 1.1);
                                }
                            }
                        }
                    }
                    KeyEventKind::Release
                        if matches!(keymap.get(&KeyBinding::Key(key.code)),
                            Some(&Action::Deck1NudgeBackward) | Some(&Action::Deck1NudgeForward)) =>
                    {
                        if let Some(ref mut d) = decks[0] {
                            if d.nudge_mode == NudgeMode::Warp {
                                d.nudge = 0;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            }
                        }
                    }
                    // Deck 2 nudge
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck2NudgeBackward) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current - 0.010).max(0.0);
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_b);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = -1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 0.9);
                                }
                            }
                        }
                    }
                    KeyEventKind::Press | KeyEventKind::Repeat
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Deck2NudgeForward) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            match d.nudge_mode {
                                NudgeMode::Jump => {
                                    let current = d.audio.seek_handle.current_pos().as_secs_f64();
                                    let target = (current + 0.010).min(d.total_duration);
                                    d.audio.seek_handle.set_position(target);
                                    d.display.smooth_display_samp += (target - current) * d.audio.sample_rate as f64;
                                    if d.audio.player.is_paused() {
                                        scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                                                    d.audio.sample_rate, d.display.smooth_display_samp as usize, scrub_spc_b);
                                    }
                                }
                                NudgeMode::Warp => {
                                    d.nudge = 1;
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm * 1.1);
                                }
                            }
                        }
                    }
                    KeyEventKind::Release
                        if matches!(keymap.get(&KeyBinding::Key(key.code)),
                            Some(&Action::Deck2NudgeBackward) | Some(&Action::Deck2NudgeForward)) =>
                    {
                        if let Some(ref mut d) = decks[1] {
                            if d.nudge_mode == NudgeMode::Warp {
                                d.nudge = 0;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                            }
                        }
                    }
                    _ => {}
                }
                // Base BPM ramp — fires on Press and Repeat with time-based step size.
                // The ramp resets only when no base-BPM key has been seen for >500 ms,
                // so a quick release-and-repress continues at the current tier.
                if !vinyl_mode
                    && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                    && matches!(keymap.get(&KeyBinding::Key(key.code)),
                        Some(&Action::Deck1BaseBpmIncrease) | Some(&Action::Deck1BaseBpmDecrease) |
                        Some(&Action::Deck2BaseBpmIncrease) | Some(&Action::Deck2BaseBpmDecrease))
                {
                    let gap = bpm_ramp_last.map_or(Duration::MAX, |t| t.elapsed());
                    if gap > Duration::from_millis(80) {
                        bpm_ramp_started = Some(Instant::now());
                    }
                    bpm_ramp_last = Some(Instant::now());
                    let elapsed = bpm_ramp_started.map_or(Duration::ZERO, |t| t.elapsed());
                    let step: f32 = if elapsed >= Duration::from_secs(3) { 0.05 }
                                    else { 0.01 };
                    let action = keymap.get(&KeyBinding::Key(key.code));
                    let (slot, sign) = match action {
                        Some(&Action::Deck1BaseBpmIncrease) => (0,  1.0f32),
                        Some(&Action::Deck1BaseBpmDecrease) => (0, -1.0f32),
                        Some(&Action::Deck2BaseBpmIncrease) => (1,  1.0f32),
                        _                                   => (1, -1.0f32),
                    };
                    if let Some(ref mut d) = decks[slot] {
                        d.tempo.base_bpm = (d.tempo.base_bpm + sign * step).clamp(40.0, 240.0);
                        d.tempo.bpm_established = true;
                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                        shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
                        anchor_beat_grid_to_cue(d);
                        if let Some(ref hash) = d.tempo.analysis_hash {
                            cache.set(hash.clone(), cache_entry_for_deck(d));
                            cache.save();
                        }
                    }
                }
                // All other actions fire on Press only.
                if key.kind == KeyEventKind::Press {
                    // Dismiss any open overlay before processing any other action.
                    if help_open {
                        help_open = false;
                        if keymap.get(&KeyBinding::Key(key.code)) == Some(&Action::Quit) {
                            suppress_quit_until = Some(Instant::now() + Duration::from_millis(300));
                        }
                        continue 'tui;
                    }
                    // Browser-blocked intercept — y overrides, Esc/n cancels.
                    if let Some((_, target)) = browser_blocked {
                        if matches!(key.code, KeyCode::Char('y')) {
                            browser_blocked = None;
                            global_notification = None;
                            let workspace = cache.workspace().map(|p| p.to_path_buf());
                            browser_state = Some((BrowserState::new(browser_dir.clone(), workspace)?, target));
                            preview_output = Some(PreviewOutput::new(mixer));
                        } else if matches!(key.code, KeyCode::Char('n') | KeyCode::Esc) {
                            browser_blocked = None;
                            global_notification = None;
                        }
                        continue 'tui;
                    }
                    // Quit confirmation intercept — y/Enter confirms, anything else cancels.
                    if pending_quit.is_some() {
                        pending_quit = None;
                        if matches!(key.code, KeyCode::Char('y') | KeyCode::Enter) {
                            for slot in 0..2 {
                                if let Some(ref d) = decks[slot] {
                                    d.audio.player.stop();
                                    if let Some(ref hash) = d.tempo.analysis_hash {
                                        cache.set(hash.clone(), cache_entry_for_deck(d));
                                    }
                                }
                            }
                            cache.set_latency(audio_latency_ms);
                            cache.save();
                            return Ok(());
                        }
                        continue 'tui;
                    }
                    // Esc dismisses any active global notification.
                    if global_notification.is_some() && key.code == KeyCode::Esc {
                        global_notification = None;
                        suppress_quit_until = Some(Instant::now() + Duration::from_millis(300));
                        continue 'tui;
                    }
                    // BPM confirmation intercept — check both decks.
                    let mut bpm_intercepted = false;
                    for slot in 0..2 {
                        if let Some(ref mut d) = decks[slot] {
                            if let Some((hash, p_bpm, p_offset, _)) = d.tempo.pending_bpm.take() {
                                if matches!(key.code, KeyCode::Char('y') | KeyCode::Enter) {
                                    d.tempo.bpm = p_bpm;
                                    d.tempo.base_bpm = p_bpm;
                                    d.tempo.offset_ms = (p_offset as f64 / 10.0).round() as i64 * 10;
                                    d.tempo.bpm_established = true;
                                    d.audio.player.set_speed(1.0);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
                                    d.tempo.offset_established = true;
                                    cache.set(hash.clone(), cache_entry_for_deck(d));
                                    cache.save();
                                    d.tempo.analysis_hash = Some(hash);
                                }
                                // Any key dismisses the confirmation.
                                bpm_intercepted = true;
                                break;
                            }
                        }
                    }
                    if bpm_intercepted { continue 'tui; }

                    // Rename offer — 'y' and 'h' are intercepted when offer is visible;
                    // any other key dismisses the offer and falls through to normal handling.
                    let mut rename_offer_consumed = false;
                    for slot in 0..2 {
                        if let Some(ref mut d) = decks[slot] {
                            if d.rename_offer_active() {
                                match key.code {
                                    KeyCode::Char('y') => {
                                        let tag_values = read_tags_for_editor(&d.path);
                                        let current_stem = d.path.file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let extension = d.path.extension()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("")
                                            .to_string();
                                        d.tag_editor = Some(TagEditorState {
                                            fields: tag_values.into_iter()
                                                .map(|v| (v, 0))
                                                .collect(),
                                            active_field: 0,
                                            current_stem,
                                            extension,
                                            collision_error: None,
                                        });
                                        d.rename_offer_started = None;
                                        rename_offer_consumed = true;
                                    }
                                    _ => {
                                        // Key performs normally; offer stays.
                                    }
                                }
                                break;
                            }
                        }
                    }
                    if rename_offer_consumed { continue 'tui; }

                    let action = if space_held && key.code != KeyCode::Char(' ') {
                        if let Some(a) = keymap.get(&KeyBinding::SpaceChord(key.code)) {
                            space_held = false;
                            space_repeat_suppressed = true;
                            Some(a)
                        } else {
                            keymap.get(&KeyBinding::Key(key.code))
                        }
                    } else {
                        keymap.get(&KeyBinding::Key(key.code))
                    };
                    match action {
                    Some(Action::Quit) => {
                        if suppress_quit_until.take().map_or(false, |until| Instant::now() < until) {
                            continue 'tui;
                        }
                        let any_playing = decks.iter().flatten().any(|d| !d.audio.player.is_paused());
                        if any_playing && pending_quit.is_none() {
                            pending_quit = Some(Instant::now() + Duration::from_secs(5));
                            continue 'tui;
                        }
                        for slot in 0..2 {
                            if let Some(ref d) = decks[slot] {
                                d.audio.player.stop();
                                if let Some(ref hash) = d.tempo.analysis_hash {
                                    cache.set(hash.clone(), cache_entry_for_deck(d));
                                }
                            }
                        }
                        cache.set_latency(audio_latency_ms);
                        cache.save();
                        return Ok(());
                    }
                    Some(Action::Deck1OpenBrowser) | Some(Action::Deck2OpenBrowser) => {
                        let target = if action == Some(&Action::Deck1OpenBrowser) { 0 } else { 1 };
                        let playing = decks[target].as_ref().map_or(false, |d| !d.audio.player.is_paused());
                        if playing {
                            let expires = Instant::now() + Duration::from_secs(5);
                            global_notification = Some(Notification {
                                message: "Track is playing — open browser?  [y] open   [Esc/n] cancel".to_string(),
                                style: NotificationStyle::Error,
                                expires,
                            });
                            browser_blocked = Some((expires, target));
                        } else {
                            let workspace = cache.workspace().map(|p| p.to_path_buf());
                            browser_state = Some((BrowserState::new(browser_dir.clone(), workspace)?, target));
                            preview_output = Some(PreviewOutput::new(mixer));
                        }
                    }
                    Some(Action::Deck1PlayPause) => {
                        if let Some(ref d) = decks[0] {
                            if d.audio.player.is_paused() {
                                if d.filter_offset != 0 {
                                    d.audio.filter_state_reset.store(true, Ordering::Relaxed);
                                }
                                d.audio.player.play();
                            } else {
                                d.audio.player.pause();
                            }
                        }
                    }
                    Some(Action::Deck2PlayPause) => {
                        if let Some(ref d) = decks[1] {
                            if d.audio.player.is_paused() {
                                if d.filter_offset != 0 {
                                    d.audio.filter_state_reset.store(true, Ordering::Relaxed);
                                }
                                d.audio.player.play();
                            } else {
                                d.audio.player.pause();
                            }
                        }
                    }
                    Some(Action::Deck1LevelUp)   => { if let Some(ref mut d) = decks[0] { d.volume = (d.volume + 0.05).min(1.0); d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck1LevelDown)  => { if let Some(ref mut d) = decks[0] { d.volume = (d.volume - 0.05).max(0.0); d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck1LevelMax)   => { if let Some(ref mut d) = decks[0] { d.volume = 1.0; d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck1LevelMin)   => { if let Some(ref mut d) = decks[0] { d.volume = 0.0; d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 0 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck2LevelUp)    => { if let Some(ref mut d) = decks[1] { d.volume = (d.volume + 0.05).min(1.0); d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck2LevelDown)  => { if let Some(ref mut d) = decks[1] { d.volume = (d.volume - 0.05).max(0.0); d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck2LevelMax)   => { if let Some(ref mut d) = decks[1] { d.volume = 1.0; d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck2LevelMin)   => { if let Some(ref mut d) = decks[1] { d.volume = 0.0; d.audio.deck_volume_atomic.store(d.volume.to_bits(), Ordering::Relaxed); if pfl_active_deck.load(Ordering::Relaxed) != 1 { d.audio.player.set_volume(d.volume); } } }
                    Some(Action::Deck1PflToggle) => {
                        if pfl_active_deck.load(Ordering::Relaxed) == 0 {
                            if let Some(ref mut d) = decks[0] { d.pfl_level = 0; d.audio.pfl_level.store(0, Ordering::Relaxed); d.audio.player.set_volume(d.volume); }
                            pfl_active_deck.store(usize::MAX, Ordering::Relaxed);
                        } else {
                            if let Some(ref mut d) = decks[1] { d.pfl_level = 0; d.audio.pfl_level.store(0, Ordering::Relaxed); d.audio.player.set_volume(d.volume); }
                            if let Some(ref mut d) = decks[0] { d.pfl_level = 100; d.audio.pfl_level.store(100, Ordering::Relaxed); d.audio.player.set_volume(1.0); }
                            pfl_active_deck.store(0, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck2PflToggle) => {
                        if pfl_active_deck.load(Ordering::Relaxed) == 1 {
                            if let Some(ref mut d) = decks[1] { d.pfl_level = 0; d.audio.pfl_level.store(0, Ordering::Relaxed); d.audio.player.set_volume(d.volume); }
                            pfl_active_deck.store(usize::MAX, Ordering::Relaxed);
                        } else {
                            if let Some(ref mut d) = decks[0] { d.pfl_level = 0; d.audio.pfl_level.store(0, Ordering::Relaxed); d.audio.player.set_volume(d.volume); }
                            if let Some(ref mut d) = decks[1] { d.pfl_level = 100; d.audio.pfl_level.store(100, Ordering::Relaxed); d.audio.player.set_volume(1.0); }
                            pfl_active_deck.store(1, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck1GainIncrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.gain_db = (d.gain_db + 1).min(12);
                            d.audio.gain_linear.store(10f32.powf(d.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                cache.set(hash.clone(), cache_entry_for_deck(d));
                                cache.save();
                            }
                        }
                    }
                    Some(Action::Deck1GainDecrease) => {
                        if let Some(ref mut d) = decks[0] {
                            d.gain_db = (d.gain_db - 1).max(-12);
                            d.audio.gain_linear.store(10f32.powf(d.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                cache.set(hash.clone(), cache_entry_for_deck(d));
                                cache.save();
                            }
                        }
                    }
                    Some(Action::Deck2GainIncrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.gain_db = (d.gain_db + 1).min(12);
                            d.audio.gain_linear.store(10f32.powf(d.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                cache.set(hash.clone(), cache_entry_for_deck(d));
                                cache.save();
                            }
                        }
                    }
                    Some(Action::Deck2GainDecrease) => {
                        if let Some(ref mut d) = decks[1] {
                            d.gain_db = (d.gain_db - 1).max(-12);
                            d.audio.gain_linear.store(10f32.powf(d.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
                            if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                cache.set(hash.clone(), cache_entry_for_deck(d));
                                cache.save();
                            }
                        }
                    }
                    Some(Action::Deck1PitchUp) => {
                        if let Some(ref mut d) = decks[0] {
                            d.pitch_semitones = (d.pitch_semitones + 1).min(6);
                            d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck1PitchDown) => {
                        if let Some(ref mut d) = decks[0] {
                            d.pitch_semitones = (d.pitch_semitones - 1).max(-6);
                            d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck2PitchUp) => {
                        if let Some(ref mut d) = decks[1] {
                            d.pitch_semitones = (d.pitch_semitones + 1).min(6);
                            d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck2PitchDown) => {
                        if let Some(ref mut d) = decks[1] {
                            d.pitch_semitones = (d.pitch_semitones - 1).max(-6);
                            d.audio.pitch_semitones.store(d.pitch_semitones, Ordering::Relaxed);
                        }
                    }
                    Some(Action::Deck1MetronomeToggle) => {
                        if !vinyl_mode {
                            if let Some(ref mut d) = decks[0] {
                                d.metronome_mode = !d.metronome_mode;
                                d.last_metro_beat = if d.metronome_mode {
                                    let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
                                    let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                                        - d.tempo.offset_ms as i128 * 1_000_000;
                                    Some(ns.div_euclid(beat_period.as_nanos() as i128))
                                } else { None };
                            }
                        }
                    }
                    Some(Action::Deck2MetronomeToggle) => {
                        if !vinyl_mode {
                            if let Some(ref mut d) = decks[1] {
                                d.metronome_mode = !d.metronome_mode;
                                d.last_metro_beat = if d.metronome_mode {
                                    let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
                                    let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
                                        - d.tempo.offset_ms as i128 * 1_000_000;
                                    Some(ns.div_euclid(beat_period.as_nanos() as i128))
                                } else { None };
                            }
                        }
                    }
                    Some(Action::Deck1RedetectBpm) => {
                        if !vinyl_mode {
                        if let Some(ref mut d) = decks[0] {
                            if d.tempo.pending_bpm.is_some() {
                                d.tempo.pending_bpm = None;
                            } else if d.tempo.redetecting {
                                let (_, dead_rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                d.tempo.background_rx = Some(std::mem::replace(&mut d.tempo.bpm_rx, dead_rx));
                                d.tempo.redetecting = false;
                                d.tempo.analysis_hash = d.tempo.redetect_saved_hash.take();
                            } else if d.tempo.analysis_hash.is_some() {
                                if let Some(bg_rx) = d.tempo.background_rx.take() {
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.bpm_rx = bg_rx;
                                    d.tempo.redetecting = true;
                                } else {
                                    let mono_bg = Arc::clone(&d.audio.mono);
                                    let (tx, rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                    let hash_bg = d.tempo.analysis_hash.clone().unwrap_or_default();
                                    let sr_bg = d.audio.sample_rate;
                                    thread::spawn(move || {
                                        if let Ok(bpm) = detect_bpm(&mono_bg, sr_bg) {
                                            let _ = tx.send((hash_bg, bpm, 0, true));
                                        }
                                    });
                                    d.tempo.bpm_rx = rx;
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.redetecting = true;
                                }
                            }
                        }
                        }
                    }
                    Some(Action::Deck2RedetectBpm) => {
                        if !vinyl_mode {
                        if let Some(ref mut d) = decks[1] {
                            if d.tempo.pending_bpm.is_some() {
                                d.tempo.pending_bpm = None;
                            } else if d.tempo.redetecting {
                                let (_, dead_rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                d.tempo.background_rx = Some(std::mem::replace(&mut d.tempo.bpm_rx, dead_rx));
                                d.tempo.redetecting = false;
                                d.tempo.analysis_hash = d.tempo.redetect_saved_hash.take();
                            } else if d.tempo.analysis_hash.is_some() {
                                if let Some(bg_rx) = d.tempo.background_rx.take() {
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.bpm_rx = bg_rx;
                                    d.tempo.redetecting = true;
                                } else {
                                    let mono_bg = Arc::clone(&d.audio.mono);
                                    let (tx, rx) = mpsc::channel::<(String, f32, i64, bool)>();
                                    let hash_bg = d.tempo.analysis_hash.clone().unwrap_or_default();
                                    let sr_bg = d.audio.sample_rate;
                                    thread::spawn(move || {
                                        if let Ok(bpm) = detect_bpm(&mono_bg, sr_bg) {
                                            let _ = tx.send((hash_bg, bpm, 0, true));
                                        }
                                    });
                                    d.tempo.bpm_rx = rx;
                                    d.tempo.redetect_saved_hash = d.tempo.analysis_hash.take();
                                    d.tempo.redetecting = true;
                                }
                            }
                        }
                        }
                    }
                    Some(Action::Help)            => { help_open = true; }
                    Some(Action::TerminalRefresh)  => { let _ = terminal.clear(); }
                    Some(Action::VinylModeToggle) => {
                        vinyl_mode = !vinyl_mode;
                        for slot in 0..2 {
                            if let Some(ref mut d) = decks[slot] {
                                if vinyl_mode {
                                    // Entering vinyl mode: capture current speed as vinyl_speed;
                                    // clear tap state and stop metronome.
                                    d.tempo.vinyl_speed = d.tempo.bpm / d.tempo.base_bpm;
                                    d.audio.player.set_speed(d.tempo.vinyl_speed);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.vinyl_speed, 1.0);
                                    d.tap.tap_times.clear();
                                    d.tap.last_tap_wall = None;
                                    d.metronome_mode = false;
                                    d.last_metro_beat = None;
                                } else {
                                    // Leaving vinyl mode: convert vinyl_speed to BPM adjustment.
                                    d.tempo.bpm = (d.tempo.base_bpm * d.tempo.vinyl_speed).clamp(40.0, 240.0);
                                    d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                    shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
                                    anchor_beat_grid_to_cue(d);
                                }
                            }
                        }
                        cache.set_vinyl_mode(vinyl_mode);
                        cache.save();
                    }
                    Some(Action::LatencyDecrease)  => {
                        audio_latency_ms = (audio_latency_ms - 10).max(0);
                        cache.set_latency(audio_latency_ms);
                        cache.save();
                    }
                    Some(Action::LatencyIncrease)  => {
                        audio_latency_ms = (audio_latency_ms + 10).min(250);
                        cache.set_latency(audio_latency_ms);
                        cache.save();
                    }
                    Some(Action::Deck1FilterIncrease) => { if let Some(ref mut d) = decks[0] { d.filter_offset = (d.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterDecrease) => { if let Some(ref mut d) = decks[0] { d.filter_offset = (d.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterReset)    => { if let Some(ref mut d) = decks[0] { d.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterIncrease) => { if let Some(ref mut d) = decks[1] { d.filter_offset = (d.filter_offset + 1).min(16);  d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterDecrease) => { if let Some(ref mut d) = decks[1] { d.filter_offset = (d.filter_offset - 1).max(-16); d.audio.filter_offset_shared.store(d.filter_offset, Ordering::Relaxed); } }
                    Some(Action::Deck2FilterReset)    => { if let Some(ref mut d) = decks[1] { d.filter_offset = 0; d.audio.filter_offset_shared.store(0, Ordering::Relaxed); } }
                    Some(Action::Deck1FilterSlopeIncrease) => { if let Some(ref mut d) = decks[0] { if d.filter_poles < 4 { d.filter_poles += 2; d.audio.filter_poles.store(d.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck1FilterSlopeDecrease) => { if let Some(ref mut d) = decks[0] { if d.filter_poles > 2 { d.filter_poles -= 2; d.audio.filter_poles.store(d.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck2FilterSlopeIncrease) => { if let Some(ref mut d) = decks[1] { if d.filter_poles < 4 { d.filter_poles += 2; d.audio.filter_poles.store(d.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::Deck2FilterSlopeDecrease) => { if let Some(ref mut d) = decks[1] { if d.filter_poles > 2 { d.filter_poles -= 2; d.audio.filter_poles.store(d.filter_poles, Ordering::Relaxed); } } }
                    Some(Action::PaletteCycle) => {
                        scheme_idx = (scheme_idx + 1) % PALETTE_SCHEMES.len();
                        for slot in 0..2 {
                            if let Some(ref mut d) = decks[slot] {
                                d.display.palette = if slot == 0 { PALETTE_SCHEMES[scheme_idx].1 } else { PALETTE_SCHEMES[scheme_idx].2 };
                            }
                        }
                    }
                    Some(Action::ArtCycle) => {
                        art_bright_idx = [2u8, 0, 1][art_bright_idx as usize]; // dim→bright→off→dim
                        cache.set_art_bright_idx(art_bright_idx);
                        cache.save();
                    }
                    Some(Action::Deck1OffsetIncrease) => {
                        if !vinyl_mode { if let Some(ref mut d) = decks[0] { apply_offset_step(d, 10); } }
                    }
                    Some(Action::Deck1OffsetDecrease) => {
                        if !vinyl_mode { if let Some(ref mut d) = decks[0] { apply_offset_step(d, -10); } }
                    }
                    Some(Action::Deck2OffsetIncrease) => {
                        if !vinyl_mode { if let Some(ref mut d) = decks[1] { apply_offset_step(d, 10); } }
                    }
                    Some(Action::Deck2OffsetDecrease) => {
                        if !vinyl_mode { if let Some(ref mut d) = decks[1] { apply_offset_step(d, -10); } }
                    }
                    Some(Action::ZoomOut) => { if zoom_idx > 0 { zoom_idx -= 1; } }
                    Some(Action::ZoomIn)  => { if zoom_idx + 1 < ZOOM_LEVELS.len() { zoom_idx += 1; } }
                    Some(Action::HeightDecrease) => { if detail_height > DET_MIN as usize { detail_height -= 1; } }
                    Some(Action::HeightIncrease) => { if detail_height < max_det_h { detail_height += 1; } }
                    Some(Action::Deck1BpmIncrease) => {
                        if let Some(ref mut d) = decks[0] {
                            if vinyl_mode || !d.tempo.bpm_established {
                                d.tempo.vinyl_speed = (d.tempo.vinyl_speed + 0.001).clamp(0.1, 4.0);
                                d.audio.player.set_speed(d.tempo.vinyl_speed);
                                shared_renderer.store_speed_ratio(0, d.tempo.vinyl_speed, 1.0);
                            } else {
                                d.tempo.bpm = (d.tempo.bpm + 0.1).min(240.0);
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                                anchor_beat_grid_to_cue(d);
                            }
                        }
                    }
                    Some(Action::Deck1BpmDecrease) => {
                        if let Some(ref mut d) = decks[0] {
                            if vinyl_mode || !d.tempo.bpm_established {
                                d.tempo.vinyl_speed = (d.tempo.vinyl_speed - 0.001).clamp(0.1, 4.0);
                                d.audio.player.set_speed(d.tempo.vinyl_speed);
                                shared_renderer.store_speed_ratio(0, d.tempo.vinyl_speed, 1.0);
                            } else {
                                d.tempo.bpm = (d.tempo.bpm - 0.1).max(40.0);
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                                anchor_beat_grid_to_cue(d);
                            }
                        }
                    }
                    Some(Action::Deck2BpmIncrease) => {
                        if let Some(ref mut d) = decks[1] {
                            if vinyl_mode || !d.tempo.bpm_established {
                                d.tempo.vinyl_speed = (d.tempo.vinyl_speed + 0.001).clamp(0.1, 4.0);
                                d.audio.player.set_speed(d.tempo.vinyl_speed);
                                shared_renderer.store_speed_ratio(1, d.tempo.vinyl_speed, 1.0);
                            } else {
                                d.tempo.bpm = (d.tempo.bpm + 0.1).min(240.0);
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                                anchor_beat_grid_to_cue(d);
                            }
                        }
                    }
                    Some(Action::Deck2BpmDecrease) => {
                        if let Some(ref mut d) = decks[1] {
                            if vinyl_mode || !d.tempo.bpm_established {
                                d.tempo.vinyl_speed = (d.tempo.vinyl_speed - 0.001).clamp(0.1, 4.0);
                                d.audio.player.set_speed(d.tempo.vinyl_speed);
                                shared_renderer.store_speed_ratio(1, d.tempo.vinyl_speed, 1.0);
                            } else {
                                d.tempo.bpm = (d.tempo.bpm - 0.1).max(40.0);
                                d.tempo.bpm_established = true;
                                d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                                anchor_beat_grid_to_cue(d);
                            }
                        }
                    }
                    Some(Action::Deck1JumpForward4b)   => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  8.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  16); } } }
                    Some(Action::Deck1JumpBackward4b)  => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -8.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -16); } } }
                    Some(Action::Deck1JumpForward8b)   => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, 16.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  32); } } }
                    Some(Action::Deck1JumpBackward8b)  => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,-16.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -32); } } }
                    Some(Action::Deck1JumpForward1bt)  => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  0.5); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  1); } } }
                    Some(Action::Deck1JumpBackward1bt) => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -0.5); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -1); } } }
                    Some(Action::Deck1JumpForward4bt)  => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  2.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  4); } } }
                    Some(Action::Deck1JumpBackward4bt) => { if let Some(ref d) = decks[0] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -2.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -4); } } }
                    Some(Action::Deck2JumpForward4b)   => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  8.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  16); } } }
                    Some(Action::Deck2JumpBackward4b)  => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -8.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -16); } } }
                    Some(Action::Deck2JumpForward8b)   => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, 16.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  32); } } }
                    Some(Action::Deck2JumpBackward8b)  => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,-16.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -32); } } }
                    Some(Action::Deck2JumpForward1bt)  => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  0.5); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  1); } } }
                    Some(Action::Deck2JumpBackward1bt) => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -0.5); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -1); } } }
                    Some(Action::Deck2JumpForward4bt)  => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration,  2.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration,  4); } } }
                    Some(Action::Deck2JumpBackward4bt) => { if let Some(ref d) = decks[1] { if vinyl_mode { do_time_jump(&d.audio.seek_handle, &d.audio.player, d.total_duration, -2.0); } else { deck::do_jump(&d.audio.seek_handle, &d.audio.player, d.tempo.base_bpm, d.total_duration, -4); } } }
                    Some(Action::Deck1TempoReset) => {
                        if let Some(ref mut d) = decks[0] {
                            d.tempo.vinyl_speed = 1.0;
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.audio.player.set_speed(1.0);
                            if vinyl_mode {
                                shared_renderer.store_speed_ratio(0, 1.0, 1.0);
                            } else {
                                shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                            }
                        }
                    }
                    Some(Action::Deck2TempoReset) => {
                        if let Some(ref mut d) = decks[1] {
                            d.tempo.vinyl_speed = 1.0;
                            d.tempo.bpm = d.tempo.base_bpm;
                            d.audio.player.set_speed(1.0);
                            if vinyl_mode {
                                shared_renderer.store_speed_ratio(1, 1.0, 1.0);
                            } else {
                                shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                            }
                        }
                    }
                    Some(Action::Deck1BpmTap) => {
                        if !vinyl_mode {
                            if let Some(ref mut d) = decks[0] {
                                if !d.audio.player.is_paused() {
                                    let now = Instant::now();
                                    if let Some(last) = d.tap.last_tap_wall {
                                        if now.duration_since(last).as_secs_f64() > 2.0 { d.tap.tap_times.clear(); }
                                    }
                                    let display_samp = render[0].as_ref().map_or(d.display.smooth_display_samp, |rs| rs.display_samp);
                                    d.tap.tap_times.push(display_samp / d.audio.sample_rate as f64);
                                    d.tap.last_tap_wall = Some(now);
                                    if d.tap.tap_times.len() >= 8 {
                                        let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
                                        let tapped_offset = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
                                        let speed_ratio = d.tempo.bpm / d.tempo.base_bpm;
                                        d.tempo.base_bpm = tapped_bpm;
                                        d.tempo.bpm = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
                                        d.tempo.offset_ms = tapped_offset;
                                        d.tempo.bpm_established = true;
                                        d.tempo.offset_established = true;
                                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                        shared_renderer.store_speed_ratio(0, d.tempo.bpm, d.tempo.base_bpm);
                                    }
                                }
                            }
                        }
                    }
                    Some(Action::Deck2BpmTap) => {
                        if !vinyl_mode {
                            if let Some(ref mut d) = decks[1] {
                                if !d.audio.player.is_paused() {
                                    let now = Instant::now();
                                    if let Some(last) = d.tap.last_tap_wall {
                                        if now.duration_since(last).as_secs_f64() > 2.0 { d.tap.tap_times.clear(); }
                                    }
                                    let display_samp = render[1].as_ref().map_or(d.display.smooth_display_samp, |rs| rs.display_samp);
                                    d.tap.tap_times.push(display_samp / d.audio.sample_rate as f64);
                                    d.tap.last_tap_wall = Some(now);
                                    if d.tap.tap_times.len() >= 8 {
                                        let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
                                        let tapped_offset = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
                                        let speed_ratio = d.tempo.bpm / d.tempo.base_bpm;
                                        d.tempo.base_bpm = tapped_bpm;
                                        d.tempo.bpm = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
                                        d.tempo.offset_ms = tapped_offset;
                                        d.tempo.bpm_established = true;
                                        d.tempo.offset_established = true;
                                        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
                                        shared_renderer.store_speed_ratio(1, d.tempo.bpm, d.tempo.base_bpm);
                                    }
                                }
                            }
                        }
                    }
                    Some(Action::Deck1Cue) => {
                        if let Some(ref mut d) = decks[0] {
                            if d.audio.player.is_paused() {
                                let raw_samp = d.display.smooth_display_samp as usize;
                                d.cue_sample = Some(raw_samp);
                                anchor_beat_grid_to_cue(d);
                                if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                    cache.set(hash.clone(), cache_entry_for_deck(d));
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck2Cue) => {
                        if let Some(ref mut d) = decks[1] {
                            if d.audio.player.is_paused() {
                                let raw_samp = d.display.smooth_display_samp as usize;
                                d.cue_sample = Some(raw_samp);
                                anchor_beat_grid_to_cue(d);
                                if let Some(ref hash) = d.tempo.analysis_hash.clone() {
                                    cache.set(hash.clone(), cache_entry_for_deck(d));
                                    cache.save();
                                }
                            }
                        }
                    }
                    Some(Action::Deck1CuePlay) => {
                        if let Some(ref mut d) = decks[0] {
                            if let Some(cue_samp) = d.cue_sample {
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                    d.display.smooth_display_samp = cue_samp as f64;
                                } else {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    let target_samp = (cue_samp + latency_samps).min(d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize);
                                    d.audio.seek_handle.seek_to(target_samp as f64 / d.audio.sample_rate as f64);
                                }
                            }
                        }
                    }
                    Some(Action::Deck2CuePlay) => {
                        if let Some(ref mut d) = decks[1] {
                            if let Some(cue_samp) = d.cue_sample {
                                if d.audio.player.is_paused() {
                                    d.audio.seek_handle.seek_direct(cue_samp as f64 / d.audio.sample_rate as f64);
                                    d.display.smooth_display_samp = cue_samp as f64;
                                } else {
                                    let latency_samps = (audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0) as usize;
                                    let target_samp = (cue_samp + latency_samps).min(d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize);
                                    d.audio.seek_handle.seek_to(target_samp as f64 / d.audio.sample_rate as f64);
                                }
                            }
                        }
                    }
                    Some(Action::Deck1NudgeBackward) | Some(Action::Deck1NudgeForward)
                    | Some(Action::Deck2NudgeBackward) | Some(Action::Deck2NudgeForward)
                    | Some(Action::NudgeModeToggle)
                    | Some(Action::Deck1BaseBpmIncrease) | Some(Action::Deck1BaseBpmDecrease)
                    | Some(Action::Deck2BaseBpmIncrease) | Some(Action::Deck2BaseBpmDecrease) => {}
                    None => {}
                    }
                } // end if Press
            }
            _ => {}
            }
        }

        thread::sleep(frame_dur.saturating_sub(frame_start.elapsed()));
    }
}

fn service_deck_frame(
    slot: usize,
    decks: &mut [Option<Deck>; 2],
    col_secs: f64,
    frame_dur: Duration,
    elapsed: f64,
    mixer: &rodio::mixer::Mixer,
    shared_renderer: &SharedDetailRenderer,
    cache: &mut Cache,
    audio_latency_ms: i64,
    vinyl_mode: bool,
) {
    let Some(ref mut d) = decks[slot] else { return; };

    shared_renderer.store_gain(slot, f32::from_bits(d.audio.gain_linear.load(Ordering::Relaxed)));

    // Auto-reject pending BPM confirmation after 15 seconds.
    if let Some((_, _, _, received_at)) = &d.tempo.pending_bpm {
        if received_at.elapsed().as_secs() >= 15 {
            d.tempo.pending_bpm = None;
        }
    }

    // Expire per-deck active notification.
    if d.active_notification.as_ref().map_or(false, |n| Instant::now() >= n.expires) {
        d.active_notification = None;
    }

    // Poll BPM detection results.
    if let Ok((hash, new_bpm, new_offset, is_fresh)) = d.tempo.bpm_rx.try_recv() {
        if !is_fresh || !d.tempo.bpm_established {
            d.tempo.bpm      = new_bpm;
            d.tempo.base_bpm = new_bpm;
            shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
            d.tempo.offset_ms = (new_offset as f64 / 10.0).round() as i64 * 10;
            // Restore cue_sample, offset_established, and gain_db from cache if present.
            d.cue_sample = cache.get(hash.as_str()).and_then(|e| e.cue_sample);
            d.tempo.offset_established = cache.get(hash.as_str()).map_or(false, |e| e.offset_established);
            d.gain_db = cache.get(hash.as_str()).map_or(0, |e| e.gain_db);
            d.audio.gain_linear.store(10f32.powf(d.gain_db as f32 / 20.0).to_bits(), Ordering::Relaxed);
            if !vinyl_mode {
                if let Some(cue_samp) = d.cue_sample {
                    let cue_secs = cue_samp as f64 / d.audio.sample_rate as f64;
                    d.audio.seek_handle.seek_direct(cue_secs);
                }
            }
            cache.set(hash.clone(), cache_entry_for_deck(d));
            cache.save();
            d.tempo.analysis_hash      = Some(hash);
            if !is_fresh || d.tempo.redetecting { d.tempo.bpm_established = true; }
            d.tempo.redetecting        = false;
            d.tempo.redetect_saved_hash = None;
            d.tempo.background_rx      = None;
        } else {
            d.tempo.analysis_hash      = Some(hash.clone());
            d.tempo.redetecting        = false;
            d.tempo.redetect_saved_hash = None;
            d.tempo.background_rx      = None;
            d.tempo.pending_bpm        = Some((hash, new_bpm, new_offset, Instant::now()));
        }
    }

    // Real audio position.
    let pos_raw  = d.audio.seek_handle.position.load(Ordering::Relaxed);
    let pos_samp = pos_raw / d.audio.seek_handle.channels as usize;
    let total_mono_samps = d.audio.seek_handle.samples.len() / d.audio.seek_handle.channels as usize;

    // End-of-track: pause and reset to cue point if set, otherwise start.
    let at_end = pos_samp >= total_mono_samps;
    if at_end && !d.audio.player.is_paused() {
        d.audio.player.pause();
        let (reset_secs, reset_samp) = d.cue_sample
            .map(|s| (s as f64 / d.audio.sample_rate as f64, s as f64))
            .unwrap_or((0.0, 0.0));
        d.audio.seek_handle.seek_direct(reset_secs);
        d.display.smooth_display_samp = reset_samp;
        return;
    }

    // Advance smooth display position.
    if !d.audio.player.is_paused() {
        // Include warp-nudge speed factor so the display tracks the audio speed exactly.
        let base_speed = if vinyl_mode { d.tempo.vinyl_speed as f64 } else { (d.tempo.bpm / d.tempo.base_bpm) as f64 };
        let speed = base_speed * (1.0 + d.nudge as f64 * 0.1);
        // Use nominal frame duration rather than measured elapsed to avoid systematic drift:
        // thread::sleep overshoots, so elapsed is consistently larger than frame_dur.
        d.display.smooth_display_samp += frame_dur.as_secs_f64() * d.audio.sample_rate as f64 * speed;
    } else if d.nudge != 0 {
        // Paused with warp nudge: drift display and sync actual audio position for scrubbing.
        d.display.smooth_display_samp = (d.display.smooth_display_samp
            + elapsed * d.audio.sample_rate as f64 * d.nudge as f64 * 0.1)
            .clamp(0.0, total_mono_samps as f64);
        d.audio.seek_handle.set_position(d.display.smooth_display_samp / d.audio.sample_rate as f64);
        // Fire a scrub snippet once per half-column advance.
        let scrub_spc = if slot == 0 {
            shared_renderer.shared_a.lock().unwrap().samples_per_col
        } else {
            shared_renderer.shared_b.lock().unwrap().samples_per_col
        };
        let half_samples_per_col = (scrub_spc / 2).max(1);
        if scrub_spc > 0
            && (d.display.smooth_display_samp - d.display.last_scrub_samp).abs() >= half_samples_per_col as f64
        {
            scrub_audio(mixer, &d.audio.seek_handle.samples, d.audio.seek_handle.channels as u16,
                        d.audio.sample_rate, d.display.smooth_display_samp as usize, half_samples_per_col);
            d.display.last_scrub_samp = d.display.smooth_display_samp;
        }
    }

    // Drift correction.
    let drift = d.display.smooth_display_samp - pos_samp as f64;
    let large_drift = drift.abs() > d.audio.sample_rate as f64 * 0.5;
    let paused_snap  = d.audio.player.is_paused() && d.nudge == 0 && drift.abs() > 1.0;
    if large_drift || paused_snap {
        // Snap to nearest half-column so sub_col is stable after seeks.
        let speed = if vinyl_mode { d.tempo.vinyl_speed as f64 } else { (d.tempo.bpm / d.tempo.base_bpm) as f64 };
        let col_samp_f64 = col_secs * d.audio.sample_rate as f64 * speed;
        let half_col = col_samp_f64 / 2.0;
        d.display.smooth_display_samp = if half_col > 0.0 {
            (pos_samp as f64 / half_col).round() * half_col
        } else {
            pos_samp as f64
        };
    } else if !d.audio.player.is_paused() {
        d.display.smooth_display_samp -= drift * 0.05;
    }

    // Metronome: fire from buffer write position so the click arrives at the speaker on the beat.
    let beat_period = Duration::from_secs_f64(60.0 / d.tempo.base_bpm as f64);
    let metro_beat_index = {
        let ns = (d.display.smooth_display_samp / d.audio.sample_rate as f64 * 1_000_000_000.0) as i128
            - d.tempo.offset_ms as i128 * 1_000_000;
        ns.div_euclid(beat_period.as_nanos() as i128)
    };
    if d.metronome_mode && !d.audio.player.is_paused() {
        if d.last_metro_beat != Some(metro_beat_index) {
            play_click_tone(mixer, d.audio.sample_rate);
            d.last_metro_beat = Some(metro_beat_index);
        }
    } else {
        d.last_metro_beat = None;
    }

    // Tap session timeout: finalise BPM when the user stops tapping.
    let tap_active_now = !d.tap.tap_times.is_empty()
        && d.tap.last_tap_wall.map_or(false, |t| t.elapsed().as_secs_f64() < 2.0);
    if d.tap.was_tap_active && !tap_active_now && d.tap.tap_times.len() >= 8 {
        let (tapped_bpm, tapped_offset_raw) = compute_tap_bpm_offset(&d.tap.tap_times);
        let tapped_offset  = (tapped_offset_raw as f64 / 10.0).round() as i64 * 10;
        let speed_ratio    = d.tempo.bpm / d.tempo.base_bpm;
        d.tempo.base_bpm   = tapped_bpm;
        d.tempo.bpm        = (d.tempo.base_bpm * speed_ratio).clamp(40.0, 240.0);
        d.tempo.offset_ms  = tapped_offset;
        d.tempo.bpm_established = true;
        d.tempo.offset_established = true;
        d.audio.player.set_speed(d.tempo.bpm / d.tempo.base_bpm);
        shared_renderer.store_speed_ratio(slot, d.tempo.bpm, d.tempo.base_bpm);
        if let Some(ref hash) = d.tempo.analysis_hash {
            cache.set(hash.clone(), cache_entry_for_deck(d));
            cache.save();
        }
    }
    d.tap.was_tap_active = tap_active_now;

    // Spectrum analyser: chars every half beat, background glow every 8 beats.
    let analysing   = d.tempo.analysis_hash.is_none();
    let half_period = if analysing { Duration::from_millis(250) } else { beat_period / 4 };
    let bar_period  = beat_period * 8;
    let chars_due   = d.spectrum.last_update.map_or(true,    |t| t.elapsed() >= half_period);
    let bg_due      = d.spectrum.last_bg_update.map_or(true, |t| t.elapsed() >= bar_period);
    if chars_due || bg_due {
        let latency_correction = if d.audio.player.is_paused() { 0.0 } else { audio_latency_ms as f64 * d.audio.sample_rate as f64 / 1000.0 };
        let display_pos_samp = (d.display.smooth_display_samp - latency_correction).max(0.0) as usize;
        let (new_chars, new_bg) = compute_spectrum(&d.audio.mono, display_pos_samp, d.audio.sample_rate, d.filter_offset);
        if chars_due {
            d.spectrum.chars = new_chars;
            for i in 0..16 { d.spectrum.bg_accum[i] |= new_bg[i]; }
            d.spectrum.bg = d.spectrum.bg_accum;
            d.spectrum.last_update = Some(Instant::now());
        }
        if bg_due {
            d.spectrum.bg_accum = [false; 16];
            d.spectrum.last_bg_update = Some(Instant::now());
        }
    }
}
