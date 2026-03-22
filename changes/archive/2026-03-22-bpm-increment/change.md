# Change: BPM Increment
**Type**: Fix

## Problem

Playback BPM (s/x, f/v keys) changes in increments of 0.01 instead of 0.1.

## Fix

Change the ±0.01 step to ±0.1 in all four `BpmIncrease`/`BpmDecrease` action handlers (deck 1 and deck 2). Update the help text to match.

## Log

- Changed Deck1BpmIncrease/Decrease and Deck2BpmIncrease/Decrease from ±0.01 to ±0.1.
