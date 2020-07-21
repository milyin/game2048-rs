use ndarray::Array2;
use rand::Rng;
use JoinResult::{TileMerged, TileMoved};
use Origin::{Appear, Hold, Merged, Moved};
use Side::{Down, Left, Right, Up};

#[derive(Copy, Clone, Debug)]
pub enum Side {
    Down,
    Left,
    Up,
    Right,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Origin {
    Appear,
    Hold(usize, usize),
    Moved(usize, usize),
    Merged((usize, usize), (usize, usize)),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Tile(u32, Origin);

impl From<Tile> for u32 {
    fn from(tile: Tile) -> u32 {
        1 << tile.0
    }
}

impl Origin {
    fn hold(arr_index: (usize, usize)) -> Self {
        Hold(arr_index.1, arr_index.0)
    }
}
enum JoinResult {
    TileMoved,
    TileMerged,
}

fn join_tiles(dst: &mut Option<Tile>, src: &mut Option<Tile>) -> Option<JoinResult> {
    match (*dst, *src) {
        (None, Some(Tile(level, Hold(x, y)))) | (None, Some(Tile(level, Moved(x, y)))) => {
            *dst = Some(Tile(level, Moved(x, y)));
            *src = None;
            Some(TileMoved)
        }
        (None, Some(Tile(level, Merged(a, b)))) => {
            *dst = Some(Tile(level, Merged(a, b)));
            *src = None;
            Some(TileMoved)
        }
        (Some(Tile(ld, Hold(xd, yd))), Some(Tile(ls, Hold(xs, ys))))
        | (Some(Tile(ld, Hold(xd, yd))), Some(Tile(ls, Moved(xs, ys))))
        | (Some(Tile(ld, Moved(xd, yd))), Some(Tile(ls, Hold(xs, ys))))
        | (Some(Tile(ld, Moved(xd, yd))), Some(Tile(ls, Moved(xs, ys)))) => {
            if ld == ls {
                *dst = Some(Tile(ld + 1, Merged((xd, yd), (xs, ys))));
                *src = None;
                Some(TileMerged)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
pub struct Field(Array2<Option<Tile>>);

impl Field {
    pub fn new(width: usize, height: usize) -> Self {
        Self(Array2::default((height, width)))
    }
    pub fn from_array(array: Array2<u32>) -> Self {
        let (h, w) = (array.shape()[0], array.shape()[1]);
        let mut field = Self::new(w, h);
        for (index, v) in array.indexed_iter() {
            if let Some(p) = field.0.get_mut(index) {
                *p = match v {
                    0 => None,
                    v if v.count_ones() == 1 => Some(Tile(v.trailing_zeros(), Origin::hold(index))),
                    _ => panic!("Expected values 0,1,2,4,8,16..."),
                }
            };
        }
        field
    }
    pub fn into_array(&self) -> Array2<u32> {
        let (h, w) = (self.0.shape()[0], self.0.shape()[1]);
        Array2::from_shape_fn((h, w), |index| {
            if let Some(Tile(level, _)) = self.0.get(index).unwrap() {
                1 << *level
            } else {
                0
            }
        })
    }
    fn width_from_side(&self, side: Side) -> usize {
        match side {
            Up | Down => self.0.shape()[1],
            Left | Right => self.0.shape()[0],
        }
    }
    fn height_from_side(&self, side: Side) -> usize {
        match side {
            Up | Down => self.0.shape()[0],
            Left | Right => self.0.shape()[1],
        }
    }
    pub fn width(&self) -> usize {
        self.width_from_side(Up)
    }
    pub fn height(&self) -> usize {
        self.height_from_side(Up)
    }
    fn index_from_side(&self, side: Side, x: usize, y: usize) -> (usize, usize) {
        match side {
            Up => (y, x),
            Down => (self.height() - 1 - y, self.width() - 1 - x),
            Left => (self.height() - 1 - x, y),
            Right => (x, self.width() - 1 - y),
        }
    }
    fn get_from_side(&self, side: Side, x: usize, y: usize) -> Option<Tile> {
        *self.0.get(self.index_from_side(side, x, y)).unwrap()
    }
    fn put_from_side(&mut self, side: Side, x: usize, y: usize, tile: Option<Tile>) {
        *self.0.get_mut(self.index_from_side(side, x, y)).unwrap() = tile;
    }
    pub fn get(&self, x: usize, y: usize) -> Option<Tile> {
        self.get_from_side(Up, x, y)
    }

    pub fn swipe(&mut self, side: Side) {
        let width = self.width_from_side(side);
        let height = self.height_from_side(side);

        for x in 0..width {
            for y in 0..height {
                if let Some(tile) = self.get_from_side(side, x, y) {
                    self.put_from_side(
                        side,
                        x,
                        y,
                        Some(Tile {
                            1: Hold(x, y),
                            ..tile
                        }),
                    );
                }
            }
        }

        dbg!(&self);

        for x in 0..width {
            let mut repeat_start_from = Some(0);
            while repeat_start_from.is_some() {
                let start_from = repeat_start_from.unwrap();
                repeat_start_from = None;
                for y in start_from..height - 1 {
                    let mut up = self.get_from_side(side, x, y);
                    let mut down = self.get_from_side(side, x, y + 1);

                    if let Some(join_result) = join_tiles(&mut up, &mut down) {
                        self.put_from_side(side, x, y, up);
                        self.put_from_side(side, x, y + 1, down);
                        if repeat_start_from.is_none() {
                            match join_result {
                                TileMerged => {
                                    repeat_start_from = Some(y + 1);
                                }
                                TileMoved => repeat_start_from = Some(y),
                            }
                        }
                    }
                }
            }
            let mut repeat_start_from = Some(0);
            while repeat_start_from.is_some() {
                let start_from = repeat_start_from.unwrap();
                repeat_start_from = None;
                for y in start_from..height - 1 {
                    let mut up = self.get_from_side(side, x, y);
                    let mut down = self.get_from_side(side, x, y + 1);
                    if let Some(TileMoved) = join_tiles(&mut up, &mut down) {
                        self.put_from_side(side, x, y, up);
                        self.put_from_side(side, x, y + 1, down);
                        repeat_start_from = Some(0)
                    }
                }
            }
        }
    }
    pub fn get_free_cells(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        let width = self.width_from_side(Down);
        let height = self.height_from_side(Down);
        for x in 0..width {
            for y in 0..height {
                if self.get_from_side(Down, x, y).is_none() {
                    result.push((x, y));
                }
            }
        }
        result
    }

    pub fn append_tile(&mut self) -> bool {
        let mut rng = rand::thread_rng();
        let poses = self.get_free_cells();
        if poses.is_empty() {
            return false;
        }
        let (x, y) = poses[rng.gen_range(0, poses.len())];
        let v = rng.gen_range(1, 3);
        self.put_from_side(Down, x, y, Some(Tile(v, Appear)));
        return true;
    }
}

#[cfg(test)]
mod tests {
    use crate::Origin::{Hold, Merged /*, Moved*/};
    use crate::Tile;

    pub fn hold(level: u32, x: usize, y: usize) -> Option<Tile> {
        Some(Tile(level, Hold(x, y)))
    }
    //pub fn moved(level: u32, x: usize, y: usize) -> Option<Tile> {
    //    Some(Tile(level, Moved(x, y)))
    //}
    pub fn merged(level: u32, a: (usize, usize), b: (usize, usize)) -> Option<Tile> {
        Some(Tile(level, Merged(a, b)))
    }
}

#[test]
fn field_widht_height_at() {
    use crate::tests::hold;
    use ndarray::arr2;
    let field = Field(arr2(&[
        [hold(0, 0, 0), hold(10, 1, 0), hold(20, 2, 0)],
        [hold(1, 0, 1), hold(11, 1, 1), hold(21, 2, 1)],
        [hold(2, 0, 2), hold(12, 1, 2), hold(22, 2, 2)],
        [hold(3, 0, 3), hold(13, 1, 3), hold(23, 2, 3)],
    ]));
    assert_eq!(field.width(), 3);
    assert_eq!(field.height(), 4);
    assert_eq!(field.get(2, 1).unwrap().0, 21);
    assert_eq!(field.get(0, 2).unwrap().0, 2);
    assert_eq!(field.get(2, 3).unwrap().0, 23);
    assert_eq!(field.get_from_side(Up, 0, 0).unwrap().0, 0);
    assert_eq!(field.get_from_side(Down, 0, 0).unwrap().0, 23);
    assert_eq!(field.get_from_side(Left, 0, 0).unwrap().0, 3);
    assert_eq!(field.get_from_side(Right, 0, 0).unwrap().0, 20);
    assert_eq!(field.get_from_side(Up, 1, 2).unwrap().0, 12);
    assert_eq!(field.get_from_side(Down, 1, 2).unwrap().0, 11);
    assert_eq!(field.get_from_side(Left, 1, 2).unwrap().0, 22);
    assert_eq!(field.get_from_side(Right, 1, 2).unwrap().0, 1);
}

#[test]
fn field_from_array() {
    use crate::tests::hold;
    use ndarray::arr2;
    #[rustfmt::skip]
    let array = Array2::from_shape_vec((4, 3), vec![
        8, 4, 2, 
        4, 2, 1, 
        2, 1, 0,
        1, 0, 16
    ]);
    let field = Field::from_array(array.unwrap());

    let expected = arr2(&[
        [hold(3, 0, 0), hold(2, 1, 0), hold(1, 2, 0)],
        [hold(2, 0, 1), hold(1, 1, 1), hold(0, 2, 1)],
        [hold(1, 0, 2), hold(0, 1, 2), None],
        [hold(0, 0, 3), None, hold(4, 2, 3)],
    ]);

    assert_eq!(field.0, expected);
}

#[test]
fn field_into_array() {
    use crate::tests::hold;
    use ndarray::arr2;
    let source = arr2(&[
        [hold(3, 0, 0), hold(2, 0, 0), hold(1, 0, 0)],
        [hold(2, 0, 0), hold(1, 0, 0), hold(0, 0, 0)],
        [hold(1, 0, 0), hold(0, 0, 0), None],
        [hold(0, 0, 0), None, hold(4, 0, 0)],
    ]);
    let array = Field(source).into_array();
    #[rustfmt::skip]
    let expected = Array2::from_shape_vec((4, 3), vec![
        8, 4, 2,
        4, 2, 1,
        2, 1, 0,
        1, 0, 16
    ]).unwrap();
    assert_eq!(array, expected);
}

#[test]
fn swipe_up() {
    use crate::tests::hold;
    use crate::tests::merged;
    use ndarray::arr2;
    #[rustfmt::skip]
    let array = Array2::from_shape_vec((4, 4), vec![
        0, 2, 4, 4,
        0, 2, 2, 4,
        0, 0, 2, 2,
        0, 0, 0, 2
    ]).unwrap();
    let mut field = Field::from_array(array);
    #[rustfmt::skip]
    let expected = Array2::from_shape_vec((4, 4), vec![
        0, 4, 4, 8,
        0, 0, 4, 4,
        0, 0, 0, 0,
        0, 0, 0, 0
    ]).unwrap();
    let expected_field = arr2(&[
        [
            None,
            merged(2, (1, 0), (1, 1)),
            hold(2, 2, 0),
            merged(3, (3, 0), (3, 1)),
        ],
        [
            None,
            None,
            merged(2, (2, 1), (2, 2)),
            merged(2, (3, 2), (3, 3)),
        ],
        [None, None, None, None],
        [None, None, None, None],
    ]);
    field.swipe(Up);
    assert_eq!(field.into_array(), expected);
    assert_eq!(field.0, expected_field);
}

#[test]
fn swipe_down() {
    #[rustfmt::skip]
    let array = Array2::from_shape_vec((4, 4), vec![
        0, 2, 4, 4,
        0, 2, 2, 4,
        0, 0, 2, 2,
        0, 0, 0, 2
    ]).unwrap();
    let mut field = Field::from_array(array);
    #[rustfmt::skip]
    let expected = Array2::from_shape_vec((4, 4), vec![
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 4, 8,
        0, 4, 4, 4,
    ]).unwrap();
    field.swipe(Down);
    assert_eq!(field.into_array(), expected);
}

#[test]
fn swipe_left() {
    #[rustfmt::skip]
    let array = Array2::from_shape_vec((4, 4), vec![
        0, 2, 4, 4,
        0, 2, 2, 4,
        0, 0, 2, 2,
        0, 0, 0, 2
    ]).unwrap();
    let mut field = Field::from_array(array);
    #[rustfmt::skip]
    let expected = Array2::from_shape_vec((4, 4), vec![
        2, 8, 0, 0,
        4, 4, 0, 0,
        4, 0, 0, 0,
        2, 0, 0, 0,
    ]).unwrap();
    field.swipe(Left);
    assert_eq!(field.into_array(), expected);
}

#[test]
fn swipe_right() {
    #[rustfmt::skip]
        let array = Array2::from_shape_vec((4, 4), vec![
        0, 2, 4, 4,
        0, 2, 2, 4,
        0, 0, 2, 2,
        0, 0, 0, 2
    ]).unwrap();
    let mut field = Field::from_array(array);
    #[rustfmt::skip]
    let expected = Array2::from_shape_vec((4, 4), vec![
        0, 0, 2, 8,
        0, 0, 4, 4,
        0, 0, 0, 4,
        0, 0, 0, 2
    ]).unwrap();
    field.swipe(Right);
    assert_eq!(field.into_array(), expected);
}
