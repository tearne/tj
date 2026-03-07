# Design: BPM Sidecar
**Status: Approved**

## Approach

Sidecar files are named `<audio-filename>.tj` and sit alongside the audio file. Format is JSON — two fields, human-readable, easy to hand-edit.

```json
{"bpm":120.0,"offset_ms":-30}
```

`serde` + `serde_json` are used for serialisation (serde_json is already a transitive dependency; add it explicitly).

### Load path
1. Derive sidecar path from audio path.
2. If sidecar exists and parses cleanly → use stored BPM and offset_ms, skip detection.
3. Otherwise → detect BPM as normal, write sidecar with `offset_ms: 0`.

### Save path
- On quit (`q` or `Esc`), write the current BPM and offset_ms to the sidecar. This persists any offset the user dialled in.

## Tasks
1. ✓ **Process**: Proposals created, both approved.
2. ✓ **Impl**: Add `serde` + `serde_json` dependencies; add `Sidecar` struct with load/save helpers.
3. ✓ **Impl**: Integrate into `main` — load sidecar before BPM detection; save on quit.
4. ✓ **Verify**: Second launch reads sidecar instantly; offset persisted on quit.
5. ✓ **Process**: Confirm ready to archive.
