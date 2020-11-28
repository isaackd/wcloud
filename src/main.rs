use std::io::{self, Read, stdout};
use wcloud::{Tokenizer, WordCloud, WordCloudSize};
use clap::{Arg, App};
use regex::Regex;
use std::fs;
use std::collections::HashSet;
use image::codecs::png::PngEncoder;
use image::ColorType;
use ab_glyph::{FontVec, FontRef};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {

    let matches = App::new("wcloud")
        .version(VERSION)
        .author("afrmtbl <afrmtbl@gmail.com>")
        .about("Generate wordclouds!")
        .arg(Arg::with_name("text")
            .long("text")
            .value_name("FILE")
            .help("Specifies the file of words to build the wordcloud with"))
        .arg(Arg::with_name("regex")
            .long("regex")
            .value_name("REGEX")
            .help("Sets a custom regex to tokenize words with"))
        .arg(Arg::with_name("width")
            .long("width")
            .value_name("NUM")
            .help("Sets the width of the wordcloud"))
        .arg(Arg::with_name("height")
            .long("height")
            .value_name("NUM")
            .help("Sets the height of the wordcloud"))
        .arg(Arg::with_name("scale")
            .long("scale")
            .value_name("NUM")
            .help("Sets the scale of the final wordcloud image, relative to the width and height"))
        .arg(Arg::with_name("margin")
            .long("margin")
            .value_name("NUM")
            .help("Sets the spacing between words"))
        .arg(Arg::with_name("max-words")
            .long("max-words")
            .value_name("NUM"))
        .arg(Arg::with_name("min-font-size")
            .long("min-font-size")
            .value_name("NUM")
            .help("Sets the minimum font size for words"))
        .arg(Arg::with_name("max-font-size")
            .long("max-font-size")
            .value_name("NUM")
            .help("Sets the maximum font size for words"))
        .arg(Arg::with_name("random-seed")
            .long("random-seed")
            .value_name("NUM")
            .help("Sets the randomness seed for the wordcloud for reproducible wordclouds"))
        .arg(Arg::with_name("repeat")
            .long("repeat")
            .help("Whether to repeat words until the maximum word count is reached"))
        .arg(Arg::with_name("font-step")
            .long("font-step")
            .value_name("NUM")
            .help("Sets the amount to decrease the font size by when no space can be found for a word [1]"))
        .arg(Arg::with_name("rotate-chance")
            .long("rotate-chance")
            .value_name("NUM")
            .help("Sets the chance that words are rotated (0.0 - not at all, 1.0 - every time) [0.1]"))
        .arg(Arg::with_name("relative-scaling")
            .long("relative-scaling")
            .value_name("NUM")
            .help("Sets how much of an impact word frequency has on the font size of the word (0.0 - 1.0) [0.5]"))
        .arg(Arg::with_name("mask")
            .long("mask")
            .value_name("FILE")
            .help("Sets the boolean mask image for the wordcloud shape. Any color other than black (#000) means there is no space"))
        .arg(Arg::with_name("exclude-words")
            .long("exclude-words")
            .value_name("FILE")
            .help("A newline-separated list of words to exclude from the wordcloud"))
        .arg(Arg::with_name("output")
            .long("output")
            .short("o")
            .value_name("FILE")
            .help("The output path of the final wordcloud image"))
        .arg(Arg::with_name("font")
            .long("font")
            .short("f")
            .value_name("FILE")
            .help("Sets the font used for the wordcloud"))
        .get_matches();

    let mut tokenizer = Tokenizer::default();

    if matches.is_present("repeat") {
        tokenizer = tokenizer.with_repeat(true);
    }

    if let Some(max_words) = matches.value_of("max-words") {
        let max_words = max_words
            .parse()
            .expect("Max words must be a number greater than 0");
        tokenizer = tokenizer.with_max_words(max_words);
    }

    if let Some(regex_str) = matches.value_of("regex") {
        let regex = match Regex::new(regex_str) {
            Ok(regex) => regex,
            Err(e) => {
                println!("{}", e);
                std::process::exit(1)
            }
        };

        tokenizer = tokenizer.with_regex(regex);
    }

    let exclude_words = if let Some(exclude_words_path) = matches.value_of("exclude-words") {
        fs::read_to_string(exclude_words_path)
            .expect(&format!("Unable to read exclude words file \'{}\'", exclude_words_path))
    }
    else {
        String::new()
    };

    if !exclude_words.is_empty() {
        let exclude_words = exclude_words.split("\n").collect::<HashSet<_>>();
        tokenizer = tokenizer.with_filter(exclude_words);
    }


    let wordcloud_size = match matches.value_of("mask") {
        Some(mask_path) => {
            let mask_image = image::open(mask_path).unwrap().to_luma();

            WordCloudSize::FromMask(mask_image)
        },
        None => {
            let width = matches.value_of("width")
                .unwrap_or("400")
                .parse()
                .expect("Width must be an integer larger than 0");
            let height = matches.value_of("height")
                .unwrap_or("200")
                .parse()
                .expect("Height must be an integer larger than 0");

            WordCloudSize::FromDimensions { width, height }
        }
    };

    let mut wordcloud = WordCloud::default()
        .with_tokenizer(tokenizer);

    if let Some(margin) = matches.value_of("margin") {
        wordcloud = wordcloud.with_word_margin(
            margin.parse()
                .expect("Margin must be a valid number")
        );
    }

    if let Some(min_font_size) = matches.value_of("min-font-size") {
        wordcloud = wordcloud.with_min_font_size(
            min_font_size.parse()
                .expect("The minimum font size must be a valid number")
        );
    }

    if let Some(max_font_size) = matches.value_of("max-font-size") {
        wordcloud = wordcloud.with_max_font_size(
            Some(max_font_size.parse()
                .expect("The maximum font size must be a valid number"))
        );
    }

    if let Some(random_seed) = matches.value_of("random-seed") {
        wordcloud = wordcloud.with_rng_seed(
            random_seed.parse()
                .expect("The random seed must be a valid number")
        );
    }

    if let Some(font_step) = matches.value_of("font-step") {
        wordcloud = wordcloud.with_font_step(
            font_step.parse()
                .expect("The random seed must be a valid number")
        );
    }

    if let Some(rotate_chance) = matches.value_of("rotate-chance") {
        wordcloud = wordcloud.with_word_rotate_chance(
            rotate_chance.parse()
                .expect("The rotate chance must be a number between 0 and 1 (default: 0.10)")
        );
    }

    let mut font_file = None;

    if let Some(font_path) = matches.value_of("font") {
        font_file = Some(fs::read(font_path).expect("Unable to read font file"));
        wordcloud = wordcloud.with_font(
            FontRef::try_from_slice(font_file.unwrap().as_slice())
                .expect("Font file may be invalid")
        );
    }

    let scale = matches.value_of("scale")
        .unwrap_or("1.0")
        .parse()
        .expect("Scale must be a number between 0 and 100");

    let text = if let Some(text_file_path) = matches.value_of("text") {
        fs::read_to_string(text_file_path)
            .expect(&format!("Unable to read text file \'{}\'", text_file_path))
    }
    else {
        let mut buffer = String::new();
        let stdin = io::stdin().read_to_string(&mut buffer);

        buffer
    };


//     let text = "of course it was a disaster.
// that unbearable, dearest secret
// has always been a disaster.
// the danger when we try to leave.
// going over and over afterward
// what we should have done
// instead of what we did.
// but for those short times
// we seemed to be alive. misled,
// misused, lied to and cheated,
// certainly. still, for that
// little while, we visited
// our possible life.";


    let wordcloud_image = wordcloud.generate_from_text(&text, wordcloud_size, scale);

    if let Some(file_path) = matches.value_of("output") {
        wordcloud_image.save(file_path)
            .expect("Failed to save WordCloud image");
    }
    else {
        let encoder = PngEncoder::new(stdout());
        encoder.encode(wordcloud_image.as_raw(), wordcloud_image.width(), wordcloud_image.height(), ColorType::Rgb8)
            .expect("Failed to save WordCloud image");
    }
}
