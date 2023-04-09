use image::{ImageFormat, Luma};
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

#[derive(Clone, Debug)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

pub fn region_is_empty(top_row: &[u32], bottom_row: &[u32]) -> bool {
    let tl = *top_row.iter().next().unwrap();
    let tr = *top_row.iter().last().unwrap();

    let bl = *bottom_row.iter().next().unwrap();
    let br = *bottom_row.iter().last().unwrap();

    tl as i32 + br as i32 - tr as i32 - bl as i32 == 0
}

// https://en.wikipedia.org/wiki/Reservoir_sampling
pub fn find_space_for_rect(table: &[u32], table_width: u32, table_height: u32, rect: &Rect, rng: &mut StdRng) -> Option<Point> {
    let max_y = table_height - rect.height;

    let mut available_points = 0;
    let mut random_point = None;

    table[..table_width as usize * max_y as usize].chunks_exact(table_width as usize)
        .zip(table[..table_width as usize * max_y as usize].chunks_exact(table_width as usize)
            .skip(rect.height as usize))
        .enumerate()
        .for_each(|(y, (top_row_full, bottom_row_full))| {
            top_row_full.windows(rect.width as usize)
                .zip(bottom_row_full.windows(rect.width as usize))
                .enumerate()
                .for_each(|(x, (top_row, bottom_row))| {
                    if region_is_empty(top_row, bottom_row) {
                        let random_num = rng.gen_range(0..=available_points);
                        if random_num == available_points {
                            random_point = Some(Point { x: x as u32, y: y as u32 });
                        }
                        available_points += 1;
                    }
                });
        });

    random_point
}


pub fn to_summed_area_table(table: &mut [u32], width: usize, start_row: usize) {
    let mut prev_row = vec![0; width];

    table.chunks_exact_mut(width)
        .skip(start_row)
        .for_each(|row| {
            let mut sum = 0;
            row.iter_mut().zip(prev_row.iter()).for_each(|(el, prev_row_el)| {
                let original_value = *el;
                *el += sum + prev_row_el;
                sum += original_value;
            });

            prev_row.clone_from_slice(row);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplest_sat() {
        let mut table = [1, 1, 1, 1, 1, 1, 1, 1, 1];
        to_summed_area_table(&mut table, 3, 0);

        let expected = [1, 2, 3, 2, 4, 6, 3, 6, 9];
        assert_eq!(table, expected);
    }

    #[test]
    fn simple_sat() {
        let mut table = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100, 200, 300, 400, 500, 600];
        to_summed_area_table(&mut table, 4, 0);

        let expected = [1, 3, 6, 10, 6, 14, 24, 36, 15, 33, 143, 355, 315, 733, 1343, 2155];
        assert_eq!(table, expected);
    }
    #[test]
    fn uneven_sat() {
        let mut table = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100, 200, 300, 400, 500, 600];
        to_summed_area_table(&mut table, 8, 0);

        let expected = [1, 3, 6, 10, 15, 21, 28, 36, 10, 22, 125, 329, 634, 1040, 1547, 2155];
        assert_eq!(table, expected);
    }
}
