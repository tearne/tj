# Proposal: Notification Bar
**Status: Draft**

## Intent

Repurpose the track name bar as a dual-purpose notification area. When a notification is active it temporarily replaces the track name; when idle it shows the track name as before. This gives transient messages a dedicated, readable home rather than overlapping info bar content.

## Specification Deltas

### MODIFIED

- **Track name bar** becomes the **notification bar**: it displays the track name by default, but is temporarily replaced by a notification message when one is active. Notifications are dismissed automatically after a fixed timeout (or immediately on user action where applicable).

- **BPM confirmation prompt** moves from the info bar right group to the notification bar. The info bar right group is no longer replaced during a pending confirmation; the prompt (detected BPM, accept/reject keys, countdown) is shown in the notification bar instead.

- **Config-created notice** (currently shown in the track name bar for 5 s) is formalised as a notification using the same mechanism.

### ADDED

- A notification has a message string, an optional style (default: dim; could be red for warnings), and an expiry. When multiple notifications are queued, the most recent takes precedence.

## Unresolved

- Should the countdown timer for BPM confirmation remain visible, or is the prompt alone sufficient?
- Are there other existing messages that should migrate to the notification bar?
