/// Return the set of all interleavings of two sequences.
pub fn interleavings<'a, T>(s1: &'a [T], s2: &'a [T]) -> Vec<Vec<&'a T>> {
    let mut _interleavings = Vec::with_capacity(number_of_interleavings(s1, s2));
    if s1.len() > 0 || s2.len() > 0 {
        if s1.len() == 0 {
            _interleavings.push(s2.iter().collect());
        } else if s2.len() == 0 {
            _interleavings.push(s1.iter().collect());
        } else {
            extend_interleavings(s1, s2, &mut _interleavings);
            extend_interleavings(s2, s1, &mut _interleavings);
        }
    }
    _interleavings
}

fn extend_interleavings<'a, T>(s1: &'a [T], s2: &'a [T], _interleavings: &mut Vec<Vec<&'a T>>) {
    let (first, rest) = s1.split_first().unwrap();
    for child_interleaving in interleavings(rest, s2) {
        let mut interleaving = Vec::new();
        interleaving.push(first);
        interleaving.extend(child_interleaving);
        _interleavings.push(interleaving);
    }
}

/// The number of possible interleavings of two sequences.
// Let s1.len() == n1 and s2.len() == n2. Then, since the subsequences
// retain their order, the number of possible interleavings is equal
// to the number of ways of choosing n1 slots from (n1 + n2) slots.
fn number_of_interleavings<T>(s1: &[T], s2: &[T]) -> usize {
    number_of_subsets(s1.len() + s2.len(), s1.len())
}

/// The number of subsets of size k that can be formed from a set of size n, i.e. n choose k.
fn number_of_subsets(n: usize, k: usize) -> usize {
    number_of_tuples(n, k) / number_of_tuples(k, k)
}

/// The number of k-tuples that can be formed from a set of size n, i.e. the falling factorial.
/// prod_{i=0}^{k-1} (n - i)
fn number_of_tuples(n: usize, k: usize) -> usize {
    (0..k).map(|i| n - i).product()
}

mod tests {
    use super::*;

    #[test]
    fn test_number_of_tuples() {
        assert_eq!(number_of_tuples(5, 3), 60);
    }

    #[test]
    fn test_number_of_subsets() {
        assert_eq!(number_of_subsets(5, 3), 10);
    }

    #[test]
    fn test_number_of_interleavings() {
        let s1 = vec![1, 2, 3, 4, 5, 6];
        let s2 = vec![7, 8, 9, 10, 11];
        assert_eq!(number_of_interleavings(&s1, &s2), number_of_subsets(11, 6));
        assert_eq!(interleavings(&s1, &s2).len(), number_of_subsets(11, 6));
    }

    #[test]
    fn test_interleavings_0() {
        assert_eq!(
            interleavings(&Vec::<u8>::new(), &Vec::<u8>::new()),
            Vec::<Vec::<&u8>>::new()
        );
    }

    #[test]
    fn test_interleavings_1() {
        assert_eq!(interleavings(&vec![1], &Vec::<u8>::new()), vec![vec![&1]]);
    }

    #[test]
    fn test_interleavings_2() {
        assert_eq!(
            interleavings(&vec![1], &vec![2, 3]),
            vec![vec![&1, &2, &3], vec![&2, &3, &1], vec![&2, &1, &3]]
        );
    }

}
