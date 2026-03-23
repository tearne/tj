# Panic Log
**Type**: Fix
**Status**: Draft

## Problem

When tj panics the terminal is left in raw mode and the panic message is lost — it
either scrolls off or is never visible. There is no post-mortem record.

## Proposal

Install a custom panic hook at startup (before any other initialisation) that writes
the panic message and location to a file before the process exits. The terminal teardown
hook should still run so the shell is left in a usable state.

**File path**: `~/.local/share/tj/panic.log` — alongside the existing `cache.json`,
so it is easy to find and not mixed into the working directory.

**Content**: timestamp, thread name, panic message, file/line location. No backtrace
by default (requires `RUST_BACKTRACE=1` rerun anyway).

**Implementation sketch**:

```rust
let default_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let msg = info.to_string();
    // write to ~/.local/share/tj/panic.log
    default_hook(info);
}));
```

The default hook is chained so existing behaviour (printing to stderr) is preserved.
The file write is best-effort — failure to write must not mask the original panic.

## Log
