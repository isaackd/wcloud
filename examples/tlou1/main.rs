use std::collections::HashSet;
use wcloud::{Tokenizer, WordCloud, WordCloudSize, Word, DEFAULT_EXCLUDE_WORDS_TEXT};
use nanorand::{Rng, WyRand};
use palette::{Pixel, Srgb, Hsl, IntoColor};
use image::{ImageFormat, Rgba};

use std::time::Instant;

fn main() {

    let script_text = include_str!("tlou1.txt")
        .replace("HAN", "Han")
        .replace("LUKE'S", "Luke");

    let mut filter = DEFAULT_EXCLUDE_WORDS_TEXT.lines()
        .collect::<HashSet<_>>();

    let exclude_words = [
        "oh", "alright", "okay", "gonna", "go", "c'mon", "hey", "em",
        "maybe", "uh", "Well", "ya", "yeah", "let", "see"
    ];

    for word in exclude_words {
        filter.insert(word);
    }

    let tokenizer = Tokenizer::default()
        .with_max_words(100000)
        .with_filter(filter)
        .with_repeat(true);

    let max_font_size = Some(150.0);

    let wordcloud = WordCloud::default()
        .with_tokenizer(tokenizer)
        .with_word_margin(10)
        .with_min_font_size(5.0)
        .with_max_font_size(max_font_size)
        .with_relative_font_scaling(0.25);

    let mask_buf = include_bytes!("mask.png");
    let mask_image = image::load_from_memory_with_format(mask_buf, ImageFormat::Png)
        .expect("Unable to load mask from memory")
        .to_luma8();

    let mask = WordCloudSize::FromMask(mask_image);

    let color_func = |word: &Word, _rng: &mut WyRand| {
        // let lightness = rng.generate_range(40..100);

        // let saturation = word.frequency / 100.0;
        let freq = (word.frequency * 100.0) as u8;

        let saturation = match freq {
            90..=100 => word.frequency,
            20..=89 => 1.0,
            10..=19 => 0.8,
            6..=9 => 0.6,
            3..=5 => 0.3,
            _ => 0.2,
        };

        let col = Hsl::new(136.0, saturation, 0.5);
        let rgb: Srgb = col.into_color();

        let raw: [u8; 3] = rgb.into_format()
            .into_raw();

        Rgba([raw[0], raw[1], raw[2], 1])
    };

    let now = Instant::now();
    let wordcloud_image = wordcloud.generate_from_text_with_color_func(&script_text, mask, 5.0, color_func);

    println!("Generated in {}ms", now.elapsed().as_millis());

    wordcloud_image.save("examples/tlou1/tlou1.png")
        .expect("Unable to save image");
}
