//! Color-frame candidate location, projective sampling, and conservative
//! validation across every located symbol and every D4 orientation.
use crate::{decode, grid, Decoded, Error, FOREGROUNDS, MASK, PALETTE};

pub type Point = [f64; 2];
pub type Quad = [Point; 4]; // geometric TL, TR, BR, BL (clockwise)
type Color = [f64; 3];
pub type Samples = [[Color; 16]; 16];

fn distance(a: Color, b: Color) -> f64 {
    ((0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>() / 3.0).sqrt()
}
fn as_color(c: [u8; 3]) -> Color {
    c.map(|v| v as f64)
}
fn mapped(mut y: usize, mut x: usize, transform: usize) -> (usize, usize) {
    if transform >= 4 {
        x = 15 - x;
    }
    for _ in 0..transform % 4 {
        (y, x) = (x, 15 - y);
    }
    (y, x)
}

/// Validate all eight rotations/reflections and reject distinct valid payloads.
pub fn decode_samples(samples: &Samples) -> Result<Decoded, Error> {
    if samples
        .iter()
        .flatten()
        .flatten()
        .any(|v| !v.is_finite() || !(0.0..=255.0).contains(v))
    {
        return Err(Error::Image);
    }
    let expected = grid(0, None)?;
    let mut accepted: Option<Decoded> = None;
    for transform in 0..8 {
        let mut cells = [[[0.0; 3]; 16]; 16];
        for (y, row) in cells.iter_mut().enumerate() {
            for (x, color) in row.iter_mut().enumerate() {
                let (sy, sx) = mapped(y, x, transform);
                *color = samples[sy][sx];
            }
        }
        let mut white = [0.0; 3];
        for (i, row) in cells.iter().enumerate().take(15).skip(1) {
            for (j, color) in row.iter().enumerate().take(15).skip(1) {
                if i == 1 || j == 1 || i == 14 || j == 14 {
                    for (ch, w) in white.iter_mut().enumerate() {
                        *w += color[ch] / 52.0;
                    }
                }
            }
        }
        let black: Color = std::array::from_fn(|ch| (cells[0][0][ch] + cells[15][15][ch]) / 2.0);
        if (0..3).any(|ch| white[ch] < 110.0 || white[ch] - black[ch] < 75.0) {
            continue;
        }
        for row in &mut cells {
            for color in row {
                for ch in 0..3 {
                    color[ch] = (color[ch] - black[ch]) * 255.0 / (white[ch] - black[ch]);
                }
            }
        }
        let mut border_error = 0.0;
        let mut bad_border = 0;
        let mut bad_white = 0;
        for y in 0..16 {
            for x in 0..16 {
                if y == 0 || x == 0 || y == 15 || x == 15 {
                    let error = distance(cells[y][x], as_color(PALETTE[expected[y][x] as usize]));
                    border_error += error;
                    if error > 75.0 {
                        bad_border += 1;
                    }
                } else if (y == 1 || x == 1 || y == 14 || x == 14)
                    && distance(cells[y][x], [255.0; 3]) > 40.0
                {
                    bad_white += 1;
                }
            }
        }
        if border_error / 60.0 > 35.0 || bad_border > 4 || bad_white > 2 {
            continue;
        }
        for foreground in FOREGROUNDS {
            let fg = as_color(PALETTE[foreground as usize]);
            let classify = |c: Color| -> Option<bool> {
                let d0 = distance(c, [255.0; 3]);
                let d1 = distance(c, fg);
                if d0.min(d1) > 65.0 || (d0 - d1).abs() < 30.0 {
                    None
                } else {
                    Some(d1 < d0)
                }
            };
            let mut word = [0; 9];
            let mut erasures = Vec::new();
            let mut mismatches = 0;
            for bit in 0..72 {
                let y = bit / 6 + 2;
                let x = bit % 6 + 2;
                let left = classify(cells[y][x]);
                let right = classify(cells[y][15 - x]);
                let observed = match (left, right) {
                    (Some(a), Some(b)) if a == b => Some(a),
                    (Some(_), Some(_)) => {
                        mismatches += 1;
                        None
                    }
                    (Some(a), None) | (None, Some(a)) => Some(a),
                    _ => None,
                };
                match observed {
                    Some(true) => word[bit / 8] |= 0x80 >> (bit % 8),
                    Some(false) => (),
                    None => {
                        if !erasures.contains(&(bit / 8)) {
                            erasures.push(bit / 8);
                        }
                    }
                }
            }
            if mismatches > 8 || erasures.len() > 2 {
                continue;
            }
            for i in 0..9 {
                word[i] ^= MASK[i];
            }
            if let Ok(result) = decode(word, &erasures) {
                if result.foreground != foreground {
                    continue;
                }
                if let Some(previous) = accepted {
                    if previous.value != result.value || previous.foreground != result.foreground {
                        return Err(Error::Ambiguous);
                    }
                    if previous.repaired_bytes <= result.repaired_bytes {
                        continue;
                    }
                }
                accepted = Some(result);
            }
        }
    }
    accepted.ok_or(Error::NotFound)
}

fn validate_image(width: usize, height: usize, rgba: &[u8]) -> Result<(), Error> {
    if width < 32
        || height < 32
        || width > 4096
        || height > 4096
        || rgba.len() != width * height * 4
    {
        return Err(Error::Image);
    }
    Ok(())
}
fn pixel(rgba: &[u8], offset: usize) -> Color {
    let a = rgba[offset + 3] as f64 / 255.0;
    std::array::from_fn(|ch| rgba[offset + ch] as f64 * a + 255.0 * (1.0 - a))
}

/// Legacy clean-background proposal, also useful for exact generated images.
fn light_quad(width: usize, height: usize, rgba: &[u8]) -> Result<Quad, Error> {
    validate_image(width, height, rgba)?;
    let mut corners = [[0.0; 2]; 4];
    let mut scores = [f64::INFINITY; 4];
    let mut count = 0;
    for y in 0..height {
        for x in 0..width {
            let c = pixel(rgba, (y * width + x) * 4);
            if distance(c, [255.0; 3]) < 58.0 {
                continue;
            }
            count += 1;
            let x = x as f64;
            let y = y as f64;
            let candidates = [x + y, -x + y, -x - y, x - y];
            for i in 0..4 {
                if candidates[i] < scores[i] {
                    scores[i] = candidates[i];
                    corners[i] = [x, y];
                }
            }
        }
    }
    if count < 120 {
        return Err(Error::NotFound);
    }
    // Pixel centers at extreme samples approximate the outside boundary; the
    // subpixel error is small compared with the central cell sampling inset.
    Ok(corners)
}

/// Solve a projective unit square -> image quadrilateral transform.
fn homography(q: Quad) -> Result<[f64; 8], Error> {
    if q.iter().flatten().any(|v| !v.is_finite()) {
        return Err(Error::Image);
    }
    let mut area = 0.0;
    for i in 0..4 {
        let a = q[i];
        let b = q[(i + 1) % 4];
        let c = q[(i + 2) % 4];
        if (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) <= 0.0 {
            return Err(Error::Image);
        }
        area += a[0] * b[1] - b[0] * a[1];
    }
    if area < 1024.0 {
        return Err(Error::Quality);
    }
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut a = [[0.0; 9]; 8];
    for i in 0..4 {
        let [u, v] = uv[i];
        let [x, y] = q[i];
        a[2 * i] = [u, v, 1.0, 0.0, 0.0, 0.0, -x * u, -x * v, x];
        a[2 * i + 1] = [0.0, 0.0, 0.0, u, v, 1.0, -y * u, -y * v, y];
    }
    for col in 0..8 {
        let pivot = (col..8)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        a.swap(col, pivot);
        let d = a[col][col];
        if d.abs() < 1e-9 {
            return Err(Error::Image);
        }
        for value in a[col].iter_mut().skip(col) {
            *value /= d;
        }
        let pivot_row = a[col];
        for (i, row) in a.iter_mut().enumerate() {
            if i != col {
                let f = row[col];
                for (value, pivot) in row.iter_mut().zip(pivot_row).skip(col) {
                    *value -= f * pivot;
                }
            }
        }
    }
    Ok(std::array::from_fn(|i| a[i][8]))
}

pub fn decode_quad(width: usize, height: usize, rgba: &[u8], quad: Quad) -> Result<Decoded, Error> {
    validate_image(width, height, rgba)?;
    if quad
        .iter()
        .any(|&[x, y]| x < 0.0 || y < 0.0 || x >= width as f64 || y >= height as f64)
    {
        return Err(Error::Image);
    }
    let h = homography(quad)?;
    let mut samples = [[[0.0; 3]; 16]; 16];
    for (y, row) in samples.iter_mut().enumerate() {
        for (x, color) in row.iter_mut().enumerate() {
            let mut colors = [[0.0; 3]; 9];
            for sy in 0..3 {
                for sx in 0..3 {
                    let u = (x as f64 + 0.5 + (sx as f64 - 1.0) * 0.14) / 16.0;
                    let v = (y as f64 + 0.5 + (sy as f64 - 1.0) * 0.14) / 16.0;
                    let d = h[6] * u + h[7] * v + 1.0;
                    if d.abs() < 1e-9 {
                        return Err(Error::Image);
                    }
                    let px = (h[0] * u + h[1] * v + h[2]) / d;
                    let py = (h[3] * u + h[4] * v + h[5]) / d;
                    if px < 0.0 || py < 0.0 || px >= width as f64 || py >= height as f64 {
                        return Err(Error::Image);
                    }
                    colors[sy * 3 + sx] = pixel(
                        rgba,
                        (py.round().min((height - 1) as f64) as usize * width
                            + px.round().min((width - 1) as f64) as usize)
                            * 4,
                    );
                }
            }
            for (ch, value) in color.iter_mut().enumerate() {
                let mut channel = colors.map(|c| c[ch]);
                channel.sort_by(f64::total_cmp);
                *value = channel[4];
            }
        }
    }
    decode_samples(&samples)
}

pub fn decode_image(width: usize, height: usize, rgba: &[u8]) -> Result<Decoded, Error> {
    detect(width, height, rgba).map(|(decoded, _)| decoded)
}

/// Return the geometry of a uniquely validated payload, never the bounds of all
/// dark text or window chrome in a frame.
pub fn locate(width: usize, height: usize, rgba: &[u8]) -> Result<Quad, Error> {
    detect(width, height, rgba).map(|(_, quad)| quad)
}

fn detect(width: usize, height: usize, rgba: &[u8]) -> Result<(Decoded, Quad), Error> {
    validate_image(width, height, rgba)?;
    let mut candidates = crate::locator::candidates(width, height, rgba)?;
    if let Ok(quad) = light_quad(width, height, rgba) {
        candidates.push(quad);
    }
    let mut accepted: Option<(Decoded, Quad)> = None;
    for quad in candidates {
        match decode_quad(width, height, rgba, quad) {
            Ok(result) => {
                if let Some((previous, _)) = accepted {
                    if previous.value != result.value || previous.foreground != result.foreground {
                        return Err(Error::Ambiguous);
                    }
                    if previous.repaired_bytes <= result.repaired_bytes {
                        continue;
                    }
                }
                accepted = Some((result, quad));
            }
            Err(Error::Ambiguous) => return Err(Error::Ambiguous),
            Err(_) => (),
        }
    }
    accepted.ok_or(Error::NotFound)
}
