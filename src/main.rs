use regex::Regex;
use wcloud::{Tokenizer, Word, WordCloud, WordCloudSize};

use image::{DynamicImage, Rgba, GenericImage, GenericImageView, GrayImage, Luma, Rgb, RgbImage};

mod text;

use std::collections::HashSet;

fn main() {
    let text = "of course it was a disaster.
that unbearable, dearest secret
has always been a disaster.
the danger when we try to leave.
going over and over afterward
what we should have done
instead of what we did.
but for those short times
we seemed to be alive. misled,
misused, lied to and cheated,
certainly. still, for that
little while, we visited
our possible life.";
    // let exclude_words: HashSet<&str> = vec!["we"].into_iter().collect();

    let mask_path = "masks/joshmask.png";
    let mut mask_image = image::open(mask_path).unwrap().to_luma();

    let tokenizer = Tokenizer::default()
        .with_repeat(true);

    let wordcloud_size = WordCloudSize::FromDimensions { width: 800, height: 400 };
    // let wordcloud_size = WordCloudSize::FromMask(mask_image);
    let wordcloud = WordCloud::default()
        .with_tokenizer(tokenizer);
    let wordcloud = wordcloud.generate_from_text(text, wordcloud_size);

    wordcloud.save("output.png")
        .expect("Failed to save WordCloud image");
}
