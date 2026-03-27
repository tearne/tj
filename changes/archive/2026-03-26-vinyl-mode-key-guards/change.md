# Vinyl Mode Key Guards
**Type**: Fix
**Status**: Approved

## Log
- Added `!vinyl_mode` guard to the base BPM ramp block (`Deck1/2BaseBpmIncrease/Decrease`)
- Added `!vinyl_mode` guard to all four offset handlers (`Deck1/2OffsetIncrease/Decrease`)
- Added `!vinyl_mode` guard to metronome toggle (`Deck1/2MetronomeToggle`)
- Added `!vinyl_mode` guard to BPM tap (`Deck1/2BpmTap`)
- On entry to vinyl mode: clear tap state and stop metronome on all loaded decks
- Overview click in vinyl mode: seek to proportional time position rather than snapping to nearest bar marker
