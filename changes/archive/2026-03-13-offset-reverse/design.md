# Design: Reverse Tick Offset Direction
**Status: Approved**

## Approach

Swap the two default bindings in `resources/config.toml`. No code changes required — the `offset_increase` / `offset_decrease` actions and their handling are unchanged.

## Tasks

1. ✓ Impl: swap `offset_increase` and `offset_decrease` defaults in `config.toml`
2. ✓ Process: archive
