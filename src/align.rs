use std::cmp::min;

use unicode_segmentation::UnicodeSegmentation;

/// Needleman-Wunsch / Wagner-Fischer table for computation of edit distance and associated
/// alignment.
pub struct Alignment<'a> {
    pub xx: Vec<(usize, &'a str)>,
    pub yy: Vec<(usize, &'a str)>,
    table: Vec<usize>,
    path: Vec<usize>,
    dim: [usize; 2],
}

impl<'a> Alignment<'a> {
    /// Fill table for Levenshtein distance / alignment computation
    pub fn new(xx: Vec<(usize, &'a str)>, yy: Vec<(usize, &'a str)>) -> Self {
        let dim = [yy.len() + 1, xx.len() + 1];
        let table = vec![0; dim[0] * dim[1]];
        let path = vec![0; dim[0] * dim[1]];
        let mut alignment = Self {
            xx,
            yy,
            table,
            path,
            dim,
        };
        alignment.fill();
        alignment
    }

    // Row-major storage of 2D array.
    fn index(&self, i: usize, j: usize) -> usize {
        j * self.dim[1] + i
    }

    fn reverse_index(&self, n: usize) -> (usize, usize) {
        (n % self.dim[1], n / self.dim[1])
    }

    fn cell(&self, i: usize, j: usize) -> usize {
        self.table[self.index(i, j)]
    }

    #[allow(dead_code)]
    fn print(&self) {
        for i in 0..self.dim[0] {
            for j in 0..self.dim[1] {
                print!("{} ", self.cell(j, i));
            }
            println!("");
        }
    }

    /// Fill table for Levenshtein distance / alignment computation
    pub fn fill(&mut self) {
        // xx is written along the top of the table; yy is written down the left side of the
        // table. Also, we insert a 0 in cell (0, 0) of the table, so xx and yy are shifted by one
        // position. Therefore, the element corresponding to (xx[i], yy[j]) is in column (i + 1) and row
        // (j + 1); the index of this element is given by index(i, j).
        for i in 1..self.dim[1] {
            self.table[i] = i;
        }
        for j in 1..self.dim[0] {
            self.table[j * self.dim[1]] = j;
        }
        let (xx, yy) = (&self.xx, &self.yy);
        for (i, (_, x)) in (1..=xx.len()).zip(xx) {
            for (j, (_, y)) in (1..=yy.len()).zip(yy) {
                let substitution_cost = self.cell(i - 1, j - 1) + if x == y { 0 } else { 1 };
                let deletion_cost = self.cell(i - 1, j) + 1;
                let insertion_cost = self.cell(i, j - 1) + 1;
                let index = self.index(i, j);
                self.table[index] = min(substitution_cost, min(deletion_cost, insertion_cost));
                if self.table[index] == substitution_cost {
                    self.path[index] = self.index(i - 1, j - 1)
                } else if self.table[index] == deletion_cost {
                    self.path[index] = self.index(i - 1, j)
                } else {
                    self.path[index] = self.index(i, j - 1)
                }
            }
        }
    }

    /// Read edit operations from the table.
    pub fn edit_operations<EditOperation>(
        &self,
        noop: EditOperation,
        substitution: EditOperation,
        deletion: EditOperation,
        insertion: EditOperation,
        forwards: bool,
    ) -> Vec<(EditOperation, (usize, &'a str))>
    where
        EditOperation: Copy,
        EditOperation: PartialEq,
    {
        let mut ops = Vec::new();
        let (mut i, mut j) = (self.xx.len(), self.yy.len());

        while i > 0 && j > 0 {
            let x = self.xx[i - 1];
            let y = self.yy[j - 1];
            let (_i, _j) = self.reverse_index(self.path[self.index(i, j)]);

            let op = if (_i, _j) == (i - 1, j) {
                deletion
            } else if x.1 == y.1 && (_i, _j) == (i - 1, j - 1) {
                noop
            } else if x.1 != y.1 && (_i, _j) == (i - 1, j - 1) {
                substitution
            } else {
                insertion
            };
            ops.push((op, if forwards { x } else { y }));
            i = _i;
            j = _j;
        }
        ops.into_iter().rev().collect()
    }

    /// Compute custom distance metric from the filled table. The distance metric is
    ///
    /// (total length of edits) / (total length of longer string)
    ///
    /// where length is measured in number of unicode grapheme clusters.
    pub fn distance(&self) -> f64 {
        let (numer, denom) = self.distance_parts();
        (numer as f64) / (denom as f64)
    }

    pub fn distance_parts(&self) -> (usize, usize) {
        let noop = 0;
        let (mut numer, mut denom) = (0, 0);
        for (op, (_, s)) in self.edit_operations(0, 1, 1, 1, true) {
            let n = s.trim().graphemes(true).count();
            if op != noop {
                numer += n;
            }
            denom += n;
        }
        (numer, denom)
    }

    /// Compute levenshtein distance from the filled table.
    #[allow(dead_code)]
    pub fn levenshtein_distance(&self) -> usize {
        self.table[self.table.len() - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Debug, PartialEq)]
    enum EditOperation {
        NoOp,
        Substitution,
        Deletion,
        Insertion,
    }

    use EditOperation::*;

    #[test]
    fn test_0() {
        let (before, after) = ("aaa", "aba");
        assert_string_distance_parts(before, after, (1, 3));
        assert_eq!(
            edit_operations(before, after),
            vec![NoOp, Substitution, NoOp,]
        );
    }

    #[test]
    fn test_0_nonascii() {
        let (before, after) = ("ááb", "áaa");
        assert_string_distance_parts(before, after, (2, 3));
        assert_eq!(
            edit_operations(before, after),
            vec![NoOp, Substitution, Substitution,]
        );
    }

    #[test]
    fn test_1() {
        let (before, after) = ("kitten", "sitting");
        assert_string_distance_parts(before, after, (3, 7));
        assert_eq!(
            edit_operations(before, after),
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
            edit_operations(before, after),
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

    #[test]
    fn test_3() {
        let (before, after) = ("aaabbb", "bbbbbb");
        // assert_string_distance_parts(before, after, (6, 6));
        assert_eq!(
            edit_operations(before, after),
            vec![Deletion, Deletion, Deletion, NoOp, NoOp, NoOp, Insertion, Insertion, Insertion,]
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
            x.grapheme_indices(true).collect::<Vec<(usize, &str)>>(),
            y.grapheme_indices(true).collect::<Vec<(usize, &str)>>(),
        );
        Alignment::new(x, y).distance_parts()
    }

    fn string_levenshtein_distance(x: &str, y: &str) -> usize {
        let (x, y) = (
            x.grapheme_indices(true).collect::<Vec<(usize, &str)>>(),
            y.grapheme_indices(true).collect::<Vec<(usize, &str)>>(),
        );
        Alignment::new(x, y).levenshtein_distance()
    }

    fn edit_operations<'a>(x: &'a str, y: &'a str) -> Vec<EditOperation> {
        let (x, y) = (
            x.grapheme_indices(true).collect::<Vec<(usize, &str)>>(),
            y.grapheme_indices(true).collect::<Vec<(usize, &str)>>(),
        );
        Alignment::new(x, y)
            .edit_operations(NoOp, Substitution, Deletion, Insertion, true)
            .iter()
            .map(|(op, _)| *op)
            .collect()
    }
}
