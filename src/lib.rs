use std::collections::HashMap;
use std::collections::HashSet;
use regex::{Regex, Match};
use image::{GrayImage, Rgb, RgbImage, Luma};
use ab_glyph::{FontRef, PxScale, Point, point};

mod text;
use text::GlyphData;
mod sat;
use sat::to_summed_area_table;
use rand::{Rng, thread_rng};

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

pub struct Word<'font> {
    font: &'font FontRef<'font>,
    scale: PxScale,
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
    relative_font_scaling: f32
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
            word_margin: 10,
            word_rotate_chance: 0.10,
            relative_font_scaling: 0.5,
        }
    }
}

fn check_font_size(font_size: &mut f32, font_step: f32, min_font_size: f32) -> bool {
    let next_font_size = *font_size - font_step;
    if next_font_size >= min_font_size && next_font_size > 0.0 {
        *font_size = next_font_size;
        true
    }
    else {
        false
    }
}

impl<'a> WordCloud<'a> {
    fn generate_from_word_positions(width: u32, height: u32, word_positions: Vec<Word>, background_color: Rgb<u8>) -> RgbImage {
        let mut final_image_buffer = RgbImage::from_pixel(width, height, background_color);

        for Word { font, scale, glyphs, rotated, position } in word_positions {
            // println!("{:?} {:?} {:?} {:?} {:?}", font, scale, glyphs, rotated, position);
            let col = random_color_rgb();
            text::draw_glyphs_to_rgb_buffer(&mut final_image_buffer, glyphs, &font, position, rotated, col);
        }

        final_image_buffer
    }

    pub fn generate_from_text(&self, text: &str, size: WordCloudSize) -> RgbImage {
        let words = self.tokenizer.get_normalized_word_frequencies(text);

        // TODO: Theres probably a cleaner way to do this
        let (mut summed_area_table, mut gray_buffer) = match size {
            WordCloudSize::FromDimensions { width, height } => {
                let buf = GrayImage::from_pixel(width, height, Luma([0]));
                println!("Made the gray image buffer!");
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

        let mut final_words = Vec::with_capacity(words.len());

        let mut font_size = self.max_font_size
            .unwrap_or(gray_buffer.height() as f32 * 0.90);

        let mut last_freq = 1.0;

        'outer: for (word, freq) in &words {

            if self.relative_font_scaling != 0.0 {
                font_size *= self.relative_font_scaling * (freq / last_freq) + (1.0 - self.relative_font_scaling);
            }

            let mut rng = rand::thread_rng();
            let should_rotate = rng.gen_bool(self.word_rotate_chance);

            let mut glyphs;

            let pos = loop {
                glyphs = text::text_to_glyphs(word, &self.font, PxScale::from(font_size));
                let rect = if !should_rotate {
                    sat::Rect { width: glyphs.width + self.word_margin, height: glyphs.height + self.word_margin }
                }
                else {
                    sat::Rect { width: glyphs.height + self.word_margin, height: glyphs.width + self.word_margin }
                };

                if rect.width > gray_buffer.width() as u32 || rect.height > gray_buffer.height() as u32 {
                    if check_font_size(&mut font_size, self.font_step, self.min_font_size) { continue } else { break 'outer; };
                }

                match sat::find_space_for_rect(&summed_area_table, gray_buffer.width(), gray_buffer.height(), &rect) {
                    Some(pos) => break point(pos.x as f32 + self.word_margin as f32 / 2.0, pos.y as f32 + self.word_margin as f32 / 2.0),
                    None => {
                        if check_font_size(&mut font_size, self.font_step, self.min_font_size) { continue } else { break 'outer; };
                    }
                };
            };
            text::draw_glyphs_to_gray_buffer(&mut gray_buffer, glyphs.clone(), &self.font, pos, should_rotate, Luma([1]));

            final_words.push(Word {
                font: &self.font,
                scale: PxScale::from(font_size),
                glyphs: glyphs.clone(),
                rotated: should_rotate,
                position: pos
            });
            println!("Wrote \"{}\" at {:?}", word, pos);

            summed_area_table = to_uint_vec(&gray_buffer);

            // TODO: Do a partial sat like the Python implementation
            sat::to_summed_area_table(&mut summed_area_table, gray_buffer.width() as usize, gray_buffer.height() as usize);

            last_freq = *freq;
        }

        // println!("{:?}", words);

        WordCloud::generate_from_word_positions(gray_buffer.width(), gray_buffer.height(), final_words, self.background_color)
    }
}

fn random_color_rgb() -> Rgb<u8> {
    let mut rng = thread_rng();

    let r = rng.gen_range(40, 255);
    let g = rng.gen_range(40, 255);
    let b = rng.gen_range(40, 255);

    Rgb([r, g, b])
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
}
