use std::fs;
use std::io::{BufWriter, stdout, Write};
use std::path::{PathBuf};
use image::{GrayImage, Rgb, RgbImage, Luma};
use ab_glyph::{PxScale, Point, point, FontVec, Font, Glyph};
use palette::{Pixel, Srgb, Hsl, IntoColor};
use std::process::exit;
use woff2::decode::{convert_woff2_to_ttf, is_woff2};

mod text;
use text::GlyphData;
pub mod sat;
mod tokenizer;
pub use tokenizer::{Tokenizer, DEFAULT_EXCLUDE_WORDS_TEXT};

use rand::{Rng, thread_rng, SeedableRng};
use rand::rngs::StdRng;
use crate::sat::{Rect, Region};

#[cfg(feature = "visualize")]
mod visualize;
#[cfg(feature = "visualize")]
use crate::visualize::{Init, Message};

pub struct Word<'a> {
    pub text: &'a str,
    pub font: &'a FontVec,
    pub font_size: PxScale,
    pub glyphs: GlyphData,
    pub rotated: bool,
    pub position: Point,
}

// TODO: Figure out a better way to structure this
pub enum WordCloudSize {
    FromDimensions { width: u32, height: u32 },
    FromMask(GrayImage),
}

pub struct WordCloud {
    tokenizer: Tokenizer,
    background_color: Rgb<u8>,
    pub font: FontVec,
    min_font_size: f32,
    max_font_size: Option<f32>,
    font_step: f32,
    word_margin: u32,
    word_rotate_chance: f64,
    relative_font_scaling: f32,
    rng_seed: Option<u64>,
}

impl Default for WordCloud {
    fn default() -> Self {
        let font = FontVec::try_from_vec(include_bytes!("../fonts/Ubuntu-B.ttf").to_vec()).unwrap();

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
    pub fn with_font_from_path(mut self, path: PathBuf) -> Self {
        let font_file = if path.extension() == Some("woff2".as_ref()) {
            let buffer = fs::read(path).unwrap();
            assert!(is_woff2(&buffer));
            convert_woff2_to_ttf(&mut std::io::Cursor::new(buffer)).expect("Invalid WOFF2 file")
        }
        else {
            fs::read(path).expect("Unable to read font file")
        };

        self.font = FontVec::try_from_vec(font_file)
            .expect("Font file may be invalid");

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
        assert!((0.0..=1.0).contains(&value), "Relative scaling must be between 0 and 1");
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
        if !(0.0..=100.0).contains(&scale) {
            // TODO: Idk if this is good practice
            // println!("The scale must be between 0 and 100 (both exclusive)");
            exit(1);
        }

        let mut final_image_buffer = RgbImage::from_pixel((width as f32 * scale) as u32, (height as f32 * scale) as u32, background_color);

        for mut word in word_positions.into_iter() {
            let col = color_func(&word, rng);

            if scale != 1.0 {
                word.font_size.x *= scale;
                word.font_size.y *= scale;

                word.position.x *= scale;
                word.position.y *= scale;

                word.glyphs = text::text_to_glyphs(word.text, word.font, word.font_size);
            }

            text::draw_glyphs_to_rgb_buffer(&mut final_image_buffer, word.glyphs, word.font, word.position, word.rotated, col);
        }

        final_image_buffer
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

    fn glyphs_height(&self, glyphs: &[Glyph]) -> u32 {
        glyphs.iter().map(|g| {
            let outlined = self.font.outline_glyph(g.clone())
                .expect("Unable to outline glyph");

            let bounds = outlined.px_bounds();
            bounds.height() as u32
        }).max().expect("No glyphs!")
    }

    fn text_dimensions_at_font_size(&self, text: &str, font_size: PxScale) -> Rect {
        let glyphs = text::text_to_glyphs(text, &self.font, font_size);
        sat::Rect { width: glyphs.width + self.word_margin, height: glyphs.height + self.word_margin }
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

        let (mut summed_area_table, mut gray_buffer) = match size {
            WordCloudSize::FromDimensions { width, height } => {
                let buf = GrayImage::from_pixel(width, height, Luma([0]));
                let mut summed_area_table = vec![0; buf.len()];

                u8_to_u32_vec(&buf, &mut summed_area_table);
                (summed_area_table, buf)
            },
            WordCloudSize::FromMask(image) => {
                let mut table = vec![0; image.len()];

                u8_to_u32_vec(&image, &mut table);
                sat::to_summed_area_table(
                    &mut table, image.width() as usize, 0
                );
                (table, image)
            }
        };

        #[cfg(feature = "visualize")]
        {
            let mask = if matches!(WordCloudSize::FromMask, size) {
                Some(gray_buffer.to_vec())
            }
            else {
                None
            };

            let serialized = serde_json::to_string(&Message::InitMessage(Init {
                width: gray_buffer.width(),
                height: gray_buffer.height(),
                mask,
                font: self.font.as_slice().to_vec(),
                background_color: self.background_color.0,
            })).unwrap();
            println!("{}", serialized);
        };

        let mut final_words = Vec::with_capacity(words.len());

        let mut last_freq = 1.0;

        let mut rng = match self.rng_seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_rng(thread_rng()).unwrap()
        };

        let first_word = words.first()
            .expect("There are no words!");

        let skip_list = create_mask_skip_list(&gray_buffer);

        let mut font_size = {
            let rect_at_image_height = self.text_dimensions_at_font_size(
                first_word.0,
                PxScale::from(gray_buffer.height() as f32 * 0.95)
            );

            let height_ratio = rect_at_image_height.height as f32 / rect_at_image_height.width as f32;

            let mut start_height = gray_buffer.width() as f32 * height_ratio;

            if matches!(WordCloudSize::FromMask, _size) {
                let black_pixels = gray_buffer.as_raw().iter().filter(|p| **p == 0).count();
                let available_space: f32 = black_pixels as f32 / gray_buffer.len() as f32;
                start_height *= available_space;
            }

            if let Some(max) = self.max_font_size {
                start_height.min(max)
            }
            else {
                start_height
            }
        };

        'outer: for (word, freq) in &words {
            if !self.tokenizer.repeat && self.relative_font_scaling != 0.0 {
                font_size *= self.relative_font_scaling * (freq / last_freq) + (1.0 - self.relative_font_scaling);
            }

            if font_size < self.min_font_size {
                break;
            }

            let initial_font_size = font_size;

            let mut should_rotate = rng.gen_bool(self.word_rotate_chance);
            let mut tried_rotate = false;
            let mut glyphs;

            let pos = loop {
                glyphs = text::text_to_glyphs(word, &self.font, PxScale::from(font_size));
                let glyphs_height = self.glyphs_height(&glyphs.glyphs);

                let rect = if !should_rotate {
                    sat::Rect { width: glyphs.width + self.word_margin, height: glyphs.height + self.word_margin }
                }
                else {
                    sat::Rect { width: glyphs.height + self.word_margin, height: glyphs.width + self.word_margin }
                };

                #[cfg(feature = "visualize")]
                {
                    let serialized = serde_json::to_string(&Message::ChangeWordMessage(visualize::Word {
                        text: word.to_string(),
                        font_size: font_size as u32,
                        rect_width: rect.width,
                        rect_height: rect.height,
                        rotation: if should_rotate { 270 } else { 0 },
                    })).unwrap();
                    println!("{}", serialized);
                };

                if rect.width > gray_buffer.width() || rect.height > gray_buffer.height() {
                    if Self::check_font_size(&mut font_size, self.font_step, self.min_font_size) {
                        continue
                    }
                    else {
                        break 'outer;
                    };
                }

                if matches!(WordCloudSize::FromMask, _size) {
                    match sat::find_space_for_rect_masked(&summed_area_table, gray_buffer.width(), gray_buffer.height(), &skip_list, &rect, &mut rng) {
                        Some(pos) => {
                            let half_margin = self.word_margin as f32 / 2.0;
                            let x = pos.x as f32 + half_margin;
                            let y = pos.y as f32 + half_margin;

                            break point(x, y)
                        },
                        None => {
                            if !Self::check_font_size(&mut font_size, self.font_step, self.min_font_size) {
                                if !tried_rotate {
                                    should_rotate = true;
                                    tried_rotate = true;
                                    font_size = initial_font_size;
                                }
                                else {
                                    break 'outer;
                                }
                            }
                        }
                    };
                }
                else {
                    match sat::find_space_for_rect(&summed_area_table, gray_buffer.width(), gray_buffer.height(), &rect, &mut rng) {
                        Some(pos) => {
                            let half_margin = self.word_margin as f32 / 2.0;
                            let x = pos.x as f32 + half_margin;
                            let y = pos.y as f32 + half_margin;

                            break point(x, y)
                        },
                        None => {
                            if !Self::check_font_size(&mut font_size, self.font_step, self.min_font_size) {
                                if !tried_rotate {
                                    should_rotate = true;
                                    tried_rotate = true;
                                    font_size = initial_font_size;
                                }
                                else {
                                    break 'outer;
                                }
                            }
                        }
                    };
                }

            };
            text::draw_glyphs_to_gray_buffer(&mut gray_buffer, glyphs.clone(), &self.font, pos, should_rotate);

            #[cfg(feature = "visualize")]
            {
                let serialized = serde_json::to_string(&Message::PlacedWordMessage(visualize::PlaceWord {
                    text: word.to_string(),
                    font_size: font_size as u32,
                    x: pos.x as u32,
                    y: pos.y as u32,
                    rotation: if should_rotate { 270 } else { 0 },
                })).unwrap();
                println!("{}", serialized);
            };

            final_words.push(Word {
                text: word,
                font: &self.font,
                font_size: PxScale::from(font_size),
                glyphs: glyphs.clone(),
                rotated: should_rotate,
                position: pos
            });

            // TODO: Do a partial sat like the Python implementation
            u8_to_u32_vec(&gray_buffer, &mut summed_area_table);
            let start_row = (pos.y - 1.0).min(0.0) as usize;
            sat::to_summed_area_table(&mut summed_area_table, gray_buffer.width() as usize, start_row);

            last_freq = *freq;
        }

        WordCloud::generate_from_word_positions(
            &mut rng, gray_buffer.width(), gray_buffer.height(), final_words, scale, self.background_color, color_func
        )
    }
}

fn random_color_rgb(_word: &Word, rng: &mut StdRng) -> Rgb<u8> {
    let hue = rng.gen_range(0.0..255.0);
    // TODO: Python uses 0.8 for the saturation but it looks too washed out when used here
    //   Maybe something to do with the linear stuff?
    //   It's not really a problem just curious
    //   https://github.com/python-pillow/Pillow/blob/66209168847ad1b55190a629b49cc6ba829efe92/src/PIL/ImageColor.py#L83
    let col = Hsl::new(hue, 1.0, 0.5);
    let rgb: Srgb = col.into_color();

    let raw: [u8; 3] = rgb.into_format()
        .into_raw();

    Rgb(raw)
}

// TODO: This doesn't seem particularly efficient
fn u8_to_u32_vec(buffer: &GrayImage, dst: &mut [u32]) {
    for (i, el) in buffer.as_raw().iter().enumerate() {
        dst[i] = *el as u32;
    }
}

fn find_image_bounds(img: &GrayImage) -> Region {
    let mut min_x = img.width();
    let mut min_y = 0;
    let mut max_x = 0;
    let mut max_y = 0;

    let mut found_min_y = false;
    for (y, mut row) in img.enumerate_rows() {
        if let Some(pos) = row.position(|p| p.2 == &Luma::from([0])) {
            if !found_min_y {
                min_y = y;
                found_min_y = true;
            }

            max_y = y;

            if pos < min_x as usize {
                min_x = pos as u32;
            }

            if let Some(last_pos) = row.filter(|p| p.2 == &Luma::from([0])).last() {
                if last_pos.0 > max_x {
                    max_x = last_pos.0;
                }
            }
            else if pos > max_x as usize {
                max_x = pos as u32;
            }
        }
    }

    let width = max_x - min_x;
    let height = max_y - min_y;

    Region { x: min_x, y: min_y, width, height }
}

fn create_mask_skip_list(img: &GrayImage) -> Vec<(usize, usize)> {
    img.rows().map(|mut row| {
        let furthest_left = row.rposition(|p| p == &Luma::from([0])).unwrap_or(img.width() as usize);
        let furthest_right = row.position(|p| p == &Luma::from([0])).unwrap_or(0);

        (furthest_right, furthest_left)
    }).collect()
}