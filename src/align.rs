use std::cmp::max;
use std::collections::VecDeque;

const DELETION_COST: usize = 2;
const INSERTION_COST: usize = 2;
// extra cost for starting a new group of changed tokens
const INITIAL_MISMATCH_PENALITY: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    NoOp,
    Deletion,
    Insertion,
}

use Operation::*;

/// Needleman-Wunsch / Wagner-Fischer table for computation of edit distance and associated
/// alignment.
#[derive(Clone, Debug)]
struct Cell {
    parent: usize,
    operation: Operation,
    cost: usize,
}

#[derive(Debug)]
pub struct Alignment<'a> {
    pub x: Vec<&'a str>,
    pub y: Vec<&'a str>,
    table: Vec<Cell>,
    dim: [usize; 2],
}

impl<'a> Alignment<'a> {
    /// Fill table for Levenshtein distance / alignment computation
    pub fn new(x: Vec<&'a str>, y: Vec<&'a str>) -> Self {
        // TODO: Something downstream of the alignment algorithm requires that the first token in
        // both x and y is "", so this is explicitly inserted in `tokenize()`.
        let dim = [y.len() + 1, x.len() + 1];
        let table = vec![
            Cell {
                parent: 0,
                operation: NoOp,
                cost: 0
            };
            dim[0] * dim[1]
        ];
        let mut alignment = Self { x, y, table, dim };
        alignment.fill();
        alignment
    }

    /// Fill table for Levenshtein distance / alignment computation
    pub fn fill(&mut self) {
        // x is written along the top of the table; y is written down the left side of the
        // table. Also, we insert a 0 in cell (0, 0) of the table, so x and y are shifted by one
        // position. Therefore, the element corresponding to (x[i], y[j]) is in column (i + 1) and
        // row (j + 1); the index of this element is given by index(i, j).
        for i in 1..self.dim[1] {
            self.table[i] = Cell {
                parent: 0,
                operation: Deletion,
                cost: i * DELETION_COST + INITIAL_MISMATCH_PENALITY,
            };
        }
        for j in 1..self.dim[0] {
            self.table[j * self.dim[1]] = Cell {
                parent: 0,
                operation: Insertion,
                cost: j * INSERTION_COST + INITIAL_MISMATCH_PENALITY,
            };
        }

        for (i, x_i) in self.x.iter().enumerate() {
            for (j, y_j) in self.y.iter().enumerate() {
                let (left, diag, up) =
                    (self.index(i, j + 1), self.index(i, j), self.index(i + 1, j));
                // The order of the candidates matters if two of them have the
                // same cost as in that case we choose the first one. Consider
                // insertions and deletions before matches in order to group
                // changes together. Insertions are preferred to deletions in
                // order to highlight moved tokens as a deletion followed by an
                // insertion (as the edit sequence is read backwards we need to
                // choose the insertion first)
                let candidates = [
                    Cell {
                        parent: up,
                        operation: Insertion,
                        cost: self.mismatch_cost(up, INSERTION_COST),
                    },
                    Cell {
                        parent: left,
                        operation: Deletion,
                        cost: self.mismatch_cost(left, DELETION_COST),
                    },
                    Cell {
                        parent: diag,
                        operation: NoOp,
                        cost: if x_i == y_j {
                            self.table[diag].cost
                        } else {
                            usize::MAX
                        },
                    },
                ];
                let index = self.index(i + 1, j + 1);
                self.table[index] = candidates
                    .iter()
                    .min_by_key(|cell| cell.cost)
                    .unwrap()
                    .clone();
            }
        }
    }

    fn mismatch_cost(&self, parent: usize, basic_cost: usize) -> usize {
        self.table[parent].cost
            + basic_cost
            + if self.table[parent].operation == NoOp {
                INITIAL_MISMATCH_PENALITY
            } else {
                0
            }
    }

    /// Read edit operations from the table.
    pub fn operations(&self) -> Vec<Operation> {
        let mut ops = VecDeque::with_capacity(max(self.x.len(), self.y.len()));
        let mut cell = &self.table[self.index(self.x.len(), self.y.len())];
        loop {
            ops.push_front(cell.operation);
            if cell.parent == 0 {
                break;
            }
            cell = &self.table[cell.parent];
        }
        Vec::from(ops)
    }

    pub fn coalesced_operations(&self) -> Vec<(Operation, usize)> {
        run_length_encode(self.operations())
    }

    /// Compute custom distance metric from the filled table. The distance metric is
    ///
    /// (total length of edits) / (total length of longer string)
    ///
    /// where length is measured in number of unicode grapheme clusters.
    #[allow(dead_code)]
    pub fn distance(&self) -> f64 {
        let (numer, denom) = self.distance_parts();
        (numer as f64) / (denom as f64)
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn levenshtein_distance(&self) -> usize {
        self.table[self.index(self.x.len(), self.y.len())].cost
    }

    // Row-major storage of 2D array.
    fn index(&self, i: usize, j: usize) -> usize {
        j * self.dim[1] + i
    }

    #[allow(dead_code)]
    fn format_cell(&self, cell: &Cell) -> String {
        let parent = &self.table[cell.parent];
        let op = match cell.operation {
            Deletion => "-",
            Insertion => "+",
            NoOp => ".",
        };
        format!("{}{}{}", parent.cost, op, cell.cost)
    }

    #[allow(dead_code)]
    fn print(&self) {
        println!("x: {:?}", self.x);
        println!("y: {:?}", self.y);
        println!();
        print!("      ");
        for j in 0..self.dim[1] {
            print!("{}     ", if j > 0 { self.x[j - 1] } else { " " })
        }
        println!();

        for i in 0..self.dim[0] {
            for j in 0..self.dim[1] {
                if j == 0 {
                    print!("{}     ", if i > 0 { self.y[i - 1] } else { " " })
                }
                let cell = &self.table[self.index(j, i)];
                print!("{}   ", self.format_cell(cell));
            }
            println!();
        }
        println!();
    }
}

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
        pub fn run(&self) -> () {
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
}
