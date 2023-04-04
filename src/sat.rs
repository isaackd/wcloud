// Naive implementation of a summed-area table
// https://en.wikipedia.org/wiki/Summed-area_table
use rand::{Rng, rngs::StdRng};

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

pub fn region_is_empty(table: &[u32], table_width: u32, region: &Region) -> bool {
    let tl = table[(region.x + region.y * table_width) as usize];
    let tr = table[(region.x  + region.width + region.y * table_width) as usize];

    let bl = table[(region.x + (region.y + region.height) * table_width) as usize];
    let br = table[(region.x + region.width + (region.y + region.height) * table_width) as usize];

    tl as i32 + br as i32 - tr as i32 - bl as i32 == 0
}

// https://en.wikipedia.org/wiki/Reservoir_sampling
pub fn find_space_for_rect(table: &[u32], table_width: u32, table_height: u32, rect: &Rect, rng: &mut StdRng) -> Option<Point> {

    let max_x = table_width - rect.width;
    let max_y = table_height - rect.height;

    let mut available_points = 0;
    let mut random_point = None;

    for y in 0..max_y {
        for x in 0..max_x {
            let region = Region { x, y, width: rect.width, height: rect.height };
            if region_is_empty(&table, table_width, &region) {

                let random_num = rng.gen_range(0..=available_points);
                if random_num == available_points {
                    random_point = Some(Point { x, y });
                }
                available_points += 1;
            }
        }
    }

    random_point
}

pub fn to_summed_area_table(table: &mut [u32], width: usize, height: usize) {
    // Sum each row
    table.chunks_exact_mut(width)
        .for_each(|row| {
            let mut acc = 0;
            for el in row {
                let prev_value = *el;
                *el += acc;
                acc += prev_value;

            }
        });

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
}
