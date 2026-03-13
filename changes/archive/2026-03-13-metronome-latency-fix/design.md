# Design: Metronome Latency Fix
**Status: Approved**
*(retrospective)*

## Approach

`beat_index` was derived from `display_samp` (= `smooth_display_samp − latency`), which represents the speaker position. Firing when the speaker reaches the beat means the injected click travels through the audio pipeline and arrives one full `audio_latency_ms` late. A separate `metro_beat_index` is computed from `smooth_display_samp` (the buffer write position). When this transitions, the speaker is `audio_latency_ms` before the beat, so the click arrives on time.

The `MetronomeToggle` pre-set (added in `metronome-first-click`) was updated to use `metro_beat_index` for consistency.

## Tasks

1. ✓ Impl: compute `metro_beat_index` from `smooth_display_samp`; use it in the metronome block and `MetronomeToggle` pre-set
2. ✓ Process: archive
