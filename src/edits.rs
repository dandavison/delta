use unicode_segmentation::UnicodeSegmentation;

use crate::align;

/// Infer the edit operations responsible for the differences between a collection of old and new
/// lines. A "line" is a string. An annotated line is a Vec of (op, &str) pairs, where the &str
/// slices are slices of the line, and their concatenation equals the line. Return the input minus
/// and plus lines, in annotated form.
pub fn infer_edits<'a, EditOperation>(
    minus_lines: &'a Vec<String>,
    plus_lines: &'a Vec<String>,
    non_deletion: EditOperation,
    deletion: EditOperation,
    non_insertion: EditOperation,
    insertion: EditOperation,
    distance_threshold: f64,
) -> (
    Vec<Vec<(EditOperation, &'a str)>>, // annotated minus lines
    Vec<Vec<(EditOperation, &'a str)>>, // annotated plus lines
)
where
    EditOperation: Copy,
    EditOperation: PartialEq,
{
    let mut annotated_minus_lines = Vec::<Vec<(EditOperation, &str)>>::new();
    let mut annotated_plus_lines = Vec::<Vec<(EditOperation, &str)>>::new();

    let mut emitted = 0; // plus lines emitted so far

    'minus_lines_loop: for minus_line in minus_lines {
        let mut considered = 0; // plus lines considered so far as match for minus_line
        let minus_line = minus_line.trim_end();
        for plus_line in &plus_lines[emitted..] {
            let plus_line = plus_line.trim_end();
            let alignment = align::Alignment::new(
                minus_line
                    .grapheme_indices(true)
                    .collect::<Vec<(usize, &str)>>(),
                plus_line
                    .grapheme_indices(true)
                    .collect::<Vec<(usize, &str)>>(),
            );

            if alignment.normalized_edit_distance() < distance_threshold {
                // minus_line and plus_line are inferred to be a homologous pair.

                // Emit as unpaired the plus lines already considered and rejected
                for plus_line in &plus_lines[emitted..(emitted + considered)] {
                    annotated_plus_lines.push(vec![(non_insertion, plus_line.trim_end())]);
                }
                emitted += considered;

                // Emit the homologous pair.
                annotated_minus_lines.push(coalesce_minus_edits(
                    &alignment,
                    minus_line,
                    non_deletion,
                    deletion,
                    non_insertion,
                    insertion,
                ));
                annotated_plus_lines.push(coalesce_plus_edits(
                    &alignment,
                    plus_line,
                    non_deletion,
                    deletion,
                    non_insertion,
                    insertion,
                ));
                emitted += 1;

                // Move on to the next minus line.
                continue 'minus_lines_loop;
            } else {
                considered += 1;
            }
        }
        // No homolog was found for minus i; emit as unpaired.
        annotated_minus_lines.push(vec![(non_deletion, minus_line.trim_end())]);
    }
    // Emit any remaining plus lines
    for plus_line in &plus_lines[emitted..] {
        annotated_plus_lines.push(vec![(non_insertion, plus_line.trim_end())]);
    }

    (annotated_minus_lines, annotated_plus_lines)
}

pub fn coalesce_minus_edits<'a, EditOperation>(
    alignment: &align::Alignment<'a>,
    line: &'a str,
    non_deletion: EditOperation,
    deletion: EditOperation,
    _non_insertion: EditOperation,
    insertion: EditOperation,
) -> Vec<(EditOperation, &'a str)>
where
    EditOperation: Copy,
    EditOperation: PartialEq,
{
    coalesce_edits(
        alignment.edit_operations(non_deletion, deletion, deletion, insertion, true),
        line,
        insertion,
    )
}

pub fn coalesce_plus_edits<'a, EditOperation>(
    alignment: &align::Alignment<'a>,
    line: &'a str,
    _non_deletion: EditOperation,
    deletion: EditOperation,
    non_insertion: EditOperation,
    insertion: EditOperation,
) -> Vec<(EditOperation, &'a str)>
where
    EditOperation: Copy,
    EditOperation: PartialEq,
{
    coalesce_edits(
        alignment.edit_operations(non_insertion, insertion, deletion, insertion, false),
        line,
        deletion,
    )
}

fn coalesce_edits<'a, 'b, EditOperation>(
    operations: Vec<(EditOperation, (usize, &'b str))>,
    line: &'a str,
    irrelevant: EditOperation,
) -> Vec<(EditOperation, &'a str)>
where
    EditOperation: Copy,
    EditOperation: PartialEq,
{
    let mut edits = Vec::new(); // TODO capacity
    let mut operations = operations.iter().filter(|(op, _)| *op != irrelevant);
    let next = operations.next();
    if next.is_none() {
        return edits;
    }
    let (mut last_op, (mut last_offset, _)) = next.unwrap();
    let mut curr_op = last_op;
    let mut curr_offset;
    for (op, (offset, _)) in operations {
        curr_op = *op;
        curr_offset = *offset;
        if curr_op != last_op {
            edits.push((last_op, &line[last_offset..*offset]));
            last_offset = curr_offset;
            last_op = curr_op;
        }
    }
    if curr_op == last_op {
        edits.push((last_op, &line[last_offset..]));
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_segmentation::UnicodeSegmentation;

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

    const DISTANCE_MAX: f64 = 2.0;

    #[test]
    fn test_coalesce_edits_1() {
        assert_eq!(
            coalesce_edits(
                vec![(MinusNoop, (0, "a")), (MinusNoop, (1, "b"))],
                "ab",
                Insertion
            ),
            vec![(MinusNoop, "ab")]
        )
    }

    #[test]
    fn test_infer_edits_1() {
        assert_paired_edits(
            vec!["aaa\n"],
            vec!["aba\n"],
            (
                vec![vec![(MinusNoop, "a"), (Deletion, "a"), (MinusNoop, "a")]],
                vec![vec![(PlusNoop, "a"), (Insertion, "b"), (PlusNoop, "a")]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_2() {
        assert_paired_edits(
            vec!["áaa\n"],
            vec!["ááb\n"],
            (
                vec![vec![(MinusNoop, "á"), (Deletion, "aa")]],
                vec![vec![(PlusNoop, "á"), (Insertion, "áb")]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_3() {
        assert_paired_edits(
            vec!["d.iteritems()\n"],
            vec!["d.items()\n"],
            (
                vec![vec![
                    (MinusNoop, "d."),
                    (Deletion, "iter"),
                    (MinusNoop, "items()"),
                ]],
                vec![vec![(PlusNoop, "d.items()")]],
            ),
        )
    }

    #[test]
    fn test_infer_edits_4() {
        assert_edits(
            vec!["áaaáaaáaa\n", "áábáábááb\n"],
            vec!["áábáácááb\n"],
            (
                vec![
                    vec![(MinusNoop, "áaaáaaáaa")],
                    vec![
                        (MinusNoop, "áábáá"),
                        (Deletion, "b"),
                        (MinusNoop, "ááb"),
                    ],
                ],
                vec![vec![
                    (PlusNoop, "áábáá"),
                    (Insertion, "c"),
                    (PlusNoop, "ááb"),
                ]],
            ),
            0.66,
        )
    }

    #[test]
    fn test_infer_edits_5() {
        assert_edits(
            vec!["aaaaaaaa\n", "bbbbbbbb\n", "cccccccc\n"],
            vec!["bbbb!bbb\n", "dddddddd\n", "cccc!ccc\n"],
            (
                vec![
                    vec![(MinusNoop, "aaaaaaaa")],
                    vec![(MinusNoop, "bbbb"), (Deletion, "b"), (MinusNoop, "bbb")],
                    vec![(MinusNoop, "cccc"), (Deletion, "c"), (MinusNoop, "ccc")],
                ],
                vec![
                    vec![(PlusNoop, "bbbb"), (Insertion, "!"), (PlusNoop, "bbb")],
                    vec![(PlusNoop, "dddddddd")],
                    vec![(PlusNoop, "cccc"), (Insertion, "!"), (PlusNoop, "ccc")],
                ],
            ),
            0.66,
        )
    }

    #[test]
    fn test_infer_edits_6() {
        assert_no_edits(
            vec![
                "             let mut i = 0;\n",
                "             for ((_, c0), (_, c1)) in s0.zip(s1) {\n",
                "                 if c0 != c1 {\n",
                "                     break;\n",
                "                 } else {\n",
                "                     i += c0.len();\n",
                "                 }\n",
                "             }\n",
                "             i\n",
            ],
            vec![
                "             s0.zip(s1)\n",
                "                 .take_while(|((_, c0), (_, c1))| c0 == c1) // TODO: Don't consume one-past-the-end!\n",
                "                 .fold(0, |offset, ((_, c0), (_, _))| offset + c0.len())\n"
            ], 0.66)
    }

    fn assert_edits(
        minus_lines: Vec<&str>,
        plus_lines: Vec<&str>,
        expected_edits: Edits,
        distance_threshold: f64,
    ) {
        let minus_lines = minus_lines
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let plus_lines = plus_lines
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let actual_edits = infer_edits(
            &minus_lines,
            &plus_lines,
            MinusNoop,
            Deletion,
            PlusNoop,
            Insertion,
            distance_threshold,
        );
        assert_eq!(actual_edits, expected_edits);
    }

    // Assert that no edits are inferred for the supplied minus and plus lines.
    fn assert_no_edits(minus_lines: Vec<&str>, plus_lines: Vec<&str>, distance_threshold: f64) {
        let expected_edits = (
            minus_lines.iter().map(|s| vec![(MinusNoop, *s)]).collect(),
            plus_lines.iter().map(|s| vec![(PlusNoop, *s)]).collect(),
        );
        assert_edits(minus_lines, plus_lines, expected_edits, distance_threshold)
    }

    // Assertions for a single pair of lines, considered as a homologous pair. We set
    // distance_threshold = DISTANCE_MAX in order that the pair will be inferred to be homologous.
    fn assert_paired_edits(minus_lines: Vec<&str>, plus_lines: Vec<&str>, expected_edits: Edits) {
        assert_consistent_pairs(&expected_edits);
        assert_edits(minus_lines, plus_lines, expected_edits, DISTANCE_MAX);
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
        print!("\n");
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
