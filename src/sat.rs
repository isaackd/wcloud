// Naive implementation of a summed-area table
// https://en.wikipedia.org/wiki/Summed-area_table
// TODO: For some reason, encapsulating the SAT functionality in a struct makes it slower
//   Changing it from using its table: Vec<u32> field directly to having the methods take a
//   &[u32] instead speeds it up quite a bit but it's still a bit slower than the loose functions
//   approach. Maybe something to do with using `self.field` syntax instead of having it passed into
//   the function? I'm not good enough with asm to figure it out
use rand::{thread_rng, Rng};
use image::GrayImage;

#[derive(Debug)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct Rect {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

pub struct SummedAreaTable {
    table: Vec<u32>,
    width: u32,
    height: u32,
}

fn to_uint_vec(buffer: &GrayImage) -> Vec<u32> {
    buffer.as_raw().iter().map(|el| *el as u32).collect()
}

impl SummedAreaTable {

    pub fn new(base_image: &GrayImage, initial_sum: bool) -> Self {
        let table = to_uint_vec(base_image);

        let mut sat = SummedAreaTable {table, width: base_image.width(), height: base_image.height() };

        if initial_sum {
            sat.update(base_image);
        }

        sat
    }

    pub fn update(&mut self, base_image: &GrayImage) {
        self.table = to_uint_vec(base_image);
        // to_summed_area_table(&mut self.table, self.width as usize, self.height as usize);

        // Sum each row
        for y in 0..self.height {
            // println!("{:?}", table);
            for x in 1..self.width {
                let el_index = (y * self.width + x) as usize;
                self.table[el_index] += self.table[el_index - 1];
            }
        }

        // Sum each column
        for y in 1..self.height {
            for x in 0..self.width {
                let el_index = (x + y * self.width) as usize;
                self.table[el_index] += self.table[el_index - self.width as usize];
            }
        }
    }

    pub fn find_space_for_rect(&self, table_width: u32, table_height: u32, rect: &Rect) -> Option<Point> {
        find_space_for_rect(&self.table, self.width, self.height, rect)
    }
}

// region: x, y, width, height
pub fn region_is_empty(table: &[u32], table_width: u32, region: &Region) -> bool {
    let tl = table[(region.x + region.y * table_width) as usize];
    let tr = table[(region.x  + region.width + region.y * table_width) as usize];

    let bl = table[(region.x + (region.y + region.height) * table_width) as usize];
    let br = table[(region.x + region.width + (region.y + region.height) * table_width) as usize];

    tl as i32 + br as i32 - tr as i32 - bl as i32 == 0
}

pub fn find_space_for_rect(table: &[u32], table_width: u32, table_height: u32, rect: &Rect) -> Option<Point> {
    // TODO: Determine whether there's a better way to get a random point
    //  It is not feasible to store all the points because there can be way too many
    //  on larger images and smaller regions
    //  ----
    //  Python wordcloud does this by:
    //    1. Count how many points there are to place this region
    //    2. Choose a random point while running the algorithm for a second time
    //  For now we will do the same

    let mut available_points = 0;

    let max_x = table_width - rect.width;
    let max_y = table_height - rect.height;
    //
    // println!("We have to check {} and {} of x and ys ({}) | {:?}", max_x, max_y, max_x * max_y, rect);

    for y in 0..max_y {
        for x in 0..max_x {
            let region = Region { x, y, width: rect.width, height: rect.height };
            if region_is_empty(&table, table_width, &region) {
                available_points += 1;
            }
        }
    }

    // println!("Found {} spaces", available_points);

    if available_points == 0 {
        return None;
    }

    // let mut rng = thread_rng();
    // let chosen_point_index: u32 = rng.gen_range(0, available_points + 1);
    let chosen_point_index: u32 = 1;
    println!("Chose as point index: {}", chosen_point_index);
    available_points = 0;

    for y in 0..max_y {
        for x in 0..max_x {
            let region = Region { x, y, width: rect.width, height: rect.height };
            if region_is_empty(&table, table_width, &region) {
                available_points += 1;

                if available_points == chosen_point_index {
                    return Some(Point { x, y });
                }
            }
        }
    }

    None
}

pub fn to_summed_area_table(table: &mut [u32], width: usize, height: usize) {
    // Sum each row
    for y in 0..height {
        // println!("{:?}", table);
        for x in 1..width {
            let el_index = y * width + x;
            table[el_index] += table[el_index - 1];
        }
    }

    // Sum each column
    for y in 1..height {
        for x in 0..width {
            let el_index = x + y * width;
            table[el_index] += table[el_index - width];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplest_sat() {
        let mut table = [1, 1, 1, 1, 1, 1, 1, 1, 1];
        to_summed_area_table(&mut table, 3, 3);

        let expected = [1, 2, 3, 2, 4, 6, 3, 6, 9];
        assert_eq!(table, expected);
    }

    #[test]
    fn simple_sat() {
        let mut table = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100, 200, 300, 400, 500, 600];
        to_summed_area_table(&mut table, 4, 4);

        let expected = [1, 3, 6, 10, 6, 14, 24, 36, 15, 33, 143, 355, 315, 733, 1343, 2155];
        assert_eq!(table, expected);
    }
    #[test]
    fn uneven_sat() {
        let mut table = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100, 200, 300, 400, 500, 600];
        to_summed_area_table(&mut table, 8, 2);

        let expected = [1, 3, 6, 10, 15, 21, 28, 36, 10, 22, 125, 329, 634, 1040, 1547, 2155];
        assert_eq!(table, expected);
    }

    #[test]
    fn large_zeros_sat() {
        let mut table = vec![0; 10_000 * 10_000];
        to_summed_area_table(&mut table, 10_000, 10_000);

        assert!(true);
    }
}
