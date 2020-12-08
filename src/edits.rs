use regex::Regex;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::align;

/// Infer the edit operations responsible for the differences between a collection of old and new
/// lines. A "line" is a string. An annotated line is a Vec of (op, &str) pairs, where the &str
/// slices are slices of the line, and their concatenation equals the line. Return the input minus
/// and plus lines, in annotated form. Also return a specification of the inferred alignment of
/// minus and plus lines. `noop_deletions[i]` is the appropriate deletion operation tag to be used
/// for `minus_lines[i]`; `noop_deletions` is guaranteed to be the same length as `minus_lines`.
/// The equivalent statements hold for `plus_insertions` and `plus_lines`.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn infer_edits<'a, EditOperation>(
    minus_lines: Vec<&'a str>,
    plus_lines: Vec<&'a str>,
    noop_deletions: Vec<EditOperation>,
    deletion: EditOperation,
    noop_insertions: Vec<EditOperation>,
    insertion: EditOperation,
    tokenization_regex: &Regex,
    max_line_distance: f64,
    max_line_distance_for_naively_paired_lines: f64,
) -> (
    Vec<Vec<(EditOperation, &'a str)>>,  // annotated minus lines
    Vec<Vec<(EditOperation, &'a str)>>,  // annotated plus lines
    Vec<(Option<usize>, Option<usize>)>, // line alignment
)
where
    EditOperation: Copy,
    EditOperation: PartialEq,
{
    let mut annotated_minus_lines = Vec::<Vec<(EditOperation, &str)>>::new();
    let mut annotated_plus_lines = Vec::<Vec<(EditOperation, &str)>>::new();
    let mut line_alignment = Vec::<(Option<usize>, Option<usize>)>::new();

    let mut plus_index = 0; // plus lines emitted so far

    'minus_lines_loop: for (minus_index, minus_line) in minus_lines.iter().enumerate() {
        let mut considered = 0; // plus lines considered so far as match for minus_line
        for plus_line in &plus_lines[plus_index..] {
            let alignment = align::Alignment::new(
                tokenize(minus_line, tokenization_regex),
                tokenize(plus_line, tokenization_regex),
            );
            let (annotated_minus_line, annotated_plus_line, distance) = annotate(
                alignment,
                noop_deletions[minus_index],
                deletion,
                noop_insertions[plus_index],
                insertion,
                minus_line,
                plus_line,
            );
            if minus_lines.len() == plus_lines.len()
                && distance <= max_line_distance_for_naively_paired_lines
                || distance <= max_line_distance
            {
                // minus_line and plus_line are inferred to be a homologous pair.

                // Emit as unpaired the plus lines already considered and rejected
                for plus_line in &plus_lines[plus_index..(plus_index + considered)] {
                    annotated_plus_lines.push(vec![(noop_insertions[plus_index], plus_line)]);
                    line_alignment.push((None, Some(plus_index)));
                    plus_index += 1;
                }
                annotated_minus_lines.push(annotated_minus_line);
                annotated_plus_lines.push(annotated_plus_line);
                line_alignment.push((Some(minus_index), Some(plus_index)));
                plus_index += 1;

                // Greedy: move on to the next minus line.
                continue 'minus_lines_loop;
            } else {
                considered += 1;
            }
        }
        // No homolog was found for minus i; emit as unpaired.
        annotated_minus_lines.push(vec![(noop_deletions[minus_index], minus_line)]);
        line_alignment.push((Some(minus_index), None));
    }
    // Emit any remaining plus lines
    for plus_line in &plus_lines[plus_index..] {
        annotated_plus_lines.push(vec![(noop_insertions[plus_index], plus_line)]);
        line_alignment.push((None, Some(plus_index)));
        plus_index += 1;
    }

    (annotated_minus_lines, annotated_plus_lines, line_alignment)
}

/// Split line into tokens for alignment. The alignment algorithm aligns sequences of substrings;
/// not individual characters.
fn tokenize<'a>(line: &'a str, regex: &Regex) -> Vec<&'a str> {
    let mut tokens = Vec::new();
    let mut offset = 0;
    for m in regex.find_iter(line) {
        if offset == 0 && m.start() > 0 {
            tokens.push("");
        }
        // Align separating text as multiple single-character tokens.
        for t in line[offset..m.start()].graphemes(true) {
            tokens.push(t);
        }
        tokens.push(&line[m.start()..m.end()]);
        offset = m.end();
    }
    if offset < line.len() {
        if offset == 0 {
            tokens.push("");
        }
        for t in line[offset..line.len()].graphemes(true) {
            tokens.push(t);
        }
    }
    tokens
}

/// Use alignment to "annotate" minus and plus lines. An "annotated" line is a sequence of
/// (a: Annotation, s: &str) pairs, where the &strs reference the memory
/// of the original line and their concatenation equals the line.
// This function doesn't return "coalesced" annotations: i.e. they're often are runs of consecutive
// occurrences of the same operation. Since it is returning &strs pointing into the memory of the
// original line, it's not possible to coalesce them in this function.
#[allow(clippy::type_complexity)]
fn annotate<'a, Annotation>(
    alignment: align::Alignment<'a>,
    noop_deletion: Annotation,
    deletion: Annotation,
    noop_insertion: Annotation,
    insertion: Annotation,
    minus_line: &'a str,
    plus_line: &'a str,
) -> (Vec<(Annotation, &'a str)>, Vec<(Annotation, &'a str)>, f64)
where
    Annotation: Copy,
    Annotation: PartialEq,
{
    let mut annotated_minus_line = Vec::new();
    let mut annotated_plus_line = Vec::new();

    let (mut x_offset, mut y_offset) = (0, 0);
    let (mut minus_line_offset, mut plus_line_offset) = (0, 0);
    let (mut d_numer, mut d_denom) = (0, 0);

    // Note that the inputs to align::Alignment are not the original strings themselves, but
    // sequences of substrings derived from the tokenization process. We have just applied
    // run_length_encoding to "coalesce" runs of the same edit operation into a single
    // operation. We now need to form a &str, pointing into the memory of the original line,
    // identifying a "section" which is the concatenation of the substrings involved in this
    // coalesced operation. That's what the following closures do. Note that they must be called
    // once only since they advance offset pointers.
    let get_section = |n: usize,
                       line_offset: &mut usize,
                       substrings_offset: &mut usize,
                       substrings: &[&str],
                       line: &'a str| {
        let section_length = substrings[*substrings_offset..*substrings_offset + n]
            .iter()
            .fold(0, |n, s| n + s.len());
        let old_offset = *line_offset;
        *line_offset += section_length;
        *substrings_offset += n;
        &line[old_offset..*line_offset]
    };
    let mut minus_section = |n| {
        get_section(
            n,
            &mut minus_line_offset,
            &mut x_offset,
            &alignment.x,
            minus_line,
        )
    };
    let mut plus_section = |n| {
        get_section(
            n,
            &mut plus_line_offset,
            &mut y_offset,
            &alignment.y,
            plus_line,
        )
    };
    let distance_contribution = |section: &str| UnicodeWidthStr::width(section.trim());

    let (mut minus_op_prev, mut plus_op_prev) = (noop_deletion, noop_insertion);
    for (op, n) in alignment.coalesced_operations() {
        match op {
            align::Operation::Deletion => {
                let minus_section = minus_section(n);
                let n_d = distance_contribution(minus_section);
                d_denom += n_d;
                d_numer += n_d;
                annotated_minus_line.push((deletion, minus_section));
                minus_op_prev = deletion;
            }
            align::Operation::NoOp => {
                let minus_section = minus_section(n);
                let n_d = distance_contribution(minus_section);
                d_denom += n_d;
                let is_space = minus_section.trim().is_empty();
                let coalesce_space_with_previous = is_space
                    && ((minus_op_prev == deletion && plus_op_prev == insertion)
                        || (minus_op_prev == noop_deletion && plus_op_prev == noop_insertion));
                annotated_minus_line.push((
                    if coalesce_space_with_previous {
                        minus_op_prev
                    } else {
                        noop_deletion
                    },
                    minus_section,
                ));
                annotated_plus_line.push((
                    if coalesce_space_with_previous {
                        plus_op_prev
                    } else {
                        noop_insertion
                    },
                    plus_section(n),
                ));
                minus_op_prev = noop_deletion;
                plus_op_prev = noop_insertion;
            }
            align::Operation::Substitution => {
                let minus_section = minus_section(n);
                let n_d = distance_contribution(minus_section);
                d_denom += n_d;
                d_numer += n_d;
                annotated_minus_line.push((deletion, minus_section));
                annotated_plus_line.push((insertion, plus_section(n)));
                minus_op_prev = deletion;
                plus_op_prev = insertion;
            }
            align::Operation::Insertion => {
                let plus_section = plus_section(n);
                let n_d = distance_contribution(plus_section);
                d_denom += n_d;
                d_numer += n_d;
                annotated_plus_line.push((insertion, plus_section));
                plus_op_prev = insertion;
            }
        }
    }
    (
        annotated_minus_line,
        annotated_plus_line,
        compute_distance(d_numer as f64, d_denom as f64),
    )
}

fn compute_distance(d_numer: f64, d_denom: f64) -> f64 {
    if d_denom > 0.0 {
        d_numer / d_denom
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use lazy_static::lazy_static;
    use unicode_segmentation::UnicodeSegmentation;

    lazy_static! {
        static ref DEFAULT_TOKENIZATION_REGEXP: Regex = Regex::new(r#"\w+"#).unwrap();
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum EditOperation {
        MinusNoop,
        PlusNoop,
        Deletion,
        Insertion,
    }

    type Annotation<'a> = (EditOperation, &'a str);
    type AnnotatedLine<'a> = Vec<Annotation<'a>>;
    type AnnotatedLines<'a> = Vec<AnnotatedLine<'a>>;
    type Edits<'a> = (AnnotatedLines<'a>, AnnotatedLines<'a>);

    use EditOperation::*;

    #[test]
    fn test_tokenize_0() {
        assert_tokenize("", &[]);
        assert_tokenize(";", &["", ";"]);
        assert_tokenize(";;", &["", ";", ";"]);
        assert_tokenize(";;a", &["", ";", ";", "a"]);
        assert_tokenize(";;ab", &["", ";", ";", "ab"]);
        assert_tokenize(";;ab;", &["", ";", ";", "ab", ";"]);
        assert_tokenize(";;ab;;", &["", ";", ";", "ab", ";", ";"]);
    }

    #[test]
    fn test_tokenize_1() {
        assert_tokenize("aaa bbb", &["aaa", " ", "bbb"])
    }

    #[test]
    fn test_tokenize_2() {
        assert_tokenize(
            "fn coalesce_edits<'a, EditOperation>(",
            &[
                "fn",
                " ",
                "coalesce_edits",
                "<",
                "'",
                "a",
                ",",
                " ",
                "EditOperation",
                ">",
                "(",
            ],
        );
    }

    #[test]
    fn test_tokenize_3() {
        assert_tokenize(
            "fn coalesce_edits<'a, 'b, EditOperation>(",
            &[
                "fn",
                " ",
                "coalesce_edits",
                "<",
                "'",
                "a",
                ",",
                " ",
                "'",
                "b",
                ",",
                " ",
                "EditOperation",
                ">",
                "(",
            ],
        );
    }

    #[test]
    fn test_tokenize_4() {
        assert_tokenize(
            "annotated_plus_lines.push(vec![(noop_insertion, plus_line)]);",
            &[
                "annotated_plus_lines",
                ".",
                "push",
                "(",
                "vec",
                "!",
                "[",
                "(",
                "noop_insertion",
                ",",
                " ",
                "plus_line",
                ")",
                "]",
                ")",
                ";",
            ],
        );
    }

    #[test]
    fn test_tokenize_5() {
        assert_tokenize(
            "         let col = Color::from_str(s).unwrap_or_else(|_| die());",
            &[
                "",
                " ",
                " ",
                " ",
                " ",
                " ",
                " ",
                " ",
                " ",
                " ",
                "let",
                " ",
                "col",
                " ",
                "=",
                " ",
                "Color",
                ":",
                ":",
                "from_str",
                "(",
                "s",
                ")",
                ".",
                "unwrap_or_else",
                "(",
                "|",
                "_",
                "|",
                " ",
                "die",
                "(",
                ")",
                ")",
                ";",
            ],
        )
    }

    #[test]
    fn test_tokenize_6() {
        assert_tokenize(
            "         (minus_file, plus_file) => format!(\"renamed: {} ⟶  {}\", minus_file, plus_file),",
            &["",
              " ",
              " ",
              " ",
              " ",
              " ",
              " ",
              " ",
              " ",
              " ",
              "(",
              "minus_file",
              ",",
              " ",
              "plus_file",
              ")",
              " ",
              "=",
              ">",
              " ",
              "format",
              "!",
              "(",
              "\"",
              "renamed",
              ":",
              " ",
              "{",
              "}",
              " ",
              "⟶",
              " ",
              " ",
              "{",
              "}",
              "\"",
              ",",
              " ",
              "minus_file",
              ",",
              " ",
              "plus_file",
              ")",
              ","])
    }

    fn assert_tokenize(text: &str, expected_tokens: &[&str]) {
        let actual_tokens = tokenize(text, &*DEFAULT_TOKENIZATION_REGEXP);
        assert_eq!(text, expected_tokens.iter().join(""));
        assert_eq!(actual_tokens, expected_tokens);
    }

    #[test]
    fn test_infer_edits_1() {
        assert_paired_edits(
            vec!["aaa"],
            vec!["aba"],
            (
                vec![vec![(Deletion, "aaa")]],
                vec![vec![(Insertion, "aba")]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_1_2() {
        assert_paired_edits(
            vec!["aaa ccc"],
            vec!["aba ccc"],
            (
                vec![vec![(Deletion, "aaa"), (MinusNoop, " ccc")]],
                vec![vec![(Insertion, "aba"), (PlusNoop, " ccc")]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_2() {
        assert_paired_edits(
            vec!["áaa"],
            vec!["ááb"],
            (
                vec![vec![(Deletion, "áaa")]],
                vec![vec![(Insertion, "ááb")]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_3() {
        assert_paired_edits(
            vec!["d.iteritems()"],
            vec!["d.items()"],
            (
                vec![vec![
                    (MinusNoop, "d."),
                    (Deletion, "iteritems"),
                    (MinusNoop, "()"),
                ]],
                vec![vec![
                    (PlusNoop, "d."),
                    (Insertion, "items"),
                    (PlusNoop, "()"),
                ]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_4() {
        assert_edits(
            vec!["á a a á a a á a a", "á á b á á b á á b"],
            vec!["á á b á á c á á b"],
            (
                vec![
                    vec![(MinusNoop, "á a a á a a á a a")],
                    vec![
                        (MinusNoop, "á á b á á "),
                        (Deletion, "b"),
                        (MinusNoop, " á á b"),
                    ],
                ],
                vec![vec![
                    (PlusNoop, "á á b á á "),
                    (Insertion, "c"),
                    (PlusNoop, " á á b"),
                ]],
            ),
            0.66,
        )
    }

    #[test]
    fn test_infer_edits_5() {
        assert_edits(
            vec!["aaaa a aaa", "bbbb b bbb", "cccc c ccc"],
            vec!["bbbb ! bbb", "dddd d ddd", "cccc ! ccc"],
            (
                vec![
                    vec![(MinusNoop, "aaaa a aaa")],
                    vec![(MinusNoop, "bbbb "), (Deletion, "b"), (MinusNoop, " bbb")],
                    vec![(MinusNoop, "cccc "), (Deletion, "c"), (MinusNoop, " ccc")],
                ],
                vec![
                    vec![(PlusNoop, "bbbb "), (Insertion, "!"), (PlusNoop, " bbb")],
                    vec![(PlusNoop, "dddd d ddd")],
                    vec![(PlusNoop, "cccc "), (Insertion, "!"), (PlusNoop, " ccc")],
                ],
            ),
            0.66,
        )
    }

    #[test]
    fn test_infer_edits_6() {
        assert_no_edits(
            vec![
                "             let mut i = 0;",
                "             for ((_, c0), (_, c1)) in s0.zip(s1) {",
                "                 if c0 != c1 {",
                "                     break;",
                "                 } else {",
                "                     i += c0.len();",
                "                 }",
                "             }",
                "             i",
            ],
            vec![
                "             s0.zip(s1)",
                "                 .take_while(|((_, c0), (_, c1))| c0 == c1) // TODO: Don't consume one-past-the-end!",
                "                 .fold(0, |offset, ((_, c0), (_, _))| offset + c0.len())"
            ], 0.5)
    }

    #[test]
    fn test_infer_edits_7() {
        assert_edits(
            vec!["fn coalesce_edits<'a, EditOperation>("],
            vec!["fn coalesce_edits<'a, 'b, EditOperation>("],
            (
                vec![vec![
                    (MinusNoop, "fn coalesce_edits<'a"),
                    (MinusNoop, ", EditOperation>("),
                ]],
                vec![vec![
                    (PlusNoop, "fn coalesce_edits<'a"),
                    (Insertion, ", 'b"),
                    (PlusNoop, ", EditOperation>("),
                ]],
            ),
            0.66,
        )
    }

    #[test]
    fn test_infer_edits_8() {
        assert_edits(
            vec!["for _ in range(0, options[\"count\"]):"],
            vec!["for _ in range(0, int(options[\"count\"])):"],
            (
                vec![vec![
                    (MinusNoop, "for _ in range(0, "),
                    (MinusNoop, "options[\"count\"]"),
                    (MinusNoop, "):"),
                ]],
                vec![vec![
                    (PlusNoop, "for _ in range(0, "),
                    (Insertion, "int("),
                    (PlusNoop, "options[\"count\"]"),
                    (Insertion, ")"),
                    (PlusNoop, "):"),
                ]],
            ),
            0.3,
        )
    }

    #[test]
    fn test_infer_edits_9() {
        assert_edits(
            vec!["a a"],
            vec!["a b a"],
            (
                vec![vec![(MinusNoop, "a"), (MinusNoop, " a")]],
                vec![vec![(PlusNoop, "a"), (Insertion, " b"), (PlusNoop, " a")]],
            ),
            1.0,
        );
        assert_edits(
            vec!["a a"],
            vec!["a b b a"],
            (
                vec![vec![(MinusNoop, "a"), (MinusNoop, " a")]],
                vec![vec![(PlusNoop, "a"), (Insertion, " b b"), (PlusNoop, " a")]],
            ),
            1.0,
        );
    }

    #[test]
    fn test_infer_edits_10() {
        assert_edits(
            vec!["so it is safe to read the commit number from any one of them."],
            vec!["so it is safe to read build info from any one of them."],
            (
                // TODO: Coalesce runs of the same operation.
                vec![vec![
                    (MinusNoop, "so it is safe to read "),
                    (Deletion, "the"),
                    (Deletion, " "),
                    (Deletion, "commit"),
                    (Deletion, " "),
                    (Deletion, "number "),
                    (MinusNoop, "from any one of them."),
                ]],
                vec![vec![
                    (PlusNoop, "so it is safe to read "),
                    (Insertion, "build"),
                    (Insertion, " "),
                    (Insertion, "info"),
                    (Insertion, " "),
                    (PlusNoop, "from any one of them."),
                ]],
            ),
            1.0,
        );
    }

    #[test]
    fn test_infer_edits_11() {
        assert_edits(
            vec!["                 self.table[index] ="],
            vec!["                 self.table[index] = candidates"],
            (
                vec![vec![(MinusNoop, "                 self.table[index] =")]],
                vec![vec![
                    (PlusNoop, "                 self.table[index] ="),
                    (Insertion, " candidates"),
                ]],
            ),
            1.0,
        );
    }

    #[test]
    #[ignore]
    fn test_infer_edits_12() {
        assert_edits(
            vec!["                     (xxxxxxxxx, \"build info\"),"],
            vec!["                     (xxxxxxxxx, \"build\"),"],
            (
                vec![vec![
                    (MinusNoop, "                     (xxxxxxxxx, \"build"),
                    (Deletion, " info"),
                    (MinusNoop, "\"),"),
                ]],
                vec![vec![
                    (PlusNoop, "                     (xxxxxxxxx, \"build"),
                    (PlusNoop, "\"),"),
                ]],
            ),
            1.0,
        );
    }

    fn assert_edits(
        minus_lines: Vec<&str>,
        plus_lines: Vec<&str>,
        expected_edits: Edits,
        max_line_distance: f64,
    ) {
        let (minus_lines, noop_deletions): (Vec<&str>, Vec<EditOperation>) =
            minus_lines.into_iter().map(|s| (s, MinusNoop)).unzip();
        let (plus_lines, noop_insertions): (Vec<&str>, Vec<EditOperation>) =
            plus_lines.into_iter().map(|s| (s, PlusNoop)).unzip();
        let actual_edits = infer_edits(
            minus_lines,
            plus_lines,
            noop_deletions,
            Deletion,
            noop_insertions,
            Insertion,
            &*DEFAULT_TOKENIZATION_REGEXP,
            max_line_distance,
            0.0,
        );
        // compare_annotated_lines(actual_edits, expected_edits);
        // TODO: test line alignment
        assert_eq!((actual_edits.0, actual_edits.1), expected_edits);
    }

    // Assert that no edits are inferred for the supplied minus and plus lines.
    fn assert_no_edits(minus_lines: Vec<&str>, plus_lines: Vec<&str>, max_line_distance: f64) {
        let expected_edits = (
            minus_lines.iter().map(|s| vec![(MinusNoop, *s)]).collect(),
            plus_lines.iter().map(|s| vec![(PlusNoop, *s)]).collect(),
        );
        assert_edits(minus_lines, plus_lines, expected_edits, max_line_distance)
    }

    // Assertions for a single pair of lines, considered as a homologous pair. We set
    // max_line_distance = 1.0 in order that the pair will be inferred to be homologous.
    fn assert_paired_edits(minus_lines: Vec<&str>, plus_lines: Vec<&str>, expected_edits: Edits) {
        assert_consistent_pairs(&expected_edits);
        assert_edits(minus_lines, plus_lines, expected_edits, 1.0);
    }

    fn assert_consistent_pairs(edits: &Edits) {
        let (minus_annotated_lines, plus_annotated_lines) = edits;

        for (minus_annotated_line, plus_annotated_line) in
            minus_annotated_lines.iter().zip(plus_annotated_lines)
        {
            let (minus_total, minus_delta) = summarize_annotated_line(minus_annotated_line);
            let (plus_total, plus_delta) = summarize_annotated_line(plus_annotated_line);
            assert_eq!(
                minus_total - minus_delta,
                plus_total - plus_delta,
                "\nInconsistent edits:\n \
                 {:?}\n \
                 \tminus_total - minus_delta = {} - {} = {}\n \
                 {:?}\n \
                 \tplus_total  - plus_delta  = {} - {} = {}\n",
                minus_annotated_line,
                minus_total,
                minus_delta,
                minus_total - minus_delta,
                plus_annotated_line,
                plus_total,
                plus_delta,
                plus_total - plus_delta
            );
        }
    }

    fn summarize_annotated_line(sections: &AnnotatedLine) -> (usize, usize) {
        let mut total = 0;
        let mut delta = 0;
        for (edit, s) in sections {
            let length = s.graphemes(true).count();
            total += length;
            if is_edit(edit) {
                delta += length;
            }
        }
        (total, delta)
    }

    // For debugging test failures:

    #[allow(dead_code)]
    fn compare_annotated_lines(actual: Edits, expected: Edits) {
        let (minus, plus) = actual;
        println!("\n\nactual minus:");
        print_annotated_lines(minus);
        println!("\nactual plus:");
        print_annotated_lines(plus);

        let (minus, plus) = expected;
        println!("\n\nexpected minus:");
        print_annotated_lines(minus);
        println!("\nexpected plus:");
        print_annotated_lines(plus);
    }

    #[allow(dead_code)]
    fn print_annotated_lines(annotated_lines: AnnotatedLines) {
        for annotated_line in annotated_lines {
            print_annotated_line(annotated_line);
        }
    }

    #[allow(dead_code)]
    fn print_annotated_line(annotated_line: AnnotatedLine) {
        for (edit, s) in annotated_line {
            print!("({} {}), ", fmt_edit(edit), s.trim_end());
        }
        println!();
    }

    #[allow(dead_code)]
    fn fmt_edit(edit: EditOperation) -> &'static str {
        match edit {
            MinusNoop => "MinusNoop",
            Deletion => "Deletion",
            PlusNoop => "PlusNoop",
            Insertion => "Insertion",
        }
    }

    fn is_edit(edit: &EditOperation) -> bool {
        *edit == Deletion || *edit == Insertion
    }
}
