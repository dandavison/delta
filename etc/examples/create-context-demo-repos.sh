#!/usr/bin/env bash
#
# Creates three small git repos demonstrating the syntax-highlighting context
# bug. In each repo, `git diff HEAD~1` produces a hunk that starts inside a
# multiline construct (docstring / block comment / template literal), so
# syntect misparses the closing delimiter.
#
# Usage:
#   bash etc/examples/create-context-demo-repos.sh
#
# Then compare:
#   cd /tmp/context-demo-python
#   git diff HEAD~1 | delta                          # bug: return is string-colored
#   git diff -U9999 HEAD~1 | delta -U 3              # fix: return is keyword-colored

set -euo pipefail

DIR="${1:-/tmp}"

# --- Python triple-quoted string ---
REPO="$DIR/context-demo-python"
rm -rf "$REPO"
mkdir -p "$REPO" && cd "$REPO" && git init -q

cat > example.py << 'PYEOF'
def foo():
    """
    This is a docstring that spans
    multiple lines.
    It has lots of content.
    More content here.
    Even more explanation.
    Still going on.
    And yet more.
    Final docstring line.
    """
    x = 1
    return x
PYEOF
git add example.py && git commit -q -m "initial"

cat > example.py << 'PYEOF'
def foo():
    """
    This is a docstring that spans
    multiple lines.
    It has lots of content.
    More content here.
    Even more explanation.
    Still going on.
    And yet more.
    Final docstring line.
    """
    x = 2
    return x + 1
PYEOF
git add example.py && git commit -q -m "change"

# --- Rust block comment ---
REPO="$DIR/context-demo-rust"
rm -rf "$REPO"
mkdir -p "$REPO" && cd "$REPO" && git init -q

cat > lib.rs << 'RSEOF'
/// Entry point for the library.
pub fn process(input: &str) -> Result<String, Error> {
    /*
     * Multi-line comment explaining
     * the complex validation logic
     * that follows below.
     * It covers edge cases for:
     * - empty input
     * - unicode input
     * - oversized input
     * - malformed input
     */
    let validated = validate(input)?;
    let result = transform(validated);
    Ok(result)
}
RSEOF
git add lib.rs && git commit -q -m "initial"

cat > lib.rs << 'RSEOF'
/// Entry point for the library.
pub fn process(input: &str) -> Result<String, Error> {
    /*
     * Multi-line comment explaining
     * the complex validation logic
     * that follows below.
     * It covers edge cases for:
     * - empty input
     * - unicode input
     * - oversized input
     * - malformed input
     */
    let validated = validate(input)?;
    let trimmed = validated.trim();
    let result = transform(trimmed);
    Ok(result)
}
RSEOF
git add lib.rs && git commit -q -m "change"

# --- JavaScript template literal ---
REPO="$DIR/context-demo-js"
rm -rf "$REPO"
mkdir -p "$REPO" && cd "$REPO" && git init -q

cat > render.js << 'JSEOF'
function render(data) {
    const html = `
        <div class="container">
            <header>
                <h1>${data.title}</h1>
                <nav>${data.breadcrumbs}</nav>
            </header>
            <main>
                <p>${data.description}</p>
                <ul>${data.items}</ul>
            </main>
        </div>
    `;
    const element = document.createElement("div");
    element.innerHTML = html;
    return element;
}
JSEOF
git add render.js && git commit -q -m "initial"

cat > render.js << 'JSEOF'
function render(data) {
    const html = `
        <div class="container">
            <header>
                <h1>${data.title}</h1>
                <nav>${data.breadcrumbs}</nav>
            </header>
            <main>
                <p>${data.description}</p>
                <ul>${data.items}</ul>
            </main>
        </div>
    `;
    const wrapper = document.createElement("section");
    wrapper.innerHTML = html;
    wrapper.classList.add("rendered");
    return wrapper;
}
JSEOF
git add render.js && git commit -q -m "change"

echo "Created repos:"
echo "  $DIR/context-demo-python"
echo "  $DIR/context-demo-rust"
echo "  $DIR/context-demo-js"
echo
echo "Try:"
echo "  cd $DIR/context-demo-python"
echo "  git diff HEAD~1 | delta                    # bug"
echo "  git diff -U9999 HEAD~1 | delta -U 3        # fix"
