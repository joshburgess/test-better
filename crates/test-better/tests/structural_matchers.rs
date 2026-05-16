//! Acceptance tests for the structural matcher macros (`matches_struct!`,
//! `matches_tuple!`, `matches_variant!`).
//!
//! These run from outside the matcher crate, through the `test-better` facade,
//! which is how the macros are meant to be used: their generated code refers to
//! `::test_better`.

use test_better::prelude::*;
use test_better::{matches_struct, matches_tuple, matches_variant};

#[derive(Debug)]
struct User {
    name: String,
    age: u32,
    email: String,
}

#[derive(Debug)]
struct Point(i32, i32);

#[derive(Debug)]
enum Shape {
    Circle { radius: f64 },
    Rectangle(f64, f64),
    Unit,
}

#[test]
fn matches_struct_checks_named_fields() -> TestResult {
    let user = User {
        name: String::from("alice"),
        age: 30,
        email: String::from("alice@example.com"),
    };
    check!(user).satisfies(matches_struct!(User {
        name: eq(String::from("alice")),
        age: gt(0u32),
        email: contains_str("@"),
    }))?;
    Ok(())
}

#[test]
fn matches_struct_rest_ignores_unlisted_fields() -> TestResult {
    let user = User {
        name: String::from("bob"),
        age: 41,
        email: String::from("bob@example.com"),
    };
    check!(user).satisfies(matches_struct!(User {
        name: eq(String::from("bob")),
        ..
    }))?;
    Ok(())
}

#[test]
fn matches_struct_failure_labels_the_field() -> TestResult {
    let user = User {
        name: String::from("carol"),
        age: 0,
        email: String::from("carol@example.com"),
    };
    let error = check!(user)
        .satisfies(matches_struct!(User {
            name: eq(String::from("carol")),
            age: gt(18u32),
            ..
        }))
        .expect_err("age 0 is not greater than 18");
    let rendered = error.to_string();
    check!(rendered.contains("age")).satisfies(is_true())?;
    Ok(())
}

#[test]
fn matches_tuple_checks_positional_fields() -> TestResult {
    check!(Point(3, 4)).satisfies(matches_tuple!(Point(gt(0), lt(100))))?;
    Ok(())
}

#[test]
fn matches_tuple_rest_ignores_trailing_elements() -> TestResult {
    check!(Point(7, 999)).satisfies(matches_tuple!(Point(eq(7), ..)))?;
    Ok(())
}

#[test]
fn matches_variant_checks_struct_like_variants() -> TestResult {
    check!(Shape::Circle { radius: 2.0 })
        .satisfies(matches_variant!(Shape::Circle { radius: gt(0.0) }))?;
    Ok(())
}

#[test]
fn matches_variant_checks_tuple_like_variants() -> TestResult {
    check!(Shape::Rectangle(3.0, 4.0)).satisfies(matches_variant!(Shape::Rectangle(gt(0.0), gt(0.0))))?;
    Ok(())
}

#[test]
fn matches_variant_checks_unit_variants() -> TestResult {
    check!(Shape::Unit).satisfies(matches_variant!(Shape::Unit))?;
    Ok(())
}

#[test]
fn matches_variant_rejects_a_different_variant() -> TestResult {
    let error = check!(Shape::Unit)
        .satisfies(matches_variant!(Shape::Circle { radius: gt(0.0) }))
        .expect_err("the Unit value is not a Circle");
    let rendered = error.to_string();
    check!(rendered.contains("Circle")).satisfies(is_true())?;
    Ok(())
}

#[test]
fn structural_matchers_nest() -> TestResult {
    // An inner structural matcher is just another matcher expression.
    let user = User {
        name: String::from("dave"),
        age: 25,
        email: String::from("dave@example.com"),
    };
    check!(Shape::Unit).satisfies(matches_variant!(Shape::Unit))?;
    check!(user).satisfies(matches_struct!(User {
        name: starts_with("da"),
        age: gt(18u32),
        ..
    }))?;
    Ok(())
}
