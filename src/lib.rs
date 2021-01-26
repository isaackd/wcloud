use image::{GrayImage, Rgb, RgbImage, Luma};
use ab_glyph::{PxScale, Point, point, FontVec};
use palette::{Pixel, Srgb, Hsl, IntoColor};
use std::process::exit;

mod text;
use text::GlyphData;
mod sat;
mod tokenizer;
pub use tokenizer::Tokenizer;

use rand::{Rng, thread_rng, SeedableRng};
use rand::rngs::StdRng;

pub struct Word<'a> {
    text: &'a str,
    font: &'a FontVec,
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

pub struct WordCloud {
    tokenizer: Tokenizer,
    background_color: Rgb<u8>,
    font: FontVec,
    min_font_size: f32,
    max_font_size: Option<f32>,
    font_step: f32,
    word_margin: u32,
    word_rotate_chance: f64,
    relative_font_scaling: f32,
    rng_seed: Option<u64>,
}

impl<'a> Default for WordCloud {
    fn default() -> Self {
        let font = FontVec::try_from_vec(include_bytes!("../fonts/DroidSansMono.ttf").to_vec()).unwrap();

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

impl WordCloud {
    pub fn with_tokenizer(mut self, value: Tokenizer) -> Self {
        self.tokenizer = value;
        self
    }
    pub fn with_background_color(mut self, value: Rgb<u8>) -> Self {
        self.background_color = value;
        self
    }
    pub fn with_font(mut self, value: FontVec) -> Self {
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

impl WordCloud {
    fn generate_from_word_positions(
        rng: &mut StdRng,
        width: u32,
        height: u32,
        word_positions: Vec<Word>,
        scale: f32,
        background_color: Rgb<u8>,
        color_func: fn(&Word, &mut StdRng) -> Rgb<u8>
    ) -> RgbImage {
        // TODO: Refactor this so that we can fail earlier
        if scale < 0.0 || scale > 100.0 {
            // TODO: Idk if this is good practice
            // println!("The scale must be between 0 and 100 (both exclusive)");
            exit(1);
        }

        let mut final_image_buffer = RgbImage::from_pixel((width as f32 * scale) as u32, (height as f32 * scale) as u32, background_color);

        for mut word in word_positions.into_iter() {
            // println!("{:?} {:?} {:?} {:?} {:?}", font, scale, glyphs, rotated, position);

            let col = color_func(&word, rng);

            if scale != 1.0 {
                word.font_size.x *= scale;
                word.font_size.y *= scale;

                word.position.x *= scale;
                word.position.y *= scale;

                word.glyphs = text::text_to_glyphs(word.text, &word.font, word.font_size);
            }

            text::draw_glyphs_to_rgb_buffer(&mut final_image_buffer, word.glyphs, &word.font, word.position, word.rotated, col);
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
        self.generate_from_text_with_color_func(text, size, scale, random_color_rgb)
    }

    pub fn generate_from_text_with_color_func(
        &self,
        text: &str,
        size: WordCloudSize,
        scale: f32,
        color_func: fn(&Word, &mut StdRng) -> Rgb<u8>
    ) -> RgbImage {
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

            println!("Placing word {}", word);

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
                    Some(pos) => {
                        let half_margin = self.word_margin as f32 / 2.0;
                        let x = pos.x as f32 + half_margin;
                        let y = pos.y as f32 + half_margin;
                        break point(x, y)
                    },
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

        WordCloud::generate_from_word_positions(
            &mut rng, gray_buffer.width(), gray_buffer.height(), final_words, scale, self.background_color, color_func
        )
    }
}

fn random_color_rgb(_word: &Word, rng: &mut StdRng) -> Rgb<u8> {
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
