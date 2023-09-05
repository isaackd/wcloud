use std::collections::HashSet;
use wcloud::{Tokenizer, WordCloud, WordCloudSize, Word, DEFAULT_EXCLUDE_WORDS_TEXT};
use nanorand::{Rng, WyRand};
use palette::{Pixel, Srgb, Hsl, IntoColor};
use image::{ImageFormat, Rgba};

use std::time::Instant;

fn main() {

    let script_text = include_str!("tlou2.txt")
        .replace("HAN", "Han")
        .replace("LUKE'S", "Luke");

    let mut filter = DEFAULT_EXCLUDE_WORDS_TEXT.lines()
        .collect::<HashSet<_>>();

    let exclude_words = [
        "oh", "alright", "okay", "gonna", "go", "c'mon", "hey", "em",
        "maybe", "uh", "Well", "ya", "yeah", "let", "see", "didn",
        "re", "s", "come", "got", "ll", "right", "ve", "don", "t", "C"
    ];

    for word in exclude_words {
        filter.insert(word);
    }

    let tokenizer = Tokenizer::default()
        .with_max_words(1000)
        .with_filter(filter)
        .with_min_word_length(2);

    let max_font_size = Some(150.0);

    let wordcloud = WordCloud::default()
        .with_tokenizer(tokenizer)
        .with_word_margin(10)
        .with_min_font_size(20.0)
        // .with_max_font_size(max_font_size)
        .with_relative_font_scaling(0.25)
        .with_background_color(Rgba([0, 0, 0, 0]));

    let mask_buf = include_bytes!("mask.png");
    let mask_image = image::load_from_memory_with_format(mask_buf, ImageFormat::Png)
        .expect("Unable to load mask from memory")
        .to_luma8();

    let mask = WordCloudSize::FromMask(mask_image);

    let color_func = |word: &Word, _rng: &mut WyRand| {
        // let lightness = rng.generate_range(40..100);

        // let saturation = word.frequency / 100.0;
        let freq = (word.frequency * 100.0) as u8;

        // let saturation = match freq {
        //     90..=100 => word.frequency,
        //     20..=89 => 1.0,
        //     10..=19 => 0.8,
        //     6..=9 => 0.6,
        //     3..=5 => 0.3,
        //     _ => 0.2,
        // };

        let saturation = (200.0 - word.index as f32) / 200.0;
        let saturation = match word.index {
            0..=10 => 1.0,
            11..=20 => 0.9,
            21..=30 => 0.8,
            31..=40 => 0.75,
            41..=50 => 0.7,
            51..=70 => 0.6,
            71..=100 => 0.5,
            101..=200 => 0.4,
            201..=300 => 0.3,
            _ => 0.2,
        };

        let col = Hsl::new(136.0, saturation, 0.5);
        let rgb: Srgb = col.into_color();

        let raw: [u8; 3] = rgb.into_format()
            .into_raw();

        Rgba([raw[0], raw[1], raw[2], 1])
    };

    let now = Instant::now();
    let wordcloud_image = wordcloud.generate_from_text_with_color_func(&script_text, mask, 1.0, color_func);

    println!("Generated in {}ms", now.elapsed().as_millis());

    wordcloud_image.save("examples/tlou2/tlou2.png")
        .expect("Unable to save image");
}
