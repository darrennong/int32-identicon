use int32_identicon_core as core;
use std::{env, fs, path::Path};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02X}")).collect()
}

fn save(
    value: i32,
    target: &Path,
    fg: Option<u8>,
    size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    match target.extension().and_then(|s| s.to_str()) {
        Some("svg") => fs::write(target, core::svg_with_size(value, fg, size)?)?,
        Some("png") => {
            let (size, pixels) = core::rgba_with_size(value, fg, size)?;
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
        Some("encode") if args.len() >= 3 => {
            let value = args[1].parse::<i32>()?;
            let mut foreground = None;
            let mut size = None;
            let mut options = args[3..].iter();
            while let Some(option) = options.next() {
                match option.as_str() {
                    "--size" if size.is_none() => {
                        let pixels = options.next().ok_or("--size requires a pixel value")?;
                        size = Some(
                            pixels
                                .parse::<usize>()
                                .ok()
                                .filter(|size| {
                                    (core::MIN_IMAGE_SIZE..=core::MAX_IMAGE_SIZE).contains(size)
                                })
                                .ok_or_else(|| {
                                    format!(
                                        "--size must be an integer from {} to {} pixels",
                                        core::MIN_IMAGE_SIZE,
                                        core::MAX_IMAGE_SIZE
                                    )
                                })?,
                        );
                    }
                    "--size" => return Err("--size may only be specified once".into()),
                    id if foreground.is_none() && !id.starts_with("--") => {
                        foreground = Some(id.parse::<u8>()?);
                    }
                    _ => return Err(format!("unexpected encode argument: {option}").into()),
                }
            }
            save(
                value,
                Path::new(&args[2]),
                foreground,
                size.unwrap_or(core::DEFAULT_IMAGE_SIZE),
            )?;
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
                for extension in ["svg", "png"] {
                    save(
                        value,
                        &dir.join(format!("{stem}.{extension}")),
                        None,
                        core::DEFAULT_IMAGE_SIZE,
                    )?;
                }
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
            eprintln!(
                "Usage:\n  identicon encode <int32> <output.svg|output.png> [foreground-id] [--size <pixels>]\n  identicon decode <input.png|input.jpg>\n  identicon examples <directory>\n\nImage size defaults to {} px and accepts {}–{} px.",
                core::DEFAULT_IMAGE_SIZE,
                core::MIN_IMAGE_SIZE,
                core::MAX_IMAGE_SIZE
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
