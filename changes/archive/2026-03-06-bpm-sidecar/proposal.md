# Proposal: BPM Sidecar
**Status: Approved**

## Intent
BPM detection on a full-length track takes several seconds on every launch. A sidecar file stores the result alongside the audio file so subsequent launches are instant.

## Specification Deltas

### ADDED
- On load, if a sidecar file exists for the audio file, BPM is read from it instead of being detected.
- If no sidecar exists, BPM is detected as normal and the result is written to a sidecar file.
- The sidecar also persists the user's last phase offset for that track.
- Sidecar files are named `<audio-filename>.tj` and placed in the same directory as the audio file.

## Scope
- **In scope**: sidecar read/write for BPM and phase offset.
- **Out of scope**: sidecar invalidation/versioning, storing other metadata.
