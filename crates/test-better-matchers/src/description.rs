//! [`Description`]: a composable, human-readable account of what a matcher
//! expects.
//!
//! A `Description` is a tree, not a string. Combinators (`not`, `all_of`,
//! `any_of`) build a description out of their children's descriptions, and
//! nested matchers (`some(ok(eq(..)))`) nest them. Text is produced once, at
//! the end, by [`Display`](std::fmt::Display) — so the structure is still
//! inspectable right up until it is rendered.

use std::borrow::Cow;
use std::fmt;

/// A composable description of a matcher's expectation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    node: Node,
}

/// The tree behind a [`Description`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    /// A leaf: a finished phrase such as `equal to 4`.
    Text(Cow<'static, str>),
    /// Negation: `not <child>`.
    Not(Box<Node>),
    /// Conjunction: every child must hold. Flattened on construction.
    All(Vec<Node>),
    /// Disjunction: at least one child must hold. Flattened on construction.
    Any(Vec<Node>),
    /// A `header:` line with `child` rendered indented beneath it. This is how
    /// nested matchers (`some(ok(..))`) keep their expected blocks aligned.
    Labeled {
        /// The line shown above the indented child.
        header: Cow<'static, str>,
        /// The nested description.
        child: Box<Node>,
    },
}

impl Description {
    /// A leaf description: a finished phrase like `equal to 4`.
    #[must_use]
    pub fn text(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            node: Node::Text(text.into()),
        }
    }

    /// Combines this description with `other` under conjunction. Nested
    /// conjunctions are flattened, so `a.and(b).and(c)` is a single `All`.
    #[must_use]
    pub fn and(self, other: Description) -> Self {
        let mut parts = match self.node {
            Node::All(parts) => parts,
            node => vec![node],
        };
        match other.node {
            Node::All(more) => parts.extend(more),
            node => parts.push(node),
        }
        Self {
            node: Node::All(parts),
        }
    }

    /// Combines this description with `other` under disjunction. Nested
    /// disjunctions are flattened, so `a.or(b).or(c)` is a single `Any`.
    #[must_use]
    pub fn or(self, other: Description) -> Self {
        let mut parts = match self.node {
            Node::Any(parts) => parts,
            node => vec![node],
        };
        match other.node {
            Node::Any(more) => parts.extend(more),
            node => parts.push(node),
        }
        Self {
            node: Node::Any(parts),
        }
    }

    /// Places `child` indented beneath a `header:` line. Used by nested
    /// matchers to keep their expected blocks readable.
    #[must_use]
    pub fn labeled(header: impl Into<Cow<'static, str>>, child: Description) -> Self {
        Self {
            node: Node::Labeled {
                header: header.into(),
                child: Box::new(child.node),
            },
        }
    }
}

impl std::ops::Not for Description {
    type Output = Description;

    /// Negates this description. Double negation cancels, so `!!x` renders as
    /// `x`.
    fn not(self) -> Description {
        let node = match self.node {
            Node::Not(inner) => *inner,
            other => Node::Not(Box::new(other)),
        };
        Description { node }
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&render(&self.node))
    }
}

/// Renders a node to text. Each [`Node::Labeled`] level prefixes its child's
/// lines with two spaces; because the recursion does this at every level, a
/// chain of labels indents by a steady two spaces per level.
fn render(node: &Node) -> String {
    match node {
        Node::Text(text) => text.to_string(),
        Node::Not(inner) => format!("not {}", parenthesize_under_not(inner)),
        Node::All(parts) => parts
            .iter()
            .map(parenthesize_under_all)
            .collect::<Vec<_>>()
            .join(" and "),
        Node::Any(parts) => parts.iter().map(render).collect::<Vec<_>>().join(" or "),
        Node::Labeled { header, child } => {
            let body = render(child)
                .lines()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{header}:\n{body}")
        }
    }
}

/// `not` binds tighter than `and`/`or`, so a compound child is parenthesized.
fn parenthesize_under_not(node: &Node) -> String {
    match node {
        Node::All(_) | Node::Any(_) => format!("({})", render(node)),
        _ => render(node),
    }
}

/// Inside `and`, an `or` child is parenthesized; everything else binds at
/// least as tightly and needs no parentheses.
fn parenthesize_under_all(node: &Node) -> String {
    match node {
        Node::Any(_) => format!("({})", render(node)),
        _ => render(node),
    }
}

#[cfg(test)]
mod tests {
    use test_better_core::TestResult;

    use super::*;
    use crate::{check, eq};

    #[test]
    fn text_renders_verbatim() -> TestResult {
        check!(Description::text("equal to 4").to_string())
            .satisfies(eq("equal to 4".to_string()))?;
        Ok(())
    }

    #[test]
    fn not_negates_and_double_negation_cancels() -> TestResult {
        let base = Description::text("equal to 4");
        check!((!base.clone()).to_string()).satisfies(eq("not equal to 4".to_string()))?;
        check!((!!base).to_string()).satisfies(eq("equal to 4".to_string()))?;
        Ok(())
    }

    #[test]
    fn and_flattens_and_joins() -> TestResult {
        let combined = Description::text("greater than 0")
            .and(Description::text("less than 100"))
            .and(Description::text("even"));
        check!(combined.to_string())
            .satisfies(eq("greater than 0 and less than 100 and even".to_string()))?;
        Ok(())
    }

    #[test]
    fn or_flattens_and_joins() -> TestResult {
        let combined = Description::text("zero")
            .or(Description::text("one"))
            .or(Description::text("two"));
        check!(combined.to_string()).satisfies(eq("zero or one or two".to_string()))?;
        Ok(())
    }

    #[test]
    fn or_inside_and_is_parenthesized() -> TestResult {
        let combined = Description::text("positive")
            .and(Description::text("small").or(Description::text("huge")));
        check!(combined.to_string()).satisfies(eq("positive and (small or huge)".to_string()))?;
        Ok(())
    }

    #[test]
    fn not_of_compound_is_parenthesized() -> TestResult {
        let combined = !Description::text("a").and(Description::text("b"));
        check!(combined.to_string()).satisfies(eq("not (a and b)".to_string()))?;
        Ok(())
    }

    #[test]
    fn labeled_indents_the_child() -> TestResult {
        let described = Description::labeled("some", Description::text("equal to 42"));
        check!(described.to_string()).satisfies(eq("some:\n  equal to 42".to_string()))?;
        Ok(())
    }

    #[test]
    fn nested_labels_indent_two_spaces_per_level() -> TestResult {
        let described = Description::labeled(
            "some",
            Description::labeled("ok", Description::text("equal to 42")),
        );
        check!(described.to_string()).satisfies(eq("some:\n  ok:\n    equal to 42".to_string()))?;
        Ok(())
    }
}
