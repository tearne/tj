# Proposal: Code Review
**Status: Approved**

## Intent

Perform a thorough review of the codebase prior to a major structural change (multi-deck / DJ mixer).
The review covers:

- **Performance** (priority): identify hotspots, unnecessary allocations, redundant work in the audio/render path
- **Dead code**: remove unused functions, types, imports, and feature flags
- **Encapsulation / separation of concerns**: surface modules or structs that have grown too broad, and propose cleaner boundaries
- **Rust guidelines**: apply `STYLE.md` + `STYLE-RUST.md` (now active)

## Context

The follow-on change ("multi-deck") will duplicate the main playback components to create multiple independent decks (as in a DJ setup) plus a mixer for crossfading/transitioning between tracks. A clean, well-separated codebase will reduce the cost of that duplication significantly — tight coupling or shared mutable state that works for one deck will become a bottleneck or correctness hazard at two or more.

## Dependencies

- Multi-deck proposal (not yet open) is the primary consumer of this work.
