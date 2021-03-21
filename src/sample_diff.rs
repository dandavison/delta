pub const DIFF: &[u8; 8715] = b"\
commit dc267979a46caee4d79ed2e3d17af9bd513c4e39
Author: Dan Davison <dandavison7@gmail.com>
Date:   Fri Jan 8 10:33:55 2021 -0500

    Prevent tests setting env vars from affecting other tests

diff --git a/src/git_config/git_config.rs b/src/git_config/git_config.rs
index 9620960..d95a6b1 100644
--- a/src/git_config/git_config.rs
+++ b/src/git_config/git_config.rs
@@ -42,10 +42,14 @@ impl GitConfig {
     }

     #[cfg(test)]
-    pub fn from_path(path: &Path) -> Self {
+    pub fn from_path(path: &Path, honor_env_var: bool) -> Self {
         Self {
             config: git2::Config::open(path).unwrap(),
-            config_from_env_var: parse_config_from_env_var(),
+            config_from_env_var: if honor_env_var {
+                parse_config_from_env_var()
+            } else {
+                HashMap::new()
+            },
             repo: None,
             enabled: true,
         }
diff --git a/src/options/get.rs b/src/options/get.rs
index f19c329..a36987c 100644
--- a/src/options/get.rs
+++ b/src/options/get.rs
@@ -117,8 +117,13 @@ pub mod tests {

     use crate::tests::integration_test_utils::integration_test_utils;

+    // TODO: the followig tests are collapsed into one since they all set the same env var and thus
+    // could affect each other if allowed to run concurrently.
+
     #[test]
-    fn test_simple_string_env_var_overrides_git_config() {
+    fn test_env_var_overrides_git_config() {
+        // ----------------------------------------------------------------------------------------
+        // simple string
         let git_config_contents = b\"
 [delta]
     plus-style = blue
@@ -133,7 +138,7 @@ pub mod tests {
         assert_eq!(opt.plus_style, \"blue\");

         env::set_var(\"GIT_CONFIG_PARAMETERS\", \"'delta.plus-style=green'\");
-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
@@ -141,10 +146,9 @@ pub mod tests {
         assert_eq!(opt.plus_style, \"green\");

         remove_file(git_config_path).unwrap();
-    }

-    #[test]
-    fn test_complex_string_env_var_overrides_git_config() {
+        // ----------------------------------------------------------------------------------------
+        // complex string
         let git_config_contents = br##\"
 [delta]
     minus-style = red bold ul \"#ffeeee\"
@@ -162,7 +166,7 @@ pub mod tests {
             \"GIT_CONFIG_PARAMETERS\",
             r##\"'delta.minus-style=magenta italic ol \"#aabbcc\"'\"##,
         );
-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
@@ -170,10 +174,9 @@ pub mod tests {
         assert_eq!(opt.minus_style, r##\"magenta italic ol \"#aabbcc\"\"##,);

         remove_file(git_config_path).unwrap();
-    }

-    #[test]
-    fn test_option_string_env_var_overrides_git_config() {
+        // ----------------------------------------------------------------------------------------
+        // option string
         let git_config_contents = b\"
 [delta]
     plus-style = blue
@@ -188,7 +191,7 @@ pub mod tests {
         assert_eq!(opt.plus_style, \"blue\");

         env::set_var(\"GIT_CONFIG_PARAMETERS\", \"'delta.plus-style=green'\");
-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
@@ -196,10 +199,9 @@ pub mod tests {
         assert_eq!(opt.plus_style, \"green\");

         remove_file(git_config_path).unwrap();
-    }

-    #[test]
-    fn test_bool_env_var_overrides_git_config() {
+        // ----------------------------------------------------------------------------------------
+        // bool
         let git_config_contents = b\"
 [delta]
     side-by-side = true
@@ -214,7 +216,7 @@ pub mod tests {
         assert_eq!(opt.side_by_side, true);

         env::set_var(\"GIT_CONFIG_PARAMETERS\", \"'delta.side-by-side=false'\");
-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
@@ -222,10 +224,9 @@ pub mod tests {
         assert_eq!(opt.side_by_side, false);

         remove_file(git_config_path).unwrap();
-    }

-    #[test]
-    fn test_int_env_var_overrides_git_config() {
+        // ----------------------------------------------------------------------------------------
+        // int
         let git_config_contents = b\"
 [delta]
     max-line-length = 1
@@ -240,7 +241,7 @@ pub mod tests {
         assert_eq!(opt.max_line_length, 1);

         env::set_var(\"GIT_CONFIG_PARAMETERS\", \"'delta.max-line-length=2'\");
-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
@@ -248,17 +249,16 @@ pub mod tests {
         assert_eq!(opt.max_line_length, 2);

         remove_file(git_config_path).unwrap();
-    }

-    #[test]
-    fn test_float_env_var_overrides_git_config() {
+        // ----------------------------------------------------------------------------------------
+        // float
         let git_config_contents = b\"
 [delta]
     max-line-distance = 0.6
     \";
         let git_config_path = \"delta__test_float_env_var_overrides_git_config.gitconfig\";

-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
@@ -266,7 +266,7 @@ pub mod tests {
         assert_eq!(opt.max_line_distance, 0.6);

         env::set_var(\"GIT_CONFIG_PARAMETERS\", \"'delta.max-line-distance=0.7'\");
-        let opt = integration_test_utils::make_options_from_args_and_git_config(
+        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
             &[],
             Some(git_config_contents),
             Some(git_config_path),
diff --git a/src/tests/integration_test_utils.rs b/src/tests/integration_test_utils.rs
index 37ae057..8eb3674 100644
--- a/src/tests/integration_test_utils.rs
+++ b/src/tests/integration_test_utils.rs
@@ -17,12 +17,29 @@ pub mod integration_test_utils {
         args: &[&str],
         git_config_contents: Option<&[u8]>,
         git_config_path: Option<&str>,
+    ) -> cli::Opt {
+        _make_options_from_args_and_git_config(args, git_config_contents, git_config_path, false)
+    }
+
+    pub fn make_options_from_args_and_git_config_honoring_env_var(
+        args: &[&str],
+        git_config_contents: Option<&[u8]>,
+        git_config_path: Option<&str>,
+    ) -> cli::Opt {
+        _make_options_from_args_and_git_config(args, git_config_contents, git_config_path, true)
+    }
+
+    fn _make_options_from_args_and_git_config(
+        args: &[&str],
+        git_config_contents: Option<&[u8]>,
+        git_config_path: Option<&str>,
+        honor_env_var: bool,
     ) -> cli::Opt {
         let mut args: Vec<&str> = itertools::chain(&[\"/dev/null\", \"/dev/null\"], args)
             .map(|s| *s)
             .collect();
         let mut git_config = match (git_config_contents, git_config_path) {
-            (Some(contents), Some(path)) => Some(make_git_config(contents, path)),
+            (Some(contents), Some(path)) => Some(make_git_config(contents, path, honor_env_var)),
             _ => {
                 args.push(\"--no-gitconfig\");
                 None
@@ -52,11 +69,11 @@ pub mod integration_test_utils {
         config::Config::from(make_options_from_args(args))
     }

-    fn make_git_config(contents: &[u8], path: &str) -> GitConfig {
+    fn make_git_config(contents: &[u8], path: &str, honor_env_var: bool) -> GitConfig {
         let path = Path::new(path);
         let mut file = File::create(path).unwrap();
         file.write_all(contents).unwrap();
-        GitConfig::from_path(&path)
+        GitConfig::from_path(&path, honor_env_var)
     }

     pub fn get_line_of_code_from_delta(
";
