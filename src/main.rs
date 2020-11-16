use regex::Regex;
use wcloud::{Tokenizer, Word, WordCloud, WordCloudSize};

use image::{DynamicImage, Rgba, GenericImage, GenericImageView, GrayImage, Luma, Rgb, RgbImage};

mod text;

mod sat;
use sat::{region_is_empty, Region};
use std::fs;
use ab_glyph::{point, FontRef, PxScale, Point};
use rand::{Rng, thread_rng};
use std::collections::HashSet;

use std::time::{Duration, Instant};
use text::GlyphData;

fn main() {
    let pat = Regex::new("\\w[\\w']*").unwrap();
    let text = "Of course it was a disaster.
That unbearable, dearest secret
has always been a disaster.
The danger when we try to leave.
Going over and over afterward
what we should have done
instead of what we did.
But for those short times
we seemed to be alive. Misled,
misused, lied to and cheated,
certainly. Still, for that
little while, we visited
our possible life.";
    // let exclude_words: HashSet<&str> = vec!["we"].into_iter().collect();

    // let mask_path = "masks/joshmask.png";
    // let mut mask_image = image::open(path).unwrap().to_luma();

    let wordcloud_size = WordCloudSize::FromDimensions { width: 3000, height: 3000 };
    let wordcloud = WordCloud::default();
    let wordcloud = wordcloud.generate_from_text(text, wordcloud_size);

    wordcloud.save("output.png");

}
