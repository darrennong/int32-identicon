use int32_identicon_core::{self as core, scan, Error};

// Deterministic neutral test scene, not a copy of a user's camera screenshot.
// Includes screen chrome, text-like dark strokes and a single-color blue icon.
fn screen_scene(value: i32, foreground: u8, angle: f64, damage: bool) -> (usize, usize, Vec<u8>) {
    let (w, h) = (640, 480);
    let mut g = core::grid(value, Some(foreground)).unwrap();
    if damage {
        for bit in 0..16 {
            let y = bit / 6 + 2;
            let x = bit % 6 + 2;
            g[y][x] = if g[y][x] == 3 { foreground } else { 3 };
            g[y][15 - x] = g[y][x];
        }
    }
    let mut image = vec![255; w * h * 4];
    let (s, c) = angle.to_radians().sin_cos();
    let mut seed = 901_u32;
    for y in 0..h {
        for x in 0..w {
            let mut color = [210, 212, 198];
            if !(25..=615).contains(&x) || (y > 34 && y < 37) {
                color = [80, 83, 81];
            }
            if (38..180).contains(&x) && (y % 23 < 3) && (100..400).contains(&y) {
                color = [115, 120, 114];
            }
            if (x as i32 - 570).pow(2) + (y as i32 - 90).pow(2) < 15 * 15 {
                color = [25, 125, 220];
            }
            let dx = x as f64 - 338.0;
            let dy = y as f64 - 244.0;
            let u = (c * dx + s * dy) / 192.0 + 0.5;
            let v = (-s * dx + c * dy) / 192.0 + 0.5;
            if (-0.125..1.125).contains(&u) && (-0.125..1.125).contains(&v) {
                color = [255; 3];
            }
            if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                color = core::PALETTE[g[(v * 16.0) as usize][(u * 16.0) as usize] as usize];
            }
            for (ch, &value) in color.iter().enumerate() {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let lift = [55.0, 52.0, 49.0][ch];
                image[(y * w + x) * 4 + ch] =
                    (value as f64 * 0.70 + lift + (seed % 7) as f64 - 3.0).clamp(0.0, 255.0) as u8;
            }
        }
    }
    // Camera resampling can make complementary adjacent patches meet through
    // a neutral seam. Blur a copy so the production locator must bridge it.
    let source = image.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            for ch in 0..3 {
                let mut sum = 0_u32;
                for yy in y - 1..=y + 1 {
                    for xx in x - 1..=x + 1 {
                        sum += source[(yy * w + xx) * 4 + ch] as u32;
                    }
                }
                image[(y * w + x) * 4 + ch] = (sum / 9) as u8;
            }
        }
    }
    (w, h, image)
}

#[test]
fn locates_small_frame_among_gray_screen_chrome_and_text() {
    for angle in [-28.0, -8.0, 0.0, 17.0, 36.0, 90.0] {
        for fg in [0, 8, 12, 15] {
            let (w, h, pixels) = screen_scene(123456789, fg, angle, false);
            let decoded = scan::decode_image(w, h, &pixels)
                .unwrap_or_else(|e| panic!("angle {angle}, foreground {fg}: {e}"));
            assert_eq!(decoded.value, 123456789);
            assert_eq!(decoded.foreground, fg);
        }
    }
}

#[test]
fn locating_a_frame_does_not_bypass_payload_validation() {
    let (w, h, pixels) = screen_scene(123456789, 15, 0.0, true);
    assert!(scan::decode_image(w, h, &pixels).is_err());
}

#[test]
fn two_distinct_valid_symbols_are_ambiguous() {
    let (w, h) = (800, 360);
    let mut image = vec![255; w * h * 4];
    for pixel in image.as_chunks_mut::<4>().0 {
        pixel[..3].copy_from_slice(&[206, 209, 196]);
    }
    for (value, left) in [(123456789, 60), (-987654321, 510)] {
        let (size, pixels) = core::rgba(value, None, 10).unwrap();
        for y in 0..size {
            for x in 0..size {
                let src = (y * size + x) * 4;
                let dst = ((y + 70) * w + x + left) * 4;
                image[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
            }
        }
    }
    assert_eq!(scan::decode_image(w, h, &image), Err(Error::Ambiguous));
}

#[test]
fn random_non_symbol_frames_are_rejected() {
    let (w, h) = (160, 120);
    let mut seed = 0x193d7931_u32;
    for _ in 0..48 {
        let mut image = vec![255; w * h * 4];
        for pixel in image.as_chunks_mut::<4>().0 {
            for channel in pixel.iter_mut().take(3) {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                *channel = seed as u8;
            }
        }
        assert!(scan::decode_image(w, h, &image).is_err());
    }
}
