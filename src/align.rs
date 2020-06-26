use std::cmp::max;
use std::collections::VecDeque;

const SUBSTITUTION_COST: usize = 1;
const DELETION_COST: usize = 1;
const INSERTION_COST: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    NoOp,
    Substitution,
    Deletion,
    Insertion,
}

use Operation::*;

/// Needleman-Wunsch / Wagner-Fischer table for computation of edit distance and associated
/// alignment.
#[derive(Clone)]
struct Cell {
    parent: usize,
    operation: Operation,
    cost: usize,
}

pub struct Alignment<'a> {
    pub x: Vec<&'a str>,
    pub y: Vec<&'a str>,
    table: Vec<Cell>,
    dim: [usize; 2],
}

impl<'a> Alignment<'a> {
    /// Fill table for Levenshtein distance / alignment computation
    pub fn new(x: Vec<&'a str>, y: Vec<&'a str>) -> Self {
        // TODO: Something about the alignment algorithm requires that the first two items in the
        // token stream are ["", " "]. In practice this means that the line must have a leading
        // space, and that the tokenization regex cooperates.
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
                cost: i,
            };
        }
        for j in 1..self.dim[0] {
            self.table[j * self.dim[1]] = Cell {
                parent: 0,
                operation: Insertion,
                cost: j,
            };
        }

        for (i, x_i) in self.x.iter().enumerate() {
            for (j, y_j) in self.y.iter().enumerate() {
                let (left, diag, up) =
                    (self.index(i, j + 1), self.index(i, j), self.index(i + 1, j));
                let candidates = [
                    Cell {
                        parent: left,
                        operation: Deletion,
                        cost: self.table[left].cost + DELETION_COST,
                    },
                    Cell {
                        parent: diag,
                        operation: if x_i == y_j { NoOp } else { Substitution },
                        cost: self.table[diag].cost
                            + if x_i == y_j { 0 } else { SUBSTITUTION_COST },
                    },
                    Cell {
                        parent: up,
                        operation: Insertion,
                        cost: self.table[up].cost + INSERTION_COST,
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
            Substitution => "*",
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
        let (before, after) = ("aaa", "aba");
        assert_string_distance_parts(before, after, (1, 3));
        assert_eq!(operations(before, after), vec![NoOp, Substitution, NoOp,]);
    }

    #[test]
    fn test_0_nonascii() {
        let (before, after) = ("ááb", "áaa");
        assert_string_distance_parts(before, after, (2, 3));
        assert_eq!(
            operations(before, after),
            vec![NoOp, Substitution, Substitution,]
        );
    }

    #[test]
    fn test_1() {
        let (before, after) = ("kitten", "sitting");
        assert_string_distance_parts(before, after, (3, 7));
        assert_eq!(
            operations(before, after),
            vec![
                Substitution, // K S
                NoOp,         // I I
                NoOp,         // T T
                NoOp,         // T T
                Substitution, // E I
                NoOp,         // N N
                Insertion     // - G
            ]
        );
    }

    #[test]
    fn test_2() {
        let (before, after) = ("saturday", "sunday");
        assert_string_distance_parts(before, after, (3, 8));
        assert_eq!(
            operations(before, after),
            vec![
                NoOp,         // S S
                Deletion,     // A -
                Deletion,     // T -
                NoOp,         // U U
                Substitution, // R N
                NoOp,         // D D
                NoOp,         // A A
                NoOp          // Y Y
            ]
        );
    }

    fn assert_string_distance_parts(s1: &str, s2: &str, parts: (usize, usize)) {
        let (numer, _) = parts;
        assert_string_levenshtein_distance(s1, s2, numer);
        assert_eq!(string_distance_parts(s1, s2), parts);
        assert_eq!(string_distance_parts(s2, s1), parts);
    }

    fn assert_string_levenshtein_distance(s1: &str, s2: &str, d: usize) {
        assert_eq!(string_levenshtein_distance(s1, s2), d);
        assert_eq!(string_levenshtein_distance(s2, s1), d);
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
