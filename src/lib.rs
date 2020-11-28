use std::collections::HashMap;
use std::collections::HashSet;
use regex::{Regex, Match};
use image::{GrayImage, Rgb, RgbImage, Luma};
use ab_glyph::{FontRef, PxScale, Point, point};
use palette::{Pixel, Srgb, Hsl, IntoColor};
use std::process::exit;

mod text;
use text::GlyphData;
mod sat;
use rand::{Rng, thread_rng, SeedableRng};
use rand::rngs::StdRng;

pub struct Tokenizer<'a> {
    regex: Regex,
    filter: HashSet<&'a str>,
    min_word_length: u32,
    exclude_numbers: bool,
    max_words: u32,
    repeat: bool,
}

impl<'a> Default for Tokenizer<'a> {
    fn default() -> Self {
        let regex = Regex::new("\\w[\\w']+")
            .expect("Unable to compile tokenization regex");

        Tokenizer {
            regex,
            filter: HashSet::new(),
            min_word_length: 0,
            exclude_numbers: true,
            max_words: 200,
            repeat: false,
        }
    }
}

// TODO: Combine same words with different cases and use the most common case
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

    fn get_word_frequencies(&'a self, text: &'a str) -> (HashMap<&'a str, usize>, usize) {
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

        if frequencies.is_empty() {
            return Vec::new();
        }

        let mut normalized_freqs: Vec<(&str, f32)> = frequencies.iter().map(|(key, val)| {
            (*key, *val as f32 / max_freq as f32)
        }).collect();

        normalized_freqs.sort_by(|a, b| {
            if a.1 != b.1 {
                (b.1).partial_cmp(&a.1).unwrap()
            }
            else {
                (a.0).partial_cmp(b.0).unwrap()
            }
        });

        if self.repeat && normalized_freqs.len() < self.max_words as usize {
            let times_extend = ((self.max_words as f32 / normalized_freqs.len() as f32).ceil()) as u32 - 1;
            // println!("Times extend: {}, max_words: {}, freqs len: {}", times_extend, self.max_words, normalized_freqs.len());
            let freqs_clone = normalized_freqs.clone();
            let down_weight = normalized_freqs.last()
                .expect("The normalized frequencies vec is empty")
                .1;

            for i in 1..=times_extend {
                normalized_freqs.extend(
                    freqs_clone.iter().map(|(word, freq)| {
                        (*word, freq * down_weight.powf(i as f32))
                    })
                )
            }
        }

        normalized_freqs
    }

    pub fn with_regex(mut self, value: Regex) -> Self {
        self.regex = value;
        self
    }
    pub fn with_filter(mut self, value: HashSet<&'a str>) -> Self {
        // TODO: Make filtering case-insensitive
        self.filter = value;
        self
    }
    pub fn with_min_word_length(mut self, value: u32) -> Self {
        self.min_word_length = value;
        self
    }
    pub fn with_exclude_numbers(mut self, value: bool) -> Self {
        self.exclude_numbers = value;
        self
    }
    pub fn with_max_words(mut self, value: u32) -> Self {
        self.max_words = value;
        self
    }
    pub fn with_repeat(mut self, value: bool) -> Self {
        self.repeat = value;
        self
    }
}

pub struct Word<'font> {
    text: &'font str,
    font: &'font FontRef<'font>,
    font_size: PxScale,
    glyphs: GlyphData,
    rotated: bool,
    position: Point,
}

// TODO: Figure out a better way to structure this
pub enum WordCloudSize {
    FromDimensions { width: u32, height: u32 },
    FromMask(GrayImage),
}

pub struct WordCloud<'a> {
    tokenizer: Tokenizer<'a>,
    background_color: Rgb<u8>,
    font: FontRef<'a>,
    min_font_size: f32,
    max_font_size: Option<f32>,
    font_step: f32,
    word_margin: u32,
    word_rotate_chance: f64,
    relative_font_scaling: f32,
    rng_seed: Option<u64>,
}

impl<'a> Default for WordCloud<'a> {
    fn default() -> Self {
        let font = FontRef::try_from_slice(include_bytes!("../fonts/DroidSansMono.ttf")).unwrap();

        WordCloud {
            tokenizer: Tokenizer::default(),
            background_color: Rgb([0, 0, 0]),
            font,
            min_font_size: 4.0,
            max_font_size: None,
            font_step: 1.0,
            word_margin: 2,
            word_rotate_chance: 0.10,
            relative_font_scaling: 0.5,
            rng_seed: None,
        }
    }
}

// TODO: Macros can simplify this probably?
impl<'a> WordCloud<'a> {
    pub fn with_tokenizer(mut self, value: Tokenizer<'a>) -> Self {
        self.tokenizer = value;
        self
    }
    pub fn with_background_color(mut self, value: Rgb<u8>) -> Self {
        self.background_color = value;
        self
    }
    pub fn with_font(mut self, value: FontRef<'a>) -> Self {
        self.font = value;
        self
    }
    pub fn with_min_font_size(mut self, value: f32) -> Self {
        assert!(value >= 0.0, "The minimum font size for a word cloud cannot be less than 0");
        self.min_font_size = value;
        self
    }
    pub fn with_max_font_size(mut self, value: Option<f32>) -> Self {
        self.max_font_size = value;
        self
    }
    pub fn with_font_step(mut self, value: f32) -> Self {
        self.font_step = value;
        self
    }
    pub fn with_word_margin(mut self, value: u32) -> Self {
        self.word_margin = value;
        self
    }
    pub fn with_word_rotate_chance(mut self, value: f64) -> Self {
        self.word_rotate_chance = value;
        self
    }
    pub fn with_relative_font_scaling(mut self, value: f32) -> Self {
        assert!(value >= 0.0 && value <= 1.0, "Relative scaling must be between 0 and 1");
        self.relative_font_scaling = value;
        self
    }
    pub fn with_rng_seed(mut self, value: u64) -> Self {
        self.rng_seed.replace(value);
        self
    }
}

impl<'a> WordCloud<'a> {
    fn generate_from_word_positions(rng: &mut StdRng, width: u32, height: u32, word_positions: Vec<Word>, scale: f32, background_color: Rgb<u8>) -> RgbImage {
        // TODO: Refactor this so that we can fail earlier
        if scale < 0.0 || scale > 100.0 {
            // TODO: Idk if this is good practice
            // println!("The scale must be between 0 and 100 (both exclusive)");
            exit(1);
        }

        let mut final_image_buffer = RgbImage::from_pixel((width as f32 * scale) as u32, (height as f32 * scale) as u32, background_color);

        for Word { text, font, mut font_size, mut glyphs, rotated, mut position } in word_positions {
            // println!("{:?} {:?} {:?} {:?} {:?}", font, scale, glyphs, rotated, position);
            let col = random_color_rgb(rng);

            if scale != 1.0 {
                font_size.x *= scale;
                font_size.y *= scale;

                position.x *= scale;
                position.y *= scale;

                glyphs = text::text_to_glyphs(text, &font, font_size);
            }

            text::draw_glyphs_to_rgb_buffer(&mut final_image_buffer, glyphs, &font, position, rotated, col);
        }

        final_image_buffer
    }

    fn check_font_size(font_size: &mut f32, font_step: f32, min_font_size: f32) -> bool {
        let next_font_size = *font_size - font_step;
        // println!("Stuck: {} {} {} {}", font_size, min_font_size, font_step, next_font_size);
        if next_font_size >= min_font_size && next_font_size > 0.0 {
            *font_size = next_font_size;
            true
        }
        else {
            false
        }
    }

    pub fn generate_from_text(&self, text: &str, size: WordCloudSize, scale: f32) -> RgbImage {
        let words = self.tokenizer.get_normalized_word_frequencies(text);

        // println!("amount of words: {:?} {:?}", words.len(), words);

        let (mut summed_area_table, mut gray_buffer) = match size {
            WordCloudSize::FromDimensions { width, height } => {
                let buf = GrayImage::from_pixel(width, height, Luma([0]));
                (to_uint_vec(&buf), buf)
            },
            WordCloudSize::FromMask(image) => {
                let mut table = to_uint_vec(&image);
                sat::to_summed_area_table(
                    &mut table, image.width() as usize, image.height() as usize
                );
                (table, image)
            }
        };

        let mut final_words = Vec::with_capacity(words.len());

        let mut font_size = self.max_font_size
            .unwrap_or(gray_buffer.height() as f32 * 0.95);

        let mut last_freq = 1.0;

        let mut rng = match self.rng_seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_rng(thread_rng()).unwrap()
        };

        // println!("The amount of freqs: {}", words.len());

        'outer: for (word, freq) in &words {

            if !self.tokenizer.repeat && self.relative_font_scaling != 0.0 {
                font_size *= self.relative_font_scaling * (freq / last_freq) + (1.0 - self.relative_font_scaling);
            }

            if font_size < self.min_font_size {
                break;
            }

            let mut should_rotate = rng.gen_bool(self.word_rotate_chance);

            let mut glyphs;

            let mut tried_rotate = false;

            let pos = loop {
                glyphs = text::text_to_glyphs(word, &self.font, PxScale::from(font_size));
                let rect = if !should_rotate {
                    sat::Rect { width: glyphs.width + self.word_margin, height: glyphs.height + self.word_margin }
                }
                else {
                    sat::Rect { width: glyphs.height + self.word_margin, height: glyphs.width + self.word_margin }
                };

                if rect.width > gray_buffer.width() as u32 || rect.height > gray_buffer.height() as u32 {
                    if Self::check_font_size(&mut font_size, self.font_step, self.min_font_size) { continue } else { break 'outer; };
                }

                match sat::find_space_for_rect(&summed_area_table, gray_buffer.width(), gray_buffer.height(), &rect, &mut rng) {
                    Some(pos) => break point(pos.x as f32 + self.word_margin as f32 / 2.0, pos.y as f32 + self.word_margin as f32 / 2.0),
                    None => {
                        if !tried_rotate {
                            should_rotate = true;
                            tried_rotate = true;
                        }
                        else if !Self::check_font_size(&mut font_size, self.font_step, self.min_font_size) {
                            break 'outer;
                        }
                    }
                };
            };
            text::draw_glyphs_to_gray_buffer(&mut gray_buffer, glyphs.clone(), &self.font, pos, should_rotate);

            final_words.push(Word {
                text: word,
                font: &self.font,
                font_size: PxScale::from(font_size),
                glyphs: glyphs.clone(),
                rotated: should_rotate,
                position: pos
            });

            // TODO: Do a partial sat like the Python implementation
            summed_area_table = to_uint_vec(&gray_buffer);
            sat::to_summed_area_table(&mut summed_area_table, gray_buffer.width() as usize, gray_buffer.height() as usize);

            last_freq = *freq;
        }

        // println!("{}", final_words.len());

        // println!("{:?}", words);

        WordCloud::generate_from_word_positions(&mut rng, gray_buffer.width(), gray_buffer.height(), final_words, scale, self.background_color)
    }
}

fn random_color_rgb(rng: &mut StdRng) -> Rgb<u8> {
    let hue = rng.gen_range(0.0, 255.0);
    // TODO: Python uses 0.8 for the saturation but it looks too washed out when used here
    //   Maybe something to do with the linear stuff?
    //   It's not really a problem just curious
    //   https://github.com/python-pillow/Pillow/blob/66209168847ad1b55190a629b49cc6ba829efe92/src/PIL/ImageColor.py#L83
    let col = Hsl::new(hue, 1.0, 0.5)
        .into_rgb();

    let col = col.into_linear();

    let raw: [u8; 3] = Srgb::from_linear(col)
        .into_format()
        .into_raw();

    Rgb(raw)
}

// TODO: This doesn't seem particularly efficient
fn to_uint_vec(buffer: &GrayImage) -> Vec<u32> {
    buffer.as_raw().iter().map(|el| *el as u32).collect()
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

    #[test]
    fn simple_normalized_word_frequencies() {
        let words = "A woodchuck would chuck as much wood as a woodchuck could chuck if a woodchuck could chuck wood";

        let tokenizer = Tokenizer::default()
            .with_repeat(true)
            .with_max_words(30);
        let frequencies = tokenizer.get_normalized_word_frequencies(words);

        // println!("{:?}", frequencies);

        let expected = vec![
            ("woodchuck", 1.0), ("chuck", 1.0), ("wood", 0.6666667),
            ("could", 0.6666667), ("as", 0.6666667), ("a", 0.6666667),
            ("would", 0.33333334), ("much", 0.33333334), ("if", 0.33333334),
            ("A", 0.33333334), ("woodchuck", 0.33333334), ("chuck", 0.33333334),
            ("wood", 0.22222224), ("could", 0.22222224), ("as", 0.22222224),
            ("a", 0.22222224), ("would", 0.11111112), ("much", 0.11111112),
            ("if", 0.11111112), ("A", 0.11111112), ("woodchuck", 0.11111112),
            ("chuck", 0.11111112), ("wood", 0.07407408), ("could", 0.07407408),
            ("as", 0.07407408), ("a", 0.07407408), ("would", 0.03703704),
            ("much", 0.03703704), ("if", 0.03703704), ("A", 0.03703704)
        ];

        assert_eq!(frequencies, expected);
    }
}
