# Architecture

## Threading

- Audio decode runs on a background thread with progress reported to the TUI via atomics; the TUI render loop starts immediately and remains responsive during decode.
- Hash computation and BPM detection run on a further background thread after decode, communicating results to the TUI via a channel.
- Audio playback runs on a dedicated thread.
- TUI rendering runs on a separate thread.
- State is shared between threads via lock-free or minimal-contention primitives to meet real-time rendering requirements.
