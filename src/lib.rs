use std::collections::HashMap;
use std::collections::HashSet;
use regex::{Regex, Match};
use image::{GrayImage, Rgb, RgbImage, Luma};
use ab_glyph::FontRef;

pub mod sat;

pub struct Tokenizer<'a> {
    regex: Regex,
    filter: HashSet<&'a str>,
    min_word_length: u32,
    exclude_numbers: bool,
    max_words: u32,
}

impl<'a> Default for Tokenizer<'a> {
    fn default() -> Self {
        let regex = Regex::new("\\w[\\w']*")
            .expect("Unable to compile tokenization regex");

        Tokenizer {
            regex,
            filter: HashSet::new(),
            min_word_length: 0,
            exclude_numbers: true,
            max_words: 200,
        }
    }
}

impl<'a> Tokenizer<'a> {
    fn tokenize(&'a self, text: &'a str) -> Box<dyn Iterator<Item=Match<'a>> + 'a> {
        let mut result: Box<dyn Iterator<Item=Match<'a>> + 'a>
            = Box::new(self.regex.find_iter(text));

        if self.max_words != 0 {
            result = Box::new(result.take(self.max_words as usize));
        }
        if !self.filter.is_empty() {
            result = Box::new(result.filter(move |word| !self.filter.contains(word.as_str())));
        }
        if self.min_word_length > 0 {
            result = Box::new(result.filter(move |word| word.as_str().len() >= self.min_word_length as usize));
        }
        if self.exclude_numbers {
            result = Box::new(result.filter(move |word| !word.as_str().chars().all(char::is_numeric)));
        }

        result
    }

    pub fn get_word_frequencies(&'a self, text: &'a str) -> (HashMap<&'a str, usize>, usize) {
        let mut frequencies = HashMap::new();
        let mut max_freq = 0;

        let included_words = self.tokenize(text);

        for word in included_words {
            let entry = frequencies.entry(word.as_str()).or_insert(0);
            *entry += 1;

            if *entry > max_freq {
                max_freq = *entry;
            }
        }

        (frequencies, max_freq)
    }

    pub fn get_normalized_word_frequencies(&'a self, text: &'a str) -> Vec<(&'a str, f32)> {
        let (frequencies, max_freq) = self.get_word_frequencies(text);

        let mut normalized: Vec<(&str, f32)> = frequencies.iter().map(|(key, val)| {
            (*key, *val as f32 / max_freq as f32)
        }).collect();

        normalized.sort_unstable_by(|a, b| {
            if a.1 != b.1 {
                (b.1).partial_cmp(&a.1).unwrap()
            }
            else {
                (b.0).partial_cmp(&a.0).unwrap()
            }
        });
        normalized
    }
}

enum WordCloudSize {
    FromSize { width: u32, height: u32 },
    FromMask(GrayImage),
}

struct WordCloud<'a> {
    tokenizer: Tokenizer<'a>,
    background_color: Rgb<u8>,
    font: FontRef<'a>,
    min_font_size: f32,
    max_font_size: f32,
    font_step: f32,
    word_margin: u32,
    word_rotate_chance: f64,
}

// TODO: This doesn't seem particularly efficient
fn to_uint_vec(buffer: &GrayImage) -> Vec<u32> {
    buffer.as_raw().iter().map(|el| *el as u32).collect()
}

impl<'a> WordCloud<'a> {
    fn generate_from_text(&self, text: &str, size: WordCloudSize) -> RgbImage {
        let frequencies = self.tokenizer.get_word_frequencies(text);

        // TODO: Theres probably a cleaner way to do this
        let (summed_area_table, gray_buffer) = match size {
            WordCloudSize::FromSize { width, height } => {
                let buf = GrayImage::from_pixel(width, height, Luma([0]));
                (to_uint_vec(&buf), buf)
            },
            WordCloudSize::FromMask(image) => {
                let mut table = to_uint_vec(&image);
                sat::to_summed_area_table(
                    &mut table,
                    image.width() as usize,
                    image.height() as usize
                );
                (table, image)
            }
        };

        RgbImage::new(10, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_word_frequencies() {
        let words = "A woodchuck would chuck as much wood as a woodchuck could chuck if a woodchuck could chuck wood";

        let tokenizer = Tokenizer::default();
        let frequencies = tokenizer.get_word_frequencies(words);

        let expected: HashMap<&str, usize> = vec![
            ("if", 1), ("a", 2), ("chuck", 3),
            ("would", 1), ("woodchuck", 3), ("A", 1),
            ("as", 2), ("could", 2), ("much", 1),
            ("wood", 2)
        ].into_iter().collect();

        assert_eq!(frequencies.0, expected);
        assert_eq!(frequencies.1, 3);
    }
}
