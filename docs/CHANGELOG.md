# Changes

## 0.1.1 — 2026-10-26

- Find multicolored frame arcs among ordinary screen text and gray UI, instead of bounding every dark pixel in the camera crop. Bridge narrow desaturated seams for connectivity while fitting corners from original color pixels.
- Keep full frame/payload validation and reject distinct valid symbols across located candidates. Reject scenes exceeding the candidate budget.
- Match camera sampling to the visible guide in rectangular as well as square previews.
- Move scan result/status beside the preview on phones; show scanner version in the footer.
- Version both WASM-loader and WASM-binary URLs so a refreshed page cannot silently reuse the previous scanner core from HTTP cache.
- Add screen-background, rotation, blur, negative-frame, ambiguity and capture-geometry regressions. One locally supplied camera-preview screenshot now decodes 123456789 without manual corners; continuous phone scanning remains pending.

Protocol V1 and existing codewords/rendered samples are unchanged.
