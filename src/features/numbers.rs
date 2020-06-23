#[cfg(test)]
pub mod tests {
    use console::strip_ansi_codes;

    use crate::tests::integration_test_utils::integration_test_utils::{make_config, run_delta};

    #[test]
    fn test_two_minus_lines() {
        let config = make_config(&[
            "--number",
            "--number-left-format",
            "%lm⋮",
            "--number-right-format",
            "%lp│",
        ]);
        let output = run_delta(TWO_MINUS_LINES_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), " 1  ⋮    │a = 1");
        assert_eq!(lines.next().unwrap(), " 2  ⋮    │b = 2");
    }

    #[test]
    fn test_two_plus_lines() {
        let config = make_config(&[
            "--number",
            "--number-left-format",
            "%lm⋮",
            "--number-right-format",
            "%lp│",
        ]);
        let output = run_delta(TWO_PLUS_LINES_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), "    ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), "    ⋮ 2  │b = 2");
    }

    #[test]
    fn test_one_minus_one_plus_line() {
        let config = make_config(&[
            "--number",
            "--number-left-format",
            "%lm⋮",
            "--number-right-format",
            "%lp│",
        ]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), " 1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "    ⋮ 2  │bb = 2");
    }

    #[test]
    fn test_repeated_placeholder() {
        let config = make_config(&[
            "--number",
            "--number-left-format",
            "%lm %lm⋮",
            "--number-right-format",
            "%lp│",
        ]);
        let output = run_delta(ONE_MINUS_ONE_PLUS_LINE_DIFF, &config);
        println!("{}", output);
        let output = strip_ansi_codes(&output);
        let mut lines = output.lines().skip(4);
        assert_eq!(lines.next().unwrap(), " 1   1  ⋮ 1  │a = 1");
        assert_eq!(lines.next().unwrap(), " 2   2  ⋮    │b = 2");
        assert_eq!(lines.next().unwrap(), "        ⋮ 2  │bb = 2");
    }

    const TWO_MINUS_LINES_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..e69de29 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +0,0 @@
-a = 1
-b = 2
";

    const TWO_PLUS_LINES_DIFF: &str = "\
diff --git c/a.py i/a.py
new file mode 100644
index 0000000..223ca50
--- /dev/null
+++ i/a.py
@@ -0,0 +1,2 @@
+a = 1
+b = 2
";

    const ONE_MINUS_ONE_PLUS_LINE_DIFF: &str = "\
diff --git i/a.py w/a.py
index 223ca50..367a6f6 100644
--- i/a.py
+++ w/a.py
@@ -1,2 +1,2 @@
 a = 1
-b = 2
+bb = 2
";
}
