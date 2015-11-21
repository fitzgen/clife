use std::cmp;
use std::fs;
use std::io;
use std::path;
use std::sync::atomic;

use crossbeam;
use rand;

use std::io::BufRead;

use error;

pub type CellState = atomic::AtomicBool;

// #[derive(Clone)]
pub struct World {
    width: i32,
    height: i32,
    cells: Vec<CellState>,
}

impl World {
    pub fn new_empty(width: i32, height: i32) -> World {
        World {
            width: width,
            height: height,
            cells: (0..width * height)
                       .map(|_| atomic::AtomicBool::new(false))
                       .collect(),
        }
    }

    pub fn new_random(width: i32, height: i32) -> World {
        World {
            width: width,
            height: height,
            cells: (0..width * height)
                       .map(|_| atomic::AtomicBool::new(rand::random::<bool>()))
                       .collect(),
        }
    }

    pub fn from_parts(parts: Vec<Vec<bool>>) -> Result<World, error::Error> {
        let mut width: Option<i32> = None;
        let mut height = 0 as i32;
        let mut cells: Vec<CellState> = vec![];

        for row in parts {
            if let Some(w) = width {
                if w as usize != row.len() {
                    return Err(error::Error::WorldBadParts);
                }
            } else {
                if row.len() == 0 {
                    return Err(error::Error::WorldBadParts);
                }
                width = Some(row.len() as i32);
            }

            for c in row {
                cells.push(atomic::AtomicBool::new(c));
            }
            height += 1;
        }

        if let Some(w) = width {
            assert!(cells.len() != 0);
            assert!(height > 0);
            Ok(World {
                width: w,
                height: height,
                cells: cells,
            })
        } else {
            Err(error::Error::WorldBadParts)
        }
    }

    pub fn from_file(file_path: &path::Path) -> Result<World, error::Error> {
        let file = try!(fs::File::open(file_path));

        let mut parts: Vec<Vec<bool>> = vec![];
        for line in io::BufReader::new(file).lines() {
            let line = try!(line);
            parts.push(try!(line.chars()
                                .map(|c| {
                                    match c {
                                        '_' => Ok(false),
                                        '#' => Ok(true),
                                        _ => Err(error::Error::WorldBadParts),
                                    }
                                })
                                .collect()));
        }

        World::from_parts(parts)
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    fn number_of_live_neighbors(&self, x: i32, y: i32) -> u8 {
        assert!(x < self.width());
        assert!(y < self.height());

        let wrap_x = |x, delta| (x + self.width + delta) % self.width;
        let wrap_y = |y, delta| (y + self.height + delta) % self.height;

        let mut count = 0;
        if self.cell_is_alive(wrap_x(x, -1), wrap_y(y, -1)) {
            count += 1;
        }
        if self.cell_is_alive(wrap_x(x, -1), wrap_y(y, 0)) {
            count += 1;
        }
        if self.cell_is_alive(wrap_x(x, -1), wrap_y(y, 1)) {
            count += 1;
        }

        if self.cell_is_alive(wrap_x(x, 0), wrap_y(y, -1)) {
            count += 1;
        }
        if self.cell_is_alive(wrap_x(x, 0), wrap_y(y, 1)) {
            count += 1;
        }

        if self.cell_is_alive(wrap_x(x, 1), wrap_y(y, -1)) {
            count += 1;
        }
        if self.cell_is_alive(wrap_x(x, 1), wrap_y(y, 0)) {
            count += 1;
        }
        if self.cell_is_alive(wrap_x(x, 1), wrap_y(y, 1)) {
            count += 1;
        }

        count
    }

    fn cell_is_alive(&self, x: i32, y: i32) -> bool {
        self.cells[(y as usize * self.width as usize) + x as usize].load(atomic::Ordering::SeqCst)
    }

    pub fn become_next_step(&mut self, previous: &World) {
        assert!(self.width() == previous.width());
        assert!(self.height() == previous.height());

        let rows = self.rows();
        crossbeam::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for (y, row) in &rows {
                        for (x, cell) in row.iter().enumerate() {
                            let neighbors = previous.number_of_live_neighbors(x as i32, y as i32);
                            let alive_next = match (previous.cell_is_alive(x as i32, y as i32),
                                                    neighbors) {
                                (true, 2) | (_, 3) => true,
                                _ => false,
                            };
                            cell.store(alive_next, atomic::Ordering::SeqCst);
                        }
                    }
                });
            }
        });
    }
}

pub struct AtomicChunksIter<'a, T: 'a> {
    slice: &'a [T],
    step: usize,
    next: atomic::AtomicUsize,
}

impl<'a, T> AtomicChunksIter<'a, T> {
    pub fn new(slice: &'a [T], step: usize) -> AtomicChunksIter<'a, T> {
        AtomicChunksIter {
            slice: slice,
            step: step,
            next: atomic::AtomicUsize::new(0),
        }
    }

    unsafe fn next_internal(&self) -> Option<(usize, &'a [T])> {
        loop {
            let current = self.next.load(atomic::Ordering::SeqCst);
            if current == self.slice.len() {
                return None;
            }

            let end = cmp::min(current + self.step, self.slice.len());
            if self.next.compare_and_swap(current, end, atomic::Ordering::SeqCst) == current {
                return Some((current / self.step, &self.slice[current..end]));
            }
        }
    }
}

impl<'a, 'b, T> Iterator for &'b AtomicChunksIter<'a, T> {
    type Item = (usize, &'a [T]);
    fn next(&mut self) -> Option<Self::Item> {
        unsafe { self.next_internal() }
    }
}
impl World {
    pub fn rows<'a>(&'a self) -> AtomicChunksIter<'a, CellState> {
        AtomicChunksIter::new(&self.cells[..], self.width as usize)
    }
}
