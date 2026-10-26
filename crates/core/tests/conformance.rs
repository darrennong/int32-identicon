use int32_identicon_core::{self as core, scan};

fn unhex(s: &str) -> [u8; 9] {
    std::array::from_fn(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
}

#[test]
fn independent_golden_vectors() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/v1-vectors.json")).unwrap();
    assert_eq!(core::crc16(b"123456789"), 0x29b1);
    for vector in fixture["vectors"].as_array().unwrap() {
        let value = vector["value"].as_i64().unwrap() as i32;
        let expected = unhex(vector["codewordHex"].as_str().unwrap());
        assert_eq!(core::encode(value, None).unwrap(), expected);
        assert_eq!(core::decode(expected, &[]).unwrap().value, value);
        assert_eq!(
            std::array::from_fn::<u8, 9, _>(|i| expected[i] ^ core::MASK[i]),
            unhex(vector["maskedHex"].as_str().unwrap())
        );
    }
}

#[test]
fn all_int32_edges_and_100000_random_round_trips() {
    let mut seed = 0x98765432_u32;
    for _ in 0..100_000 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let value = seed as i32;
        assert_eq!(
            core::decode(core::encode(value, None).unwrap(), &[])
                .unwrap()
                .value,
            value
        );
    }
    for v in [i32::MIN, i32::MAX, -1, 0, 1] {
        for fg in core::FOREGROUNDS {
            assert_eq!(
                core::decode(core::encode(v, Some(fg)).unwrap(), &[])
                    .unwrap()
                    .value,
                v
            );
        }
    }
    for invalid in [2, 3, 5, 6, 7, 10, 11, 16, 255] {
        assert!(core::encode(0, Some(invalid)).is_err());
    }
}

#[test]
fn exact_grid_structure_and_antidiagonal_border() {
    let g = core::grid(-123456789, None).unwrap();
    for i in 0..16 {
        assert_eq!(g[0][i], i as u8);
        assert_eq!(g[i][15], (15 - i) as u8);
        assert_eq!(g[i][0], core::WARM_COOL[i]);
        assert_eq!(g[15][i], core::WARM_COOL[15 - i]);
        assert_eq!(g[0][i], g[15 - i][15]);
    }
    let mut white_ring = 0;
    for (y, row) in g.iter().enumerate().take(15).skip(1) {
        for x in 1..15 {
            if y == 1 || x == 1 || y == 14 || x == 14 {
                assert_eq!(row[x], 3);
                white_ring += 1;
            } else {
                assert_eq!(g[y][x], g[y][15 - x]);
                assert!([3, core::default_foreground(-123456789)].contains(&g[y][x]));
            }
        }
    }
    assert_eq!(white_ring, 52);
}

fn samples(g: core::Grid) -> scan::Samples {
    std::array::from_fn(|y| {
        std::array::from_fn(|x| core::PALETTE[g[y][x] as usize].map(|c| c as f64))
    })
}

#[test]
fn all_colors_rotations_and_reflections() {
    for fg in core::FOREGROUNDS {
        let g = core::grid(-2147483648, Some(fg)).unwrap();
        for mirror in [false, true] {
            let mut rotated = g;
            if mirror {
                for row in &mut rotated {
                    row.reverse();
                }
            }
            for _ in 0..4 {
                assert_eq!(
                    scan::decode_samples(&samples(rotated)).unwrap().value,
                    i32::MIN
                );
                rotated = std::array::from_fn(|y| std::array::from_fn(|x| rotated[15 - x][y]));
            }
        }
    }
}

#[test]
fn mirrored_observations_correct_errors_and_reject_damage() {
    let original = core::grid(123456, Some(12)).unwrap();
    let mut one_side = original;
    for bit in 0..8 {
        let y = bit / 6 + 2;
        let x = bit % 6 + 2;
        one_side[y][x] = if one_side[y][x] == 3 { 12 } else { 3 };
    }
    assert_eq!(
        scan::decode_samples(&samples(one_side)).unwrap().value,
        123456
    );
    let mut both_sides = one_side;
    for bit in 0..8 {
        let y = bit / 6 + 2;
        let x = bit % 6 + 2;
        both_sides[y][15 - x] = both_sides[y][x];
    }
    let corrected = scan::decode_samples(&samples(both_sides)).unwrap();
    assert_eq!(corrected.value, 123456);
    assert_eq!(corrected.repaired_bytes, 1);
    for bit in 16..24 {
        let y = bit / 6 + 2;
        let x = bit % 6 + 2;
        both_sides[y][x] = if both_sides[y][x] == 3 { 12 } else { 3 };
        both_sides[y][15 - x] = both_sides[y][x];
    }
    assert!(scan::decode_samples(&samples(both_sides)).is_err());
    let mut bad_border = original;
    bad_border[0] = [3; 16];
    assert!(scan::decode_samples(&samples(bad_border)).is_err());
    let mut bad_white = original;
    bad_white[1] = [12; 16];
    assert!(scan::decode_samples(&samples(bad_white)).is_err());
    assert!(scan::decode_samples(&[[[255.0; 3]; 16]; 16]).is_err());
}

#[test]
fn png_equivalent_pixels_and_input_guards() {
    for value in [i32::MIN, i32::MAX, -1, 0, 1, 987654321] {
        let (size, pixels) = core::rgba(value, None, 8).unwrap();
        assert_eq!(
            scan::decode_image(size, size, &pixels).unwrap().value,
            value
        );
    }
    assert!(scan::decode_image(32, 32, &[]).is_err());
    assert!(scan::decode_image(5000, 5000, &[]).is_err());
    assert!(core::rgba(1, None, 0).is_err());
    let (s, p) = core::rgba(1, None, 8).unwrap();
    assert!(scan::decode_quad(s, s, &p, [[f64::NAN, 0.0]; 4]).is_err());
    assert!(scan::decode_quad(s, s, &p, [[0.0, 0.0]; 4]).is_err());
}

#[test]
fn perspective_gain_offset_and_noise() {
    let grid = core::grid(-901234567, Some(13)).unwrap();
    let (w, h) = (480, 400);
    let mut rgba = vec![255; w * h * 4];
    let mut seed = 42_u32;
    for y in 0..h {
        for x in 0..w {
            let (xx, yy) = (x as f64, y as f64);
            // Inverse of the independently chosen synthetic projective mapping.
            let b = 40.0 - 0.15 * xx;
            let d = 300.0 - 0.15 * yy;
            let u = ((xx - 80.0) * d - b * (yy - 55.0)) / (320.0 * d - 15.0 * b);
            let v = (320.0 * (yy - 55.0) - 15.0 * (xx - 80.0)) / (320.0 * d - 15.0 * b);
            let c = if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                core::PALETTE[grid[(v * 16.0) as usize][(u * 16.0) as usize] as usize]
            } else {
                [255; 3]
            };
            for ch in 0..3 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (seed % 9) as f64 - 4.0;
                rgba[(y * w + x) * 4 + ch] =
                    (c[ch] as f64 * 0.82 + 18.0 + noise).clamp(0.0, 255.0) as u8;
            }
        }
    }
    let project = |u: f64, v: f64| {
        [
            (80.0 + 320.0 * u + 40.0 * v) / (1.0 + 0.15 * v),
            (55.0 + 15.0 * u + 300.0 * v) / (1.0 + 0.15 * v),
        ]
    };
    let quad = [
        project(0.0, 0.0),
        project(1.0, 0.0),
        project(1.0, 1.0),
        project(0.0, 1.0),
    ];
    assert_eq!(
        scan::decode_quad(w, h, &rgba, quad).unwrap().value,
        -901234567
    );
    assert_eq!(scan::decode_image(w, h, &rgba).unwrap().value, -901234567);
}
