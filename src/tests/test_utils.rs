#![cfg(test)]
/// Return true iff `s` contains exactly one occurrence of substring `t`.
pub fn contains_once(s: &str, t: &str) -> bool {
    match (s.find(t), s.rfind(t)) {
        (Some(i), Some(j)) => i == j,
        _ => false,
    }
}

#[allow(dead_code)]
pub fn print_with_line_numbers(s: &str) {
    for (i, t) in s.lines().enumerate() {
        println!("{:>2}│ {}", i + 1, t);
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_utils::*;

    #[test]
    fn test_contains_once_1() {
        assert!(contains_once("", ""));
    }

    #[test]
    fn test_contains_once_2() {
        assert!(contains_once("a", "a"));
    }

    #[test]
    fn test_contains_once_3() {
        assert!(!contains_once("", "a"));
    }

    #[test]
    fn test_contains_once_4() {
        assert!(!contains_once("a", "b"));
    }

    #[test]
    fn test_contains_once_5() {
        assert!(!contains_once("a a", "a"));
    }

    #[test]
    fn test_contains_once_6() {
        assert!(contains_once("a b", "b"));
    }
}
