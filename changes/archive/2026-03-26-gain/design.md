# Gain Trim Design

## Data

**`CacheEntry`** (`src/cache/mod.rs`) — add `gain_db: i8` with `#[serde(default)]`. Zero-default means existing cache entries load as 0 dB.

**`Deck`** (`src/deck/mod.rs`) — add `gain_db: i8` field (range −12..=12, default 0).

**`DeckAudio`** (`src/deck/mod.rs`) — add `gain_linear: Arc<AtomicU32>` (f32 bits, default `1.0f32.to_bits()`), following the same pattern as `deck_volume_atomic`.

## Signal Chain

**`FilterSource`** (`src/audio/mod.rs`) — add `gain: Arc<AtomicU32>` field. In `next()`, apply gain to `filtered` before the PFL branch:

```
let gain = f32::from_bits(self.gain.load(Ordering::Relaxed));
let gained = filtered * gain;
// PFL routing and fader volume applied to `gained`
```

## Actions and Keys

Four new actions in `Action` (`src/config/mod.rs`):

```
Deck1GainIncrease, Deck1GainDecrease,
Deck2GainIncrease, Deck2GainDecrease,
```

Default bindings in `config.toml` and `ACTION_MAP`:

```toml
deck1_gain_increase = "J"
deck1_gain_decrease = "M"
deck2_gain_increase = "K"
deck2_gain_decrease = "<"
```

## Action Handlers (`src/main.rs`)

On `Deck1GainIncrease` / `Deck1GainDecrease`:
1. Clamp `gain_db` to −12..=12.
2. Compute linear: `10f32.powf(gain_db as f32 / 20.0)`.
3. Store to `gain_linear` atomic.
4. Persist: update the cache entry for the current track hash and call `cache.save()`.

## Cache Integration

On track load (`build_deck`): read `cache.get(hash).and_then(|e| Some(e.gain_db)).unwrap_or(0)`, set `gain_db` and `gain_linear` accordingly.

On gain change: update the track's `CacheEntry::gain_db` and save, same as BPM/offset/cue.

## Display

In `info_line_for_deck` (`src/render/mod.rs`), append one character immediately after the closing bracket `▏` of the level indicator. No label, no extra spacing — sits flush.

Map `gain_db` (−12..=12) to index 0..=6:

```rust
const GAIN_CHARS: [char; 7] = ['▁','▂','▃','▄','▅','▆','▇'];
let idx = ((deck.gain_db + 12) * 6 / 24).clamp(0, 6) as usize;
// ▄ (index 3) at 0 dB; below = negative gain; above = positive
```

Colour: grey (`Color::Rgb(60, 60, 60)`) at 0 dB; dim amber (`Color::Rgb(100, 80, 0)`) when non-zero. No gradient — gain is a correction value, not a performance control.

## SPEC and Help Text Updates

- `SPEC/config.md` — add `J`/`M` and `K`/`<` to the per-deck tables.
- `SPEC/config.md` — add `J`/`M` (Sh row) to keyboard diagram for `j`/`m` keys; add `K`/`<` for `k`/`,`.
- `SPEC/deck.md` — add a Gain section describing the trim control, range, step, and cache persistence.
- `src/main.rs` help text — add a gain line.
