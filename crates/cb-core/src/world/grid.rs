//! The one spatial index of the world's lookups (island-world design section 4): a square grid of
//! cells over the world, each holding the ids of the things that reach into it. Stored as one flat
//! list with a start per cell (compressed rows), so a 40 km world of mostly empty cells costs four
//! bytes a cell rather than an empty `Vec` each. Ids keep the order they were added in, so a lookup
//! never depends on anything but the data.

use glam::Vec2;

/// Ids bucketed by square cell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellIndex {
    origin_m: f32,
    cell_m: f32,
    side: usize,
    /// Where each cell's ids start in `ids`; one more entry than cells.
    starts: Vec<u32>,
    ids: Vec<u32>,
}

impl CellIndex {
    /// An index over the square from `origin_m` (x and z) `side` cells of `cell_m` across, holding
    /// each `(id, lo, hi)` in every cell its box `lo..hi` (x, z) touches. Boxes outside are clamped
    /// to the border cells.
    pub fn build(origin_m: f32, cell_m: f32, side: usize, boxes: &[(u32, Vec2, Vec2)]) -> Self {
        let mut index = Self {
            origin_m,
            cell_m,
            side: side.max(1),
            starts: Vec::new(),
            ids: Vec::new(),
        };
        let mut counts = vec![0u32; index.side * index.side + 1];
        for (_, lo, hi) in boxes {
            index.for_cells(*lo, *hi, |c| counts[c] += 1);
        }
        let mut at = 0u32;
        for c in counts.iter_mut() {
            let n = *c;
            *c = at;
            at += n;
        }
        let mut cursor = counts.clone();
        index.starts = counts;
        let mut ids = vec![0; at as usize];
        for (id, lo, hi) in boxes {
            index.for_cells(*lo, *hi, |c| {
                ids[cursor[c] as usize] = *id;
                cursor[c] += 1;
            });
        }
        index.ids = ids;
        index
    }

    fn cell(&self, v: f32) -> usize {
        let c = ((v - self.origin_m) / self.cell_m).floor();
        if c.is_nan() || c < 0.0 {
            0
        } else {
            (c as usize).min(self.side - 1)
        }
    }

    fn for_cells(&self, lo: Vec2, hi: Vec2, mut f: impl FnMut(usize)) {
        let (x0, x1) = (self.cell(lo.x), self.cell(hi.x));
        let (z0, z1) = (self.cell(lo.y), self.cell(hi.y));
        for z in z0..=z1 {
            for x in x0..=x1 {
                f(z * self.side + x);
            }
        }
    }

    /// The ids in the cell holding `p` (the border cell for a point outside), in the order added.
    pub fn at(&self, p: Vec2) -> &[u32] {
        if self.starts.is_empty() {
            return &[];
        }
        let c = self.cell(p.y) * self.side + self.cell(p.x);
        &self.ids[self.starts[c] as usize..self.starts[c + 1] as usize]
    }

    /// Every id in the cells the box `lo..hi` touches, ascending and without repeats.
    pub fn within(&self, lo: Vec2, hi: Vec2) -> Vec<u32> {
        let mut out = Vec::new();
        if self.starts.is_empty() {
            return out;
        }
        self.for_cells(lo, hi, |c| {
            out.extend_from_slice(&self.ids[self.starts[c] as usize..self.starts[c + 1] as usize]);
        });
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Bytes held, for the cost tables.
    pub fn bytes(&self) -> usize {
        (self.starts.len() + self.ids.len()) * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_found_in_every_cell_its_box_touches_and_nowhere_else() {
        let i = CellIndex::build(
            0.0,
            10.0,
            4,
            &[
                (7, Vec2::new(12.0, 12.0), Vec2::new(25.0, 13.0)),
                (3, Vec2::splat(1.0), Vec2::splat(2.0)),
            ],
        );
        assert_eq!(i.at(Vec2::new(15.0, 15.0)), &[7]);
        assert_eq!(i.at(Vec2::new(25.0, 15.0)), &[7]);
        assert_eq!(i.at(Vec2::new(35.0, 15.0)), &[] as &[u32]);
        assert_eq!(i.at(Vec2::new(5.0, 5.0)), &[3]);
        assert_eq!(i.within(Vec2::ZERO, Vec2::splat(39.0)), vec![3, 7]);
    }

    #[test]
    fn a_point_outside_reads_the_border_cell() {
        let i = CellIndex::build(0.0, 10.0, 4, &[(1, Vec2::new(31.0, 31.0), Vec2::new(32.0, 32.0))]);
        assert_eq!(i.at(Vec2::new(500.0, 900.0)), &[1]);
        assert_eq!(i.at(Vec2::new(f32::NAN, -5.0)), &[] as &[u32]);
    }
}
