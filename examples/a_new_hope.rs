use std::fs;
use std::collections::HashSet;
use wcloud::{Tokenizer, WordCloud, WordCloudSize, Word};
use rand::Rng;
use rand::rngs::StdRng;
use palette::{Pixel, Srgb, Hsl, IntoColor};
use image::Rgb;

fn main() {
    let script_path = "examples/a_new_hope.txt";
    let script_text = fs::read_to_string(script_path)
        .expect("Unable to find a_new_hope.txt")
        .replace("HAN", "Han")
        .replace("LUKE'S", "Luke");

    let exclude_words_path = "examples/python_stopwords";
    let exclude_words = fs::read_to_string(exclude_words_path)
        .expect(&format!("Unable to read exclude words file \'{}\'", exclude_words_path));
    let mut filter = exclude_words.split("\n")
        .collect::<HashSet<_>>();

    filter.insert("int");
    filter.insert("ext");

    let tokenizer = Tokenizer::default()
        .with_max_words(0)
        .with_filter(filter);

    let mut wordcloud = WordCloud::default()
        .with_tokenizer(tokenizer)
        .with_word_margin(10)
        .with_rng_seed(1);

    let mask_path = "examples/stormtrooper_mask.png";
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

    wordcloud_image.save("examples/a_new_hope.png");
}
