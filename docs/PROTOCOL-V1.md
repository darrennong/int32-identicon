# Int32 Identicon Protocol V1

**Wire format frozen; scanner implementation experimental.** The constants and mapping below are normative for version 1. Changing the palette, mask, ECC or layout requires a new wire version. Camera thresholds are implementation policy, not physical performance guarantees.

## Coordinates and rendering

All coordinates are zero-based `(row,column)`, `0..15`:

- Outer frame: row 0/15 or column 0/15, 60 distinct cells.
- White separator: boundary of rows/columns 1..14, 52 cells, all palette ID 3.
- Payload: rows/columns 2..13, 12×12 cells, two states: foreground / white.
- Independent cells: rows 2..13, columns 2..7, 72 bits. Right half is an exact left–right mirror: `grid[r][15-c] = grid[r][c]`.

No rounded corners, deleted cells or decorative overlays may alter the grid. Repository exports add a two-cell white quiet zone **outside** the 16×16 symbol (20×20 SVG viewBox). It carries no data. Retain it with the current automatic locator. Prefer lossless SVG/PNG, opaque sRGB colors and integer cell sizes.

## Normative palette

Color names are descriptions; use these exact sRGB HEX values:

| ID | Name / 名称 | HEX | Payload foreground |
|---|---|---|---|
| 0 | Black / 黑 | `#000000` | Yes |
| 1 | Dark gray / 深灰 | `#4B5563` | Yes |
| 2 | Light gray / 浅灰 | `#CBD5E1` | No |
| 3 | White / 白 | `#FFFFFF` | No |
| 4 | Red / 红 | `#B91C1C` | Yes |
| 5 | Orange / 橙 | `#F97316` | No |
| 6 | Yellow / 黄 | `#FACC15` | No |
| 7 | Yellow green / 黄绿 | `#84CC16` | No |
| 8 | Green / 绿 | `#15803D` | Yes |
| 9 | Teal / 青绿 | `#0F766E` | Yes |
| 10 | Cyan / 青 | `#06B6D4` | No |
| 11 | Sky blue / 天蓝 | `#38BDF8` | No |
| 12 | Blue / 蓝 | `#1D4ED8` | Yes |
| 13 | Purple / 紫 | `#6D28D9` | Yes |
| 14 | Magenta / 洋红 | `#A21CAF` | Yes |
| 15 | Brown / 棕 | `#78350F` | Yes |

Allowed foregrounds: `F=[0,1,4,8,9,12,13,14,15]`. Default: `F[(value as uint32) % 9]`, reinterpreting the Int32 bit pattern as unsigned. Explicit overrides MUST belong to F; the chosen ID is stored in the header. These colors exceed nominal 4.5:1 sRGB contrast against white; real camera distinguishability still requires measurement.

## Fixed border

```text
N = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]
T = [0,1,2,3,15,4,5,6,14,7,8,9,10,11,13,12]
for i = 0..15 inclusive:
    grid[0][i]  = N[i]
    grid[i][15] = N[15-i]
    grid[i][0]  = T[i]
    grid[15][i] = T[15-i]
```

Shared corners agree: TL=0, TR=15, BL=12, BR=0. A corner is one cell, not triangles. T is a fixed visual order, not Kelvin temperature. The frame has **TR–BL diagonal** symmetry; the payload has **left–right** symmetry. The border cannot resolve every reflected orientation by itself.

## Seven-byte message: 56 bits

| Byte | Meaning |
|---|---|
| 0, high nibble | Protocol version `0001` |
| 0, low nibble | Foreground ID |
| 1..4 | Int32 two's-complement, big-endian |
| 5..6 | CRC-16, big-endian |

CRC is **CRC-16/CCITT-FALSE**: width=16, poly=`0x1021`, init=`0xFFFF`, refin=false, refout=false, xorout=`0x0000`. Cover bytes **0..4 only**, including both header nibbles. ASCII `123456789` check=`0x29B1`. Parity and the visual mask are not part of CRC input.

## Reed–Solomon: 16 parity bits

Use shortened systematic **RS(9,7)** in GF(256), primitive polynomial `x^8+x^4+x^3+x^2+1` (`0x11D`), primitive element `alpha=0x02`, generator roots `alpha^0`, `alpha^1`:

```text
g(x) = (x+1)(x+2) = x^2 + 3x + 2
```

Message bytes are highest-degree-first coefficients of `m(x)`. Append the two-byte remainder of `m(x)*x^2 / g(x)`: coefficient of x, then constant. Nine bytes total: 7 unchanged message bytes + 2 parity bytes. The complete degree-at-most-8 polynomial satisfies `c(1)=c(2)=0`.

Minimum distance 3 guarantees correction of **one unknown erroneous byte OR up to two known erased bytes** (`2e+s<=2`). One byte is eight consecutive independent bits and can cross row boundaries. Two erasures leave no budget for another unknown error. Any partially unknown byte is a whole-byte erasure; do not guess its missing bits.

CRC MUST be checked after correction. Beyond-budget damage can alias another RS codeword; CRC reduces accidental acceptance but does not guarantee detection of every multi-byte corruption or provide authentication. There are **no reserved bits** remaining in V1.

## Mask and cell mapping: 72 bits

XOR each codeword byte with the corresponding fixed hexadecimal mask byte:

```text
MASK = [A5,3C,96,F0,69,5A,C3,87,1E]
```

Serialize nine resulting bytes in order, **most-significant bit first** within each byte. For bit `b=0..71`:

```text
row = 2 + floor(b/6)
col = 2 + (b%6)
grid[row][col] = foreground if bit[b]==1 else white
grid[row][15-col] = grid[row][col]
```

This fixed XOR mask is reversible, not encryption. Do not search for a prettier mask or modify cells. Decode by fusing mirror observations, undoing XOR, correcting RS, checking version / allowed foreground / CRC, then reading bytes 1..4 as signed big-endian Int32. The same integer/version/foreground always yields the same symbol.

## Acceptance

Check grid geometry, full border layout, separator whiteness, binary foreground fit and mirror agreement. Use references from the color frame / separator. Consider all eight D4 rotation/reflection candidates.

Each candidate independently passes ECC, version, allowed foreground and CRC. Collapse candidates with identical `(version,value,foreground)`. Distinct valid tuples mean **ambiguous: reject**; zero valid tuples mean reject. Never select the first plausible integer. Mirror disagreements become byte erasures, not an arbitrary choice of half.

The camera demo requires **three distinct consecutive captures** with equal tuples, increasing timestamps, no gap above 700 ms and a confirmation window at most 1500 ms. Failed or ambiguous frames reset confirmation. Static images are explicitly single-frame. These temporal thresholds may be tuned after device measurements without changing wire format.

## Conformance

[`v1-vectors.json`](../tests/fixtures/v1-vectors.json) covers Int32 minimum, maximum, -1, 0, 1 and ten deterministic random values. Random generator: xorshift32 with initial state `0x49333244`, shifts left 13, unsigned right 17, left 5, truncating to 32 bits.

The independent [JS reference](../scripts/reference.mjs) solves two syndrome equations for parity; Rust uses polynomial division. They must agree byte-for-byte. The [example manifest](../examples/manifest.json) records actual rendered images, values, foregrounds, CRC, codeword and masked bytes. Samples come from the encoder, not an image-generation model.
