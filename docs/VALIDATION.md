# Validation / 验证记录

Initial implementation: **2026-10-26**. Executed software checks and pending physical acceptance are separated below.

## Executed software checks

- Rust: Int32 minimum / maximum / -1 / 0 / 1, all 9 allowed foregrounds, 100,000 deterministic pseudorandom round trips.
- Independent JS reference: 15 fixed vectors using syndrome-equation parity; Rust uses polynomial division.
- Structure: exact border arrays / corners, 52 white separator cells, payload left–right symmetry and border anti-diagonal symmetry.
- RS: all 2,295 nonzero single-symbol errors at 9 byte locations; all 36 erasure pairs for one fixture.
- Scanner: 8 D4 candidates for each foreground; mirror disagreement recovery; one erroneous byte in both halves; selected uncorrectable damage; bad border / separator; blank and malformed inputs.
- Synthetic image: projective warp, brightness gain 0.82, offset +18, deterministic ±4/channel noise; explicit-quad and automatic localization decode it. **This is synthetic, not a camera photo.**
- Ten actual 640×640 PNGs reopened and pixel-decoded; corresponding SVGs generated from the same grid. Independent codeword check and WASM re-decode of the real PNG files.
- Browser consensus unit tests: 3 distinct captures, repeated-frame rejection, uncertainty reset, gap reset and changed-result reset.
- Rust/WASM release build, TypeScript typecheck, production Vite build, JS reference and Node/WASM pixel checks.

Reproduce:

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run -p int32-identicon-cli -- examples examples
node scripts/reference.mjs
npm ci
npm run build
npm test
```

The Node/WASM test prints execution time for ten decodes as a development observation only. It is not a mobile performance benchmark.

## Hosted desktop browser checks

The first source commit `05835319f8b023b4a761b1829aabcd4bbe6a36f6` passed [Linux CI](https://github.com/darrennong/int32-identicon/actions/runs/40371746738) and [Pages deployment](https://github.com/darrennong/int32-identicon/actions/runs/40371747386).

On the HTTPS demo in desktop Chrome, the initial check executed:

- Int32 minimum and maximum generated and decoded from SVG-rasterized pixels through the real Worker/WASM path.
- Explicit blue foreground generated and decoded.
- Input `2147483648` rejected, export and self-check buttons disabled.
- Actual `01_-1072751911.png` selected through the file picker; result `-1072751911`, zero repaired bytes, explicitly marked single-frame.
- At a 390 CSS-pixel emulated viewport, the generator computed a single-column layout with no horizontal overflow. This is a layout check, **not a physical mobile browser test**.

No physical camera was used in these browser checks. Mobile compatibility and real screen capture remain pending.

## Limits

- Automatic location requires one symbol with a clean light surround. The extreme-point locator can reject clutter, multiple symbols, large rotation and hidden borders. It is not a general scene detector.
- Color reference uses per-channel black/white normalization, expected border checks and binary foreground classification. General camera color matrices, severe clipping, reflection and moiré are not yet modeled.
- Nine inset samples per cell reduce noise; they cannot recover data destroyed by defocus or motion blur.
- RS guarantees apply only within the byte correction budget. CRC-16 is not authentication or proof of zero false accepts.
- Static image import is single-frame; camera output requires temporal consensus. Physical mobile browser behavior remains unverified.
- No backend, video upload, analytics or offline service worker. The demo requires its GitHub Pages workflow to finish.

## Physical matrix — NOT RUN

Record exact OS/browser versions; test iPhone Safari and Android Chrome against both computer and phone screens. Every field below is pending, not an implied working range.

| Capture device / browser | Display | Symbol size / cell px | Distance | Tilt / rotation | Brightness / glare / color mode | Success / attempts | False accepts / negatives | Median / p95 latency | Status |
|---|---|---|---|---|---|---|---|---|---|
| iPhone / Safari — TBD | Computer — TBD | TBD | TBD | TBD | TBD | — | — | — | Not run |
| iPhone / Safari — TBD | Phone — TBD | TBD | TBD | TBD | TBD | — | — | — | Not run |
| Android / Chrome — TBD | Computer — TBD | TBD | TBD | TBD | TBD | — | — | — | Not run |
| Android / Chrome — TBD | Phone — TBD | TBD | TBD | TBD | TBD | — | — | — | Not run |

Cover edge integers and all foregrounds; malformed frames, reflected views, blur, partial borders, non-symbol scenes and dimming as stress/negative cases. Report attempt/frame counts; zero observed false accepts must state its finite sample size.

Failure records need expected integer, protocol/foreground, display size, capture settings, rejection reason and consented test crops without private backgrounds. Define measured usable conditions before claiming screen-scan acceptance. Do not pre-commit FPS, minimum size or scan distance.
