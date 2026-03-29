# Art Cache
**Type**: Fix
**Status**: Approved

## Log

`halfblock_art` decodes and resizes the cover art image on every frame (~20×/sec per deck). At ~5% idle CPU this is the likely cause. The fix is to cache the rendered `Vec<Line<'static>>` on the deck, keyed by panel dimensions and `art_bright_idx`, and only recompute when those change.
