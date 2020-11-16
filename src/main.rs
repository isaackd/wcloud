use regex::Regex;
use wcloud::Tokenizer;

use image::{DynamicImage, Rgba, GenericImage, GenericImageView, GrayImage, Luma, Rgb, RgbImage};

mod text;

mod sat;
use sat::{region_is_empty, Region};
use std::fs;
use ab_glyph::{point, FontRef, PxScale, Point};
use rand::{Rng, thread_rng};
use std::collections::HashSet;

use std::time::{Duration, Instant};
use crate::text::GlyphData;

const IMAGE_WIDTH: usize = 1000;
const IMAGE_HEIGHT: usize = 1000;

struct Word<'font> {
    font: &'font FontRef<'font>,
    scale: PxScale,
    glyphs: GlyphData,
    rotated: bool,
    position: Point,
}

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
    let exclude_words: HashSet<&str> = vec!["we"].into_iter().collect();
    let tokenizer = Tokenizer::default();
    // let words = get_normalized_word_frequencies(text, pat, exclude_words);
    let words = tokenizer.get_normalized_word_frequencies(text);

    let mask_path: Option<&str> = Some("masks/joshmask.png");
    // let mask_path: Option<&str> = None;

    println!("Words: {:?}", words);

    let pixel = Luma([0]);
    // let mut image_buffer = GrayImage::from_pixel(IMAGE_WIDTH as u32, IMAGE_HEIGHT as u32, pixel);
    let mut image_buffer = if let Some(path) = mask_path {
        image::open(path).unwrap().to_luma()
    }
    else {
        GrayImage::from_pixel(IMAGE_WIDTH as u32, IMAGE_HEIGHT as u32, Luma([0]))
    };

    println!("Made the image buffer");
    // TODO: This doesn't seem too efficient
    let mut table: Vec<u32> = image_buffer.as_raw()
        .iter()
        .map(|el| *el as u32)
        .collect();

    // TODO: Do a partial sat like the Python implementation
    sat::to_summed_area_table(&mut table, image_buffer.width() as usize, image_buffer.height() as usize);

    println!("Made the SAT");

    let min_font_size = 4.0;
    let max_font_size = 24.0;
    let font_step = 1.0;

    let font = FontRef::try_from_slice(include_bytes!("../DejaVuSansMono.ttf")).unwrap();
    let mut font_size = image_buffer.height() as f32 * words[0].1 * 0.90;

    if max_font_size < font_size {
        font_size = max_font_size;
    }

    let margin = 10;

    println!("The font size: {}", font_size);

    let mut final_words = Vec::with_capacity(words.len());

    'outer: for (word, freq) in &words {

        let mut rng = rand::thread_rng();
        let should_rotate = rng.gen_ratio(9, 100);

        let mut glyphs;

        let pos = loop {
            glyphs = text::text_to_glyphs(word, &font, PxScale::from(font_size));
            let rect = if !should_rotate {
                sat::Rect { width: glyphs.width + margin, height: glyphs.height + margin }
            }
            else {
                sat::Rect { width: glyphs.height + margin, height: glyphs.width + margin }
            };

            if rect.width > image_buffer.width() as u32 || rect.height > image_buffer.height() as u32 {
                if font_size - font_step >= min_font_size && font_size - font_step > 0.0 {
                    font_size -= font_step;
                    continue;
                }
                else {
                    break 'outer;
                }
            }

            match sat::find_space_for_rect(&table, image_buffer.width(), image_buffer.height(), &rect) {
                Some(pos) => break point(pos.x as f32 + margin as f32 / 2.0, pos.y as f32 + margin as f32 / 2.0),
                None => {
                    if font_size - font_step >= min_font_size && font_size - font_step > 0.0 {
                        font_size -= font_step;
                    }
                    else {
                        break 'outer;
                    }
                }
            };
        };
        text::draw_glyphs_to_gray_buffer(&mut image_buffer, glyphs.clone(), &font, pos, should_rotate, Luma([1]));

        final_words.push(Word {
            font: &font,
            scale: PxScale::from(font_size),
            glyphs: glyphs.clone(),
            rotated: should_rotate,
            position: pos
        });
        println!("Wrote \"{}\" at {:?}", word, pos);

        // TODO: This doesn't seem too efficient
        table = image_buffer.as_raw()
            .iter()
            .map(|el| *el as u32)
            .collect();

        // TODO: Do a partial sat like the Python implementation
        sat::to_summed_area_table(&mut table, image_buffer.width() as usize, image_buffer.height() as usize);
    }

    let background_color = Rgb([0, 0, 0]);
    let mut final_image_buffer = RgbImage::from_pixel(image_buffer.width(), image_buffer.height(), background_color);

    for Word { font, scale, glyphs, rotated, position } in final_words {
        println!("{:?} {:?} {:?} {:?} {:?}", font, scale, glyphs, rotated, position);
        let col = random_color_rgb();
        text::draw_glyphs_to_rgb_buffer(&mut final_image_buffer, glyphs, &font, position, rotated, col);
    }

    println!("{:?}", words);

    image_buffer.save("output.png");
    final_image_buffer.save("output_colored.png");
}

fn random_color_rgb() -> Rgb<u8> {
    let mut rng = thread_rng();

    let r = rng.gen_range(40, 255);
    let g = rng.gen_range(40, 255);
    let b = rng.gen_range(40, 255);

    Rgb([r, g, b])
}
