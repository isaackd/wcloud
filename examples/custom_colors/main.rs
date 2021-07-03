use std::fs;
use std::collections::HashSet;
use wcloud::{Tokenizer, WordCloud, WordCloudSize, Word, DEFAULT_EXCLUDE_WORDS_TEXT};
use rand::Rng;
use rand::rngs::StdRng;
use palette::{Pixel, Srgb, Hsl, IntoColor};
use image::Rgb;

fn main() {
    let script_path = "examples/custom_colors/a_new_hope.txt";
    let script_text = fs::read_to_string(script_path)
        .expect("Unable to find a_new_hope.txt")
        .replace("HAN", "Han")
        .replace("LUKE'S", "Luke");

    let mut filter = DEFAULT_EXCLUDE_WORDS_TEXT.split("\n")
        .collect::<HashSet<_>>();

    filter.insert("int");
    filter.insert("ext");

    let tokenizer = Tokenizer::default()
        .with_max_words(1000)
        .with_filter(filter);

    let wordcloud = WordCloud::default()
        .with_tokenizer(tokenizer)
        .with_word_margin(10)
        .with_rng_seed(1);

    let mask_path = "examples/custom_colors/stormtrooper_mask.png";
    let mask_image = image::open(mask_path).unwrap().to_luma();
    let mask = WordCloudSize::FromMask(mask_image);

    let color_func = |_word: &Word, rng: &mut StdRng| {
        let lightness = rng.gen_range(0.4, 1.0);

        let col = Hsl::new(0.0, 0.0, lightness)
            .into_rgb();

        let col = col.into_linear();

        let raw: [u8; 3] = Srgb::from_linear(col)
            .into_format()
            .into_raw();

        Rgb(raw)
    };

    let wordcloud_image = wordcloud.generate_from_text_with_color_func(&script_text, mask, 1.0, color_func);

    wordcloud_image.save("examples/custom_colors/a_new_hope.png")
        .expect("Unable to save image to examples/a_new_hope.png");
}
