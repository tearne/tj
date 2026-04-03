# Render Bounds Crash

## Intent

The application crashes with an index-out-of-bounds panic in ratatui when the terminal is 39 rows tall. Something is attempting to write to row 39 of a 39-row buffer (valid indices 0–38). The crash needs to be reproduced, the offending render call identified, and the layout calculation corrected so no widget is ever positioned outside the available area.

```
index outside of buffer: the area is Rect { x: 0, y: 0, width: 127, height: 39 }
but index is (23, 39)
location: ratatui-core-0.1.0/src/buffer/buffer.rs:250
```

## Approach

## Plan

## Conclusion
