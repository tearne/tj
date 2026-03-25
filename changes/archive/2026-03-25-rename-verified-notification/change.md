# Rename Verified Notification
**Type**: Proposal
**Status**: Implemented

## Log

- `src/deck/mod.rs`: added `Success` variant to `NotificationStyle`
- `src/render/mod.rs`: `Success` → `Color::Green`
- `src/main.rs`: `Success` → `Color::Green`; rename success path now calls `target_path.exists()` — deck state only updated and `Success` notification shown on verification; `Error` shown if verification fails

## Problem

After a successful `std::fs::rename`, the notification bar shows `→ new-stem` in `DarkGray` (the `Info` style). Two issues:

1. There is no post-rename verification — `std::fs::rename` returning `Ok(())` does not guarantee the file is readable at the new path (e.g. cross-device edge cases, filesystem quirks). We have no confirmation the operation truly completed.
2. The notification colour does not distinguish a successful rename from routine info messages, making it easy to miss.

## Proposed Fix

**Verification**: after `std::fs::rename` returns `Ok(())`, call `target_path.exists()`. If the check fails, show an `Error` notification (`"rename could not be verified"`) and do not update `d.path`, `d.filename`, or `d.track_name` — the deck state stays pointing at the original path.

**New style**: add a `Success` variant to `NotificationStyle`, rendered as `Color::Green` in both render sites (`src/render/mod.rs` and `src/main.rs`). Use `Success` for the rename notification when verification passes, so the green colour signals a confirmed on-disk change.

The tag-only save (`"tags saved"`) keeps `Info`/`DarkGray` — that path has no rename and no special confirmation need.
