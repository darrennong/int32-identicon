use int32_identicon_core as core;
use std::{env, fs, path::Path};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02X}")).collect()
}

fn save(value: i32, target: &Path, fg: Option<u8>) -> Result<(), Box<dyn std::error::Error>> {
    match target.extension().and_then(|s| s.to_str()) {
        Some("svg") => fs::write(target, core::svg(value, fg)?)?,
        Some("png") => {
            let (size, pixels) = core::rgba(value, fg, 32)?;
            image::save_buffer(
                target,
                &pixels,
                size as u32,
                size as u32,
                image::ColorType::Rgba8,
            )?;
        }
        _ => return Err("output extension must be .svg or .png".into()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("encode") if (3..=4).contains(&args.len()) => {
            let value = args[1].parse::<i32>()?;
            let fg = args.get(3).map(|s| s.parse::<u8>()).transpose()?;
            save(value, Path::new(&args[2]), fg)?;
        }
        Some("decode") if args.len() == 2 => {
            let image = image::io::Reader::open(&args[1])?
                .with_guessed_format()?
                .into_dimensions()?;
            if image.0 > 4096 || image.1 > 4096 {
                return Err("maximum image dimensions are 4096x4096".into());
            }
            let image = image::open(&args[1])?.to_rgba8();
            let result =
                core::scan::decode_image(image.width() as usize, image.height() as usize, &image)?;
            println!(
                "{}",
                serde_json::json!({"value":result.value,"foreground":result.foreground,"repairedBytes":result.repaired_bytes,"version":1})
            );
        }
        Some("examples") if args.len() == 2 => {
            let dir = Path::new(&args[1]);
            fs::create_dir_all(dir)?;
            let mut seed = 0x4933_3244_u32;
            let mut manifest = Vec::new();
            for i in 0..10 {
                // Reproducible pseudorandom samples, not security randomness.
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let value = seed as i32;
                let stem = format!("{:02}_{value}", i + 1);
                save(value, &dir.join(format!("{stem}.svg")), None)?;
                save(value, &dir.join(format!("{stem}.png")), None)?;
                // Reopen the actual PNG and decode pixels, never its filename or metadata.
                let image = image::open(dir.join(format!("{stem}.png")))?.to_rgba8();
                let decoded = core::scan::decode_image(
                    image.width() as usize,
                    image.height() as usize,
                    &image,
                )?;
                if decoded.value != value {
                    return Err("example round-trip mismatch".into());
                }
                let word = core::encode(value, None)?;
                manifest.push(serde_json::json!({"value":value,"foreground":decoded.foreground,"codewordHex":hex(&word),"crcHex":hex(&word[5..7]),"maskedHex":hex(&std::array::from_fn::<u8,9,_>(|j|word[j]^core::MASK[j])),"png":format!("{stem}.png"),"svg":format!("{stem}.svg"),"decodedFromPng":decoded.value}));
            }
            fs::write(
                dir.join("manifest.json"),
                serde_json::to_string_pretty(
                    &serde_json::json!({"protocol":1,"seedHex":"49333244","examples":manifest}),
                )? + "\n",
            )?;
            println!("generated and pixel-decoded 10 PNG examples + 10 SVG equivalents");
        }
        _ => {
            eprintln!("Usage:\n  identicon encode <int32> <output.svg|output.png> [foreground-id]\n  identicon decode <input.png|input.jpg>\n  identicon examples <directory>");
            std::process::exit(2);
        }
    }
    Ok(())
}
