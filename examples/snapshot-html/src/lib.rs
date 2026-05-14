//! Snapshot testing rendered HTML with `test-better`.
//!
//! A renderer that produces a chunk of text is a natural fit for snapshot
//! testing: instead of hand-asserting each tag, the test pins the *whole
//! output* against a known-good copy. `expect!` supports two forms:
//!
//! - `to_match_inline_snapshot(r#"..."#)` keeps the expected value in the test
//!   source, which is what this example uses (it stays self-contained, with no
//!   committed `.snap` files);
//! - `to_match_snapshot("name")` keeps it in a file under `tests/snapshots/`,
//!   the better choice for larger outputs.
//!
//! Either way, `UPDATE_SNAPSHOTS=1 cargo test` regenerates the expected value
//! after a deliberate change, so review sees the diff.
//!
//! Run the suite with `cargo test -p snapshot-html-example`.

/// Escapes the three characters that must not appear literally in HTML text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Renders a minimal HTML page: a title, a heading, and a list of items.
///
/// An empty `items` slice renders a placeholder paragraph instead of an empty
/// list. All text is HTML-escaped.
#[must_use]
pub fn render_page(title: &str, items: &[&str]) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html>\n");
    html.push_str(&format!("<title>{}</title>\n", escape(title)));
    html.push_str(&format!("<h1>{}</h1>\n", escape(title)));
    if items.is_empty() {
        html.push_str("<p>nothing here yet</p>");
    } else {
        html.push_str("<ul>\n");
        for item in items {
            html.push_str(&format!("  <li>{}</li>\n", escape(item)));
        }
        html.push_str("</ul>");
    }
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::prelude::*;

    #[test]
    fn a_page_with_items_renders_a_list() -> TestResult {
        let page = render_page("Tasks", &["buy milk", "walk dog"]);
        expect!(page).to_match_inline_snapshot(
            r#"
            <!doctype html>
            <title>Tasks</title>
            <h1>Tasks</h1>
            <ul>
              <li>buy milk</li>
              <li>walk dog</li>
            </ul>
            "#,
        )
    }

    #[test]
    fn an_empty_page_renders_the_placeholder() -> TestResult {
        let page = render_page("Tasks", &[]);
        expect!(page).to_match_inline_snapshot(
            r#"
            <!doctype html>
            <title>Tasks</title>
            <h1>Tasks</h1>
            <p>nothing here yet</p>
            "#,
        )
    }

    #[test]
    fn html_special_characters_are_escaped() -> TestResult {
        let page = render_page("A & B", &["1 < 2", "3 > 2"]);
        expect!(page).to_match_inline_snapshot(
            r#"
            <!doctype html>
            <title>A &amp; B</title>
            <h1>A &amp; B</h1>
            <ul>
              <li>1 &lt; 2</li>
              <li>3 &gt; 2</li>
            </ul>
            "#,
        )
    }

    #[test]
    fn the_rendered_page_still_supports_ordinary_matchers() -> TestResult {
        // A snapshot is not the only tool: a targeted matcher is clearer when
        // the test cares about one fact, not the whole output.
        let page = render_page("Home", &["welcome"]);
        expect!(&page).to(starts_with("<!doctype html>"))?;
        expect!(&page).to(contains_str("<li>welcome</li>"))?;
        Ok(())
    }
}
