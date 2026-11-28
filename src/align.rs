#[cfg(test)]
use std::cmp::max;

const DELETION_COST: u32 = 2;
const INSERTION_COST: u32 = 2;
// extra cost for starting a new group of changed tokens
const INITIAL_MISMATCH_PENALTY: u32 = 1;

const OP_NOOP: u8 = 0;
const OP_DELETION: u8 = 1;
const OP_INSERTION: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    NoOp,
    Deletion,
    Insertion,
}

use Operation::*;

#[inline]
fn decode_op(op: u8) -> Operation {
    match op {
        OP_NOOP => NoOp,
        OP_DELETION => Deletion,
        _ => Insertion,
    }
}

/// Reusable scratch buffers for Needleman-Wunsch / Wagner-Fischer alignment.
#[derive(Debug, Default)]
pub struct AlignWorkspace {
    ops: Vec<u8>,
    prev_cost: Vec<u32>,
    curr_cost: Vec<u32>,
    rle_buf: Vec<(Operation, usize)>,
}

impl AlignWorkspace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fill the row-major 1-byte-per-cell operation table and return coalesced
    /// (run-length encoded) edit operations.
    pub fn coalesced_operations_for<T: PartialEq>(
        &mut self,
        x: &[T],
        y: &[T],
    ) -> &[(Operation, usize)] {
        let _ = self.fill_table(x, y);
        let cols = x.len() + 1;

        // Trace back directly into run-length encoded operations without intermediate VecDeque.
        self.rle_buf.clear();
        let mut col = x.len();
        let mut row = y.len();
        let mut cur_op = self.ops[row * cols + col];
        let mut cur_count = 0usize;

        loop {
            let op = self.ops[row * cols + col];
            if op == cur_op {
                cur_count += 1;
            } else {
                self.rle_buf.push((decode_op(cur_op), cur_count));
                cur_op = op;
                cur_count = 1;
            }
            if col == 0 || row == 0 {
                break;
            }
            match op {
                OP_INSERTION => row -= 1,
                OP_DELETION => col -= 1,
                _ => {
                    col -= 1;
                    row -= 1;
                }
            }
            if col == 0 && row == 0 {
                break;
            }
        }
        if cur_count > 0 {
            self.rle_buf.push((decode_op(cur_op), cur_count));
        }
        self.rle_buf.reverse();
        &self.rle_buf
    }

    fn fill_table<T: PartialEq>(&mut self, x: &[T], y: &[T]) -> usize {
        let cols = x.len() + 1;
        let rows = y.len() + 1;
        let total = rows * cols;

        if self.ops.len() < total {
            self.ops.resize(total, 0);
        }
        if self.prev_cost.len() < cols {
            self.prev_cost.resize(cols, 0);
            self.curr_cost.resize(cols, 0);
        }

        // Row 0: (0, 0) is NoOp with cost 0; (i, 0) for i >= 1 is Deletion with parent 0.
        self.ops[0] = OP_NOOP;
        self.prev_cost[0] = 0;
        for i in 1..cols {
            self.ops[i] = OP_DELETION;
            self.prev_cost[i] = i as u32 * DELETION_COST + INITIAL_MISMATCH_PENALTY;
        }

        // Row-major fill: each cell only needs the previous cost row, the previous op row,
        // and the cell immediately to its left (kept in registers).
        for (j, y_j) in y.iter().enumerate() {
            let prev_row_start = j * cols;
            let (prev_ops, curr_ops_slice) =
                self.ops[prev_row_start..prev_row_start + 2 * cols].split_at_mut(cols);
            let curr_ops = &mut curr_ops_slice[..cols];
            let prev_cost = &self.prev_cost[..cols];
            let curr_cost = &mut self.curr_cost[..cols];

            let mut left_cost = (j as u32 + 1) * INSERTION_COST + INITIAL_MISMATCH_PENALTY;
            let mut left_op = OP_INSERTION;
            curr_cost[0] = left_cost;
            curr_ops[0] = OP_INSERTION;

            for (i, x_i) in x.iter().enumerate() {
                let col = i + 1;
                let up_cost = prev_cost[col];
                let up_op = prev_ops[col];
                let ins_cost = up_cost + INSERTION_COST + u32::from(up_op == OP_NOOP);
                let del_cost = left_cost + DELETION_COST + u32::from(left_op == OP_NOOP);

                // Tie-break order matches original: Insertion, then Deletion, then NoOp.
                let mut best_cost = ins_cost;
                let mut best_op = OP_INSERTION;
                if del_cost < best_cost {
                    best_cost = del_cost;
                    best_op = OP_DELETION;
                }
                if x_i == y_j {
                    let diag_cost = prev_cost[i];
                    if diag_cost < best_cost {
                        best_cost = diag_cost;
                        best_op = OP_NOOP;
                    }
                }

                curr_cost[col] = best_cost;
                curr_ops[col] = best_op;
                left_cost = best_cost;
                left_op = best_op;
            }

            std::mem::swap(&mut self.prev_cost, &mut self.curr_cost);
        }

        self.prev_cost[x.len()] as usize
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct Alignment<'a> {
    pub x: Vec<&'a str>,
    pub y: Vec<&'a str>,
    ops: Vec<u8>,
    dim: [usize; 2],
    last_cost: usize,
}

#[cfg(test)]
impl<'a> Alignment<'a> {
    /// Fill table for Levenshtein distance / alignment computation
    pub fn new(x: Vec<&'a str>, y: Vec<&'a str>) -> Self {
        let dim = [y.len() + 1, x.len() + 1];
        let mut alignment = Self {
            x,
            y,
            ops: vec![0; dim[0] * dim[1]],
            dim,
            last_cost: 0,
        };
        alignment.fill();
        alignment
    }

    /// Fill table for Levenshtein distance / alignment computation
    pub fn fill(&mut self) {
        let mut ws = AlignWorkspace {
            ops: std::mem::take(&mut self.ops),
            prev_cost: Vec::new(),
            curr_cost: Vec::new(),
            rle_buf: Vec::new(),
        };
        self.last_cost = ws.fill_table(&self.x, &self.y);
        self.ops = ws.ops;
    }

    /// Read edit operations from the table.
    pub fn operations(&self) -> Vec<Operation> {
        let cols = self.dim[1];
        let mut ops = Vec::with_capacity(max(self.x.len(), self.y.len()) + 1);
        let mut col = self.x.len();
        let mut row = self.y.len();
        loop {
            let op = self.ops[row * cols + col];
            ops.push(decode_op(op));
            if col == 0 || row == 0 {
                break;
            }
            match op {
                OP_INSERTION => row -= 1,
                OP_DELETION => col -= 1,
                _ => {
                    col -= 1;
                    row -= 1;
                }
            }
            if col == 0 && row == 0 {
                break;
            }
        }
        ops.reverse();
        ops
    }

    #[allow(dead_code)]
    pub fn coalesced_operations(&self) -> Vec<(Operation, usize)> {
        run_length_encode(self.operations())
    }
}

#[cfg(test)]
fn run_length_encode<T>(sequence: Vec<T>) -> Vec<(T, usize)>
where
    T: Copy,
    T: PartialEq,
{
    let mut encoded = Vec::with_capacity(sequence.len());

    if sequence.is_empty() {
        return encoded;
    }

    let end = sequence.len();
    let (mut i, mut j) = (0, 1);
    let mut curr = &sequence[i];
    loop {
        if j == end || sequence[j] != *curr {
            encoded.push((*curr, j - i));
            if j == end {
                return encoded;
            } else {
                curr = &sequence[j];
                i = j;
            }
        }
        j += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn test_run_length_encode() {
        assert_eq!(run_length_encode::<usize>(vec![]), vec![]);
        assert_eq!(run_length_encode(vec![0]), vec![(0, 1)]);
        assert_eq!(run_length_encode(vec!["0", "0"]), vec![("0", 2)]);
        assert_eq!(
            run_length_encode(vec![0, 0, 1, 2, 2, 2, 3, 4, 4, 4]),
            vec![(0, 2), (1, 1), (2, 3), (3, 1), (4, 3)]
        );
    }

    #[test]
    fn test_0() {
        TestCase {
            before: "aaa",
            after: "aba",
            distance: 5,
            parts: (2, 4),
            operations: vec![NoOp, Deletion, Insertion, NoOp],
        }
        .run();
    }

    #[test]
    fn test_0_nonascii() {
        TestCase {
            before: "ááb",
            after: "áaa",
            distance: 9,
            parts: (4, 5),
            operations: vec![NoOp, Deletion, Deletion, Insertion, Insertion],
        }
        .run();
    }

    #[test]
    fn test_1() {
        TestCase {
            before: "kitten",
            after: "sitting",
            distance: 13,
            parts: (5, 9),
            operations: vec![
                Deletion,  // K -
                Insertion, // - S
                NoOp,      // I I
                NoOp,      // T T
                NoOp,      // T T
                Deletion,  // E -
                Insertion, // - I
                NoOp,      // N N
                Insertion, // - G
            ],
        }
        .run();
    }

    #[test]
    fn test_2() {
        TestCase {
            before: "saturday",
            after: "sunday",
            distance: 10,
            parts: (4, 9),
            operations: vec![
                NoOp,      // S S
                Deletion,  // A -
                Deletion,  // T -
                NoOp,      // U U
                Deletion,  // R -
                Insertion, // - N
                NoOp,      // D D
                NoOp,      // A A
                NoOp,      // Y Y
            ],
        }
        .run();
    }

    #[test]
    fn test_3() {
        TestCase {
            // Prefer [Deletion NoOp Insertion] over [Insertion NoOp Deletion]
            before: "ab",
            after: "ba",
            distance: 6,
            parts: (2, 3),
            operations: vec![
                Deletion,  // a -
                NoOp,      // b b
                Insertion, // - a
            ],
        }
        .run();
    }

    #[test]
    fn test_4() {
        // Deletions are grouped together.
        TestCase {
            before: "AABB",
            after: "AB",
            distance: 5,
            parts: (2, 4),
            operations: vec![
                NoOp,     // A A
                Deletion, // A -
                Deletion, // B -
                NoOp,     // B B
            ],
        }
        .run();
    }

    #[test]
    fn test_5() {
        // Insertions are grouped together.
        TestCase {
            before: "AB",
            after: "AABB",
            distance: 5,
            parts: (2, 4),
            operations: vec![
                NoOp,      // A A
                Insertion, // - A
                Insertion, // - B
                NoOp,      // B B
            ],
        }
        .run();
    }

    #[test]
    fn test_6() {
        // Insertion and Deletion are grouped together.
        TestCase {
            before: "AAABBB",
            after: "ACB",
            distance: 11,
            parts: (5, 7),
            operations: vec![
                NoOp,      // A A
                Deletion,  // A -
                Deletion,  // A -
                Deletion,  // B -
                Deletion,  // B -
                Insertion, // - C
                NoOp,      // B B
            ],
        }
        .run();
    }

    struct TestCase<'a> {
        before: &'a str,
        after: &'a str,
        distance: usize,
        parts: (usize, usize),
        operations: Vec<Operation>,
    }

    impl<'a> TestCase<'a> {
        pub fn run(&self) {
            self.assert_string_distance_parts();
            assert_eq!(operations(self.before, self.after), self.operations);
        }

        fn assert_string_distance_parts(&self) {
            self.assert_string_levenshtein_distance();
            assert_eq!(string_distance_parts(self.before, self.after), self.parts);
            assert_eq!(string_distance_parts(self.after, self.before), self.parts);
        }

        fn assert_string_levenshtein_distance(&self) {
            assert_eq!(
                string_levenshtein_distance(self.before, self.after),
                self.distance
            );
            assert_eq!(
                string_levenshtein_distance(self.after, self.before),
                self.distance
            );
        }
    }

    fn string_distance_parts(x: &str, y: &str) -> (usize, usize) {
        let (x, y) = (
            x.graphemes(true).collect::<Vec<&str>>(),
            y.graphemes(true).collect::<Vec<&str>>(),
        );
        Alignment::new(x, y).distance_parts()
    }

    fn string_levenshtein_distance(x: &str, y: &str) -> usize {
        let (x, y) = (
            x.graphemes(true).collect::<Vec<&str>>(),
            y.graphemes(true).collect::<Vec<&str>>(),
        );
        Alignment::new(x, y).levenshtein_distance()
    }

    fn operations<'a>(x: &'a str, y: &'a str) -> Vec<Operation> {
        let (x, y) = (
            x.graphemes(true).collect::<Vec<&str>>(),
            y.graphemes(true).collect::<Vec<&str>>(),
        );
        Alignment::new(x, y).operations()
    }

    impl<'a> Alignment<'a> {
        pub fn distance_parts(&self) -> (usize, usize) {
            let (mut numer, mut denom) = (0, 0);
            for op in self.operations() {
                if op != NoOp {
                    numer += 1;
                }
                denom += 1;
            }
            (numer, denom)
        }

        /// Compute levenshtein distance from the filled table.
        pub fn levenshtein_distance(&self) -> usize {
            self.last_cost
        }
    }
}
