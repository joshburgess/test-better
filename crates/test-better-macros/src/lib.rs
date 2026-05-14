//! `test-better-macros`: procedural macros.
//!
//! Home of `matches_struct!`, `matches_tuple!`, `matches_variant!`, and the
//! `#[test_case]` attribute (PROJECT_BUILD_PLAN.md §8, §13), with `#[fixture]`
//! and the inline snapshot macros still to come (§13, §12).
//!
//! The structural matchers parse a *pattern* of inner matcher expressions and
//! emit a `Matcher` impl. The matcher holds a projection (a closure that pulls
//! the fields out of the value) plus one inner matcher per field; the
//! projection's type ties the matcher's type parameters to the real field
//! types, so the field types never have to be named in the macro. The
//! projection is threaded through a generated constructor function whose
//! signature carries the `Fn` bound, which is what makes the closure infer as
//! higher-ranked over the borrow.
//!
//! `#[test_case]` is an attribute macro: stacked `#[test_case(..)]` lines on one
//! function each become a generated `#[test]`, all gathered into a module named
//! for the function so the cases share a namespace.
//!
//! The generated code refers to the testing library through the `::test_better`
//! facade crate, so these macros are meant to be used via `test-better`, not by
//! depending on `test-better-macros` directly.

use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, Ident, Index, ItemFn, LitStr, Path, Token, braced, parenthesized};

/// A named-field pattern: `Path { field: matcher, ..., .. }`.
struct StructPattern {
    path: Path,
    fields: Vec<(Ident, Expr)>,
    rest: bool,
}

impl Parse for StructPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        let content;
        braced!(content in input);
        let (fields, rest) = parse_named_fields(&content)?;
        Ok(Self { path, fields, rest })
    }
}

/// A positional pattern: `Path(matcher, ..., ..)`.
struct TuplePattern {
    path: Path,
    elems: Vec<Expr>,
    rest: bool,
}

impl Parse for TuplePattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        let content;
        parenthesized!(content in input);
        let (elems, rest) = parse_positional_fields(&content)?;
        Ok(Self { path, elems, rest })
    }
}

/// The body of a variant pattern: struct-like, tuple-like, or unit.
enum VariantBody {
    Struct {
        fields: Vec<(Ident, Expr)>,
        rest: bool,
    },
    Tuple {
        elems: Vec<Expr>,
        rest: bool,
    },
    Unit,
}

/// A variant pattern: `Enum::Variant { .. }`, `Enum::Variant( .. )`, or
/// `Enum::Variant`.
struct VariantPattern {
    path: Path,
    body: VariantBody,
}

impl Parse for VariantPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        let body = if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            let (fields, rest) = parse_named_fields(&content)?;
            VariantBody::Struct { fields, rest }
        } else if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let (elems, rest) = parse_positional_fields(&content)?;
            VariantBody::Tuple { elems, rest }
        } else {
            VariantBody::Unit
        };
        Ok(Self { path, body })
    }
}

/// Parses `field: expr` entries, optionally ending with `..`. The `..`, when
/// present, must be the final element.
fn parse_named_fields(content: ParseStream) -> syn::Result<(Vec<(Ident, Expr)>, bool)> {
    let mut fields = Vec::new();
    let mut rest = false;
    while !content.is_empty() {
        if content.peek(Token![..]) {
            content.parse::<Token![..]>()?;
            rest = true;
            break;
        }
        let name: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let expr: Expr = content.parse()?;
        fields.push((name, expr));
        if content.is_empty() {
            break;
        }
        content.parse::<Token![,]>()?;
    }
    if !content.is_empty() {
        return Err(content.error("`..` must be the final element of the pattern"));
    }
    Ok((fields, rest))
}

/// Parses positional `expr` entries, optionally ending with `..`. The `..`,
/// when present, must be the final element.
fn parse_positional_fields(content: ParseStream) -> syn::Result<(Vec<Expr>, bool)> {
    let mut elems = Vec::new();
    let mut rest = false;
    while !content.is_empty() {
        if content.peek(Token![..]) {
            content.parse::<Token![..]>()?;
            rest = true;
            break;
        }
        elems.push(content.parse()?);
        if content.is_empty() {
            break;
        }
        content.parse::<Token![,]>()?;
    }
    if !content.is_empty() {
        return Err(content.error("`..` must be the final element of the pattern"));
    }
    Ok((elems, rest))
}

/// Splits `Enum::Variant` into the enum path (`Enum`) and the variant ident
/// (`Variant`).
fn split_variant_path(path: &Path) -> syn::Result<(Path, Ident)> {
    if path.segments.len() < 2 {
        return Err(syn::Error::new_spanned(
            path,
            "expected an enum variant path like `MyEnum::Variant`",
        ));
    }
    let kept = path.segments.len() - 1;
    let segments: Punctuated<syn::PathSegment, Token![::]> =
        path.segments.iter().take(kept).cloned().collect();
    let enum_path = Path {
        leading_colon: path.leading_colon,
        segments,
    };
    let variant_ident = match path.segments.last() {
        Some(seg) => seg.ident.clone(),
        None => return Err(syn::Error::new_spanned(path, "missing variant name")),
    };
    Ok((enum_path, variant_ident))
}

/// The per-field generated idents: the matcher type parameter, the field-type
/// parameter, the struct field holding the matcher, and the binding the
/// projection's output is destructured into.
struct FieldIdents {
    matcher_ty: Vec<Ident>,
    field_ty: Vec<Ident>,
    matcher_field: Vec<Ident>,
    binding: Vec<Ident>,
}

fn field_idents(n: usize) -> FieldIdents {
    FieldIdents {
        matcher_ty: (0..n).map(|i| format_ident!("__TbM{}", i)).collect(),
        field_ty: (0..n).map(|i| format_ident!("__TbF{}", i)).collect(),
        matcher_field: (0..n).map(|i| format_ident!("__tb_m{}", i)).collect(),
        binding: (0..n).map(|i| format_ident!("__tb_f{}", i)).collect(),
    }
}

/// The body of each field's check: run the inner matcher on the projected
/// field, and on failure return a mismatch whose expectation is labeled with
/// the field name.
fn field_check_blocks(
    matcher_field: &[Ident],
    binding: &[Ident],
    labels: &[String],
) -> Vec<TokenStream2> {
    matcher_field
        .iter()
        .zip(binding)
        .zip(labels)
        .map(|((field, bind), label)| {
            let label = label.as_str();
            quote! {
                {
                    let __tb_result = ::test_better::Matcher::check(&self.#field, #bind);
                    if !__tb_result.matched {
                        let __tb_inner = match __tb_result.failure {
                            ::core::option::Option::Some(__tb_mismatch) => __tb_mismatch,
                            ::core::option::Option::None => ::test_better::Mismatch::new(
                                ::test_better::Matcher::description(&self.#field),
                                "the field matcher reported failure without detail",
                            ),
                        };
                        return ::test_better::MatchResult::fail(::test_better::Mismatch {
                            expected: ::test_better::Description::labeled(
                                #label,
                                __tb_inner.expected,
                            ),
                            actual: __tb_inner.actual,
                            diff: __tb_inner.diff,
                        });
                    }
                }
            }
        })
        .collect()
}

/// Folds each field's labeled description together under conjunction.
fn description_fold(matcher_field: &[Ident], labels: &[String]) -> TokenStream2 {
    let mut parts = matcher_field.iter().zip(labels).map(|(field, label)| {
        let label = label.as_str();
        quote! {
            ::test_better::Description::labeled(
                #label,
                ::test_better::Matcher::description(&self.#field),
            )
        }
    });
    match parts.next() {
        Some(first) => {
            let mut acc = first;
            for part in parts {
                acc = quote! { #acc.and(#part) };
            }
            acc
        }
        None => quote! { ::test_better::Description::text("a matching value") },
    }
}

/// Wraps an exhaustiveness-checking statement in a never-called function, so a
/// missing or unknown field is a hard error from rustc's own pattern checking.
fn exhaustiveness_fn(target: &TokenStream2, stmt: Option<TokenStream2>) -> TokenStream2 {
    match stmt {
        Some(stmt) => quote! {
            #[allow(dead_code, unused_variables, irrefutable_let_patterns, clippy::all)]
            fn __tb_assert_exhaustive(__tb_value: &#target) {
                #stmt
            }
        },
        None => quote! {},
    }
}

/// Assembles a plain (struct or tuple struct) structural matcher.
///
/// `projection` is a closure `Fn(&Self) -> (&F0, &F1, ...)`. It is passed to
/// the generated `__tb_make`, whose signature carries the `Fn` bound; that is
/// what lets the closure infer as higher-ranked over the borrow. `__tb_make`'s
/// where-clause then pins the matcher's `Self` and field types.
fn gen_plain(
    target: &TokenStream2,
    labels: &[String],
    field_exprs: &[&Expr],
    projection: TokenStream2,
    exhaustiveness: Option<TokenStream2>,
) -> TokenStream2 {
    let idents = field_idents(labels.len());
    let FieldIdents {
        matcher_ty,
        field_ty,
        matcher_field,
        binding,
    } = &idents;
    let n = labels.len();
    let assertion = exhaustiveness_fn(target, exhaustiveness);

    if n == 0 {
        return quote! {
            {
                #[allow(non_camel_case_types, dead_code, clippy::all)]
                struct __TbStructuralMatcher<__TbP> {
                    __tb_project: __TbP,
                }

                #[allow(clippy::all)]
                impl<__TbS, __TbP> ::test_better::Matcher<__TbS>
                    for __TbStructuralMatcher<__TbP>
                where
                    __TbP: ::core::ops::Fn(&__TbS) -> (),
                {
                    fn check(&self, __tb_actual: &__TbS) -> ::test_better::MatchResult {
                        let () = (self.__tb_project)(__tb_actual);
                        ::test_better::MatchResult::pass()
                    }

                    fn description(&self) -> ::test_better::Description {
                        ::test_better::Description::text("a matching value")
                    }
                }

                #[allow(clippy::all)]
                fn __tb_make<__TbS, __TbP>(
                    __tb_project: __TbP,
                ) -> impl ::test_better::Matcher<__TbS>
                where
                    __TbP: ::core::ops::Fn(&__TbS) -> (),
                {
                    __TbStructuralMatcher { __tb_project }
                }

                #assertion

                __tb_make(#projection)
            }
        };
    }

    let checks = field_check_blocks(matcher_field, binding, labels);
    let desc = description_fold(matcher_field, labels);

    quote! {
        {
            #[allow(non_camel_case_types, dead_code, clippy::all)]
            struct __TbStructuralMatcher<__TbP, #( #matcher_ty, )*> {
                __tb_project: __TbP,
                #( #matcher_field: #matcher_ty, )*
            }

            #[allow(clippy::all)]
            impl<__TbS, #( #field_ty, )* __TbP, #( #matcher_ty, )*>
                ::test_better::Matcher<__TbS>
                for __TbStructuralMatcher<__TbP, #( #matcher_ty, )*>
            where
                __TbP: ::core::ops::Fn(&__TbS) -> ( #( &#field_ty, )* ),
                #( #matcher_ty: ::test_better::Matcher<#field_ty>, )*
            {
                fn check(&self, __tb_actual: &__TbS) -> ::test_better::MatchResult {
                    let ( #( #binding, )* ) = (self.__tb_project)(__tb_actual);
                    #( #checks )*
                    ::test_better::MatchResult::pass()
                }

                fn description(&self) -> ::test_better::Description {
                    #desc
                }
            }

            #[allow(clippy::all)]
            fn __tb_make<__TbS, #( #field_ty, )* __TbP, #( #matcher_ty, )*>(
                __tb_project: __TbP,
                #( #matcher_field: #matcher_ty, )*
            ) -> impl ::test_better::Matcher<__TbS>
            where
                __TbP: ::core::ops::Fn(&__TbS) -> ( #( &#field_ty, )* ),
                #( #matcher_ty: ::test_better::Matcher<#field_ty>, )*
            {
                __TbStructuralMatcher {
                    __tb_project,
                    #( #matcher_field, )*
                }
            }

            #assertion

            __tb_make(#projection, #( #field_exprs, )*)
        }
    }
}

fn gen_struct(path: &Path, fields: &[(Ident, Expr)], rest: bool) -> TokenStream2 {
    let target = quote! { #path };
    let labels: Vec<String> = fields.iter().map(|(name, _)| name.to_string()).collect();
    let field_exprs: Vec<&Expr> = fields.iter().map(|(_, expr)| expr).collect();
    let field_names: Vec<&Ident> = fields.iter().map(|(name, _)| name).collect();

    let projection = if fields.is_empty() {
        quote! { |_: &#path| () }
    } else {
        quote! { |__tb_subject: &#path| ( #( &__tb_subject.#field_names, )* ) }
    };

    let exhaustiveness = if rest {
        None
    } else {
        Some(quote! { let #path { #( #field_names: _, )* } = __tb_value; })
    };

    gen_plain(&target, &labels, &field_exprs, projection, exhaustiveness)
}

fn gen_tuple(path: &Path, elems: &[Expr], rest: bool) -> TokenStream2 {
    let target = quote! { #path };
    let labels: Vec<String> = (0..elems.len()).map(|i| i.to_string()).collect();
    let field_exprs: Vec<&Expr> = elems.iter().collect();
    let indices: Vec<Index> = (0..elems.len()).map(Index::from).collect();

    let projection = if elems.is_empty() {
        quote! { |_: &#path| () }
    } else {
        quote! { |__tb_subject: &#path| ( #( &__tb_subject.#indices, )* ) }
    };

    let exhaustiveness = if rest {
        None
    } else {
        let holes = elems.iter().map(|_| quote!(_));
        Some(quote! { let #path( #( #holes, )* ) = __tb_value; })
    };

    gen_plain(&target, &labels, &field_exprs, projection, exhaustiveness)
}

fn gen_variant(pattern: &VariantPattern) -> syn::Result<TokenStream2> {
    let (enum_path, variant_ident) = split_variant_path(&pattern.path)?;
    let path = &pattern.path;
    let target = quote! { #enum_path };
    let variant_name = variant_ident.to_string();
    let variant_label = format!("the {variant_name} variant");

    // The labels, the inner matcher expressions, the projection closure, and the
    // exhaustiveness assertion all differ by variant shape.
    let (labels, field_exprs, projection, exhaustiveness): (
        Vec<String>,
        Vec<&Expr>,
        TokenStream2,
        Option<TokenStream2>,
    ) = match &pattern.body {
        VariantBody::Struct { fields, rest } => {
            let labels: Vec<String> = fields.iter().map(|(name, _)| name.to_string()).collect();
            let field_exprs: Vec<&Expr> = fields.iter().map(|(_, expr)| expr).collect();
            let field_names: Vec<&Ident> = fields.iter().map(|(name, _)| name).collect();
            let bindings: Vec<Ident> = (0..fields.len())
                .map(|i| format_ident!("__tb_p{}", i))
                .collect();
            let projection = quote! {
                |__tb_subject: &#enum_path| match __tb_subject {
                    #path { #( #field_names: #bindings, )* .. } =>
                        ::core::option::Option::Some(( #( #bindings, )* )),
                    _ => ::core::option::Option::None,
                }
            };
            let exhaustiveness = if *rest {
                None
            } else {
                Some(quote! { if let #path { #( #field_names: _, )* } = __tb_value {} })
            };
            (labels, field_exprs, projection, exhaustiveness)
        }
        VariantBody::Tuple { elems, rest } => {
            let labels: Vec<String> = (0..elems.len()).map(|i| i.to_string()).collect();
            let field_exprs: Vec<&Expr> = elems.iter().collect();
            let bindings: Vec<Ident> = (0..elems.len())
                .map(|i| format_ident!("__tb_p{}", i))
                .collect();
            let projection = quote! {
                |__tb_subject: &#enum_path| match __tb_subject {
                    #path( #( #bindings, )* .. ) =>
                        ::core::option::Option::Some(( #( #bindings, )* )),
                    _ => ::core::option::Option::None,
                }
            };
            let exhaustiveness = if *rest {
                None
            } else {
                let holes = elems.iter().map(|_| quote!(_));
                Some(quote! { if let #path( #( #holes, )* ) = __tb_value {} })
            };
            (labels, field_exprs, projection, exhaustiveness)
        }
        VariantBody::Unit => {
            let projection = quote! {
                |__tb_subject: &#enum_path| match __tb_subject {
                    #path => ::core::option::Option::Some(()),
                    _ => ::core::option::Option::None,
                }
            };
            (Vec::new(), Vec::new(), projection, None)
        }
    };

    let idents = field_idents(labels.len());
    let FieldIdents {
        matcher_ty,
        field_ty,
        matcher_field,
        binding,
    } = &idents;
    let n = labels.len();
    let assertion = exhaustiveness_fn(&target, exhaustiveness);

    let wrong_variant = quote! {
        ::test_better::MatchResult::fail(::test_better::Mismatch::new(
            ::test_better::Description::text(#variant_label),
            ::std::format!("{:?}", __tb_actual),
        ))
    };

    if n == 0 {
        return Ok(quote! {
            {
                #[allow(non_camel_case_types, dead_code, clippy::all)]
                struct __TbVariantMatcher<__TbP> {
                    __tb_project: __TbP,
                }

                #[allow(clippy::all)]
                impl<__TbS, __TbP> ::test_better::Matcher<__TbS>
                    for __TbVariantMatcher<__TbP>
                where
                    __TbP: ::core::ops::Fn(&__TbS) -> ::core::option::Option<()>,
                    __TbS: ::core::fmt::Debug,
                {
                    fn check(&self, __tb_actual: &__TbS) -> ::test_better::MatchResult {
                        match (self.__tb_project)(__tb_actual) {
                            ::core::option::Option::Some(()) => {
                                ::test_better::MatchResult::pass()
                            }
                            ::core::option::Option::None => #wrong_variant,
                        }
                    }

                    fn description(&self) -> ::test_better::Description {
                        ::test_better::Description::text(#variant_label)
                    }
                }

                #[allow(clippy::all)]
                fn __tb_make<__TbS, __TbP>(
                    __tb_project: __TbP,
                ) -> impl ::test_better::Matcher<__TbS>
                where
                    __TbP: ::core::ops::Fn(&__TbS) -> ::core::option::Option<()>,
                    __TbS: ::core::fmt::Debug,
                {
                    __TbVariantMatcher { __tb_project }
                }

                #assertion

                __tb_make(#projection)
            }
        });
    }

    let checks = field_check_blocks(matcher_field, binding, &labels);
    let desc_inner = description_fold(matcher_field, &labels);
    let desc = quote! { ::test_better::Description::labeled(#variant_name, #desc_inner) };

    Ok(quote! {
        {
            #[allow(non_camel_case_types, dead_code, clippy::all)]
            struct __TbVariantMatcher<__TbP, #( #matcher_ty, )*> {
                __tb_project: __TbP,
                #( #matcher_field: #matcher_ty, )*
            }

            #[allow(clippy::all)]
            impl<__TbS, #( #field_ty, )* __TbP, #( #matcher_ty, )*>
                ::test_better::Matcher<__TbS>
                for __TbVariantMatcher<__TbP, #( #matcher_ty, )*>
            where
                __TbP: ::core::ops::Fn(&__TbS)
                    -> ::core::option::Option<( #( &#field_ty, )* )>,
                #( #matcher_ty: ::test_better::Matcher<#field_ty>, )*
                __TbS: ::core::fmt::Debug,
            {
                fn check(&self, __tb_actual: &__TbS) -> ::test_better::MatchResult {
                    match (self.__tb_project)(__tb_actual) {
                        ::core::option::Option::Some(( #( #binding, )* )) => {
                            #( #checks )*
                            ::test_better::MatchResult::pass()
                        }
                        ::core::option::Option::None => #wrong_variant,
                    }
                }

                fn description(&self) -> ::test_better::Description {
                    #desc
                }
            }

            #[allow(clippy::all)]
            fn __tb_make<__TbS, #( #field_ty, )* __TbP, #( #matcher_ty, )*>(
                __tb_project: __TbP,
                #( #matcher_field: #matcher_ty, )*
            ) -> impl ::test_better::Matcher<__TbS>
            where
                __TbP: ::core::ops::Fn(&__TbS)
                    -> ::core::option::Option<( #( &#field_ty, )* )>,
                #( #matcher_ty: ::test_better::Matcher<#field_ty>, )*
                __TbS: ::core::fmt::Debug,
            {
                __TbVariantMatcher {
                    __tb_project,
                    #( #matcher_field, )*
                }
            }

            #assertion

            __tb_make(#projection, #( #field_exprs, )*)
        }
    })
}

/// Matches a struct by applying an inner matcher to each named field.
///
/// Without a trailing `..` every field must be listed, exactly as in a struct
/// pattern; with `..` the unlisted fields are ignored.
///
/// ```ignore
/// use test_better::prelude::*;
/// use test_better::matches_struct;
///
/// #[derive(Debug)]
/// struct User {
///     name: String,
///     age: u32,
///     email: String,
/// }
///
/// fn check(user: User) -> TestResult {
///     expect!(user).to(matches_struct!(User {
///         name: eq(String::from("alice")),
///         age: gt(0u32),
///         email: contains_str("@"),
///         .. // remaining fields ignored
///     }))?;
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn matches_struct(input: TokenStream) -> TokenStream {
    match syn::parse::<StructPattern>(input) {
        Ok(pattern) => gen_struct(&pattern.path, &pattern.fields, pattern.rest).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Matches a tuple struct by applying an inner matcher to each positional
/// field.
///
/// Without a trailing `..` every element must be listed; with `..` the unlisted
/// trailing elements are ignored.
///
/// ```ignore
/// use test_better::prelude::*;
/// use test_better::matches_tuple;
///
/// #[derive(Debug)]
/// struct Point(i32, i32);
///
/// fn check(point: Point) -> TestResult {
///     expect!(point).to(matches_tuple!(Point(gt(0), lt(100))))?;
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn matches_tuple(input: TokenStream) -> TokenStream {
    match syn::parse::<TuplePattern>(input) {
        Ok(pattern) => gen_tuple(&pattern.path, &pattern.elems, pattern.rest).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Matches an enum value against a specific variant, applying an inner matcher
/// to each of that variant's fields.
///
/// A value of a different variant is a match failure, not a compile error. The
/// variant may be struct-like (`Enum::Variant { field: m, .. }`), tuple-like
/// (`Enum::Variant(m, ..)`), or unit (`Enum::Variant`). The enum type must be
/// `Debug` so a wrong-variant failure can render the value.
///
/// ```ignore
/// use test_better::prelude::*;
/// use test_better::matches_variant;
///
/// #[derive(Debug)]
/// enum Shape {
///     Circle { radius: f64 },
///     Rectangle(f64, f64),
/// }
///
/// fn check(shape: Shape) -> TestResult {
///     expect!(shape).to(matches_variant!(Shape::Circle { radius: gt(0.0) }))?;
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn matches_variant(input: TokenStream) -> TokenStream {
    let result = syn::parse::<VariantPattern>(input).and_then(|pattern| gen_variant(&pattern));
    match result {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// One `#[test_case(..)]` invocation: the argument expressions and an optional
/// `; "label"`.
struct TestCase {
    /// A span pointing at the invocation, used to place an arg-count error on
    /// the offending attribute rather than on the function.
    span: proc_macro2::Span,
    /// The expressions passed positionally to the annotated function.
    args: Vec<Expr>,
    /// The case label after `;`, if one was given.
    label: Option<LitStr>,
}

impl Parse for TestCase {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let mut args = Vec::new();
        let mut label = None;
        while !input.is_empty() {
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
                label = Some(input.parse::<LitStr>()?);
                if !input.is_empty() {
                    return Err(input.error("unexpected tokens after the test-case label"));
                }
                break;
            }
            args.push(input.parse()?);
            if input.is_empty() || input.peek(Token![;]) {
                continue;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(Self { span, args, label })
    }
}

/// Reads one `#[test_case]` attribute. A bare `#[test_case]` (no parentheses) is
/// a zero-argument, unlabeled case; anything else is parsed as a [`TestCase`].
fn parse_test_case_attr(attribute: &syn::Attribute) -> syn::Result<TestCase> {
    match &attribute.meta {
        syn::Meta::Path(_) => Ok(TestCase {
            span: attribute.span(),
            args: Vec::new(),
            label: None,
        }),
        _ => attribute.parse_args::<TestCase>(),
    }
}

/// Whether an attribute is `#[test_case]` (matched on the final path segment so
/// a fully qualified `#[test_better::test_case]` is recognized too).
fn is_test_case_attr(attribute: &syn::Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test_case")
}

/// Turns a case label into a valid, lowercased identifier fragment: every
/// character that is not ASCII alphanumeric becomes `_`, and a leading digit
/// gets an `_` prefix. An empty result falls back to `case`.
fn sanitize_ident(label: &str) -> String {
    let mut out: String = label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push_str("case");
    }
    if out.starts_with(|ch: char| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Expands the stacked `#[test_case]` attributes on `func` into one `#[test]`
/// per case, all wrapped in a module named for the function.
fn test_case_impl(first: TestCase, mut func: ItemFn) -> syn::Result<TokenStream2> {
    // The topmost `#[test_case]` is handed to us as `attr`; the rest are still
    // attached to the function. Split the remaining attributes into further
    // cases and everything else (`#[ignore]`, doc comments, ...), which is
    // forwarded onto every generated test.
    let mut cases = vec![first];
    let mut forwarded = Vec::new();
    for attribute in std::mem::take(&mut func.attrs) {
        if is_test_case_attr(&attribute) {
            cases.push(parse_test_case_attr(&attribute)?);
        } else {
            forwarded.push(attribute);
        }
    }

    let fn_name = func.sig.ident.clone();
    let fn_name_str = fn_name.to_string();
    let ret = func.sig.output.clone();
    let expected_arity = func.sig.inputs.len();
    // A `-> ()` (explicit or implicit) test cannot carry failure context; only
    // a value-returning test (the `-> TestResult` shape) is wrapped.
    let returns_value = match &func.sig.output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => {
            !matches!(&**ty, syn::Type::Tuple(tuple) if tuple.elems.is_empty())
        }
    };

    let mut used_names: HashSet<String> = HashSet::new();
    let mut tests = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        if case.args.len() != expected_arity {
            return Err(syn::Error::new(
                case.span,
                format!(
                    "this `#[test_case]` passes {} argument(s) but `{fn_name_str}` takes {}",
                    case.args.len(),
                    expected_arity,
                ),
            ));
        }

        let base = match &case.label {
            Some(label) => sanitize_ident(&label.value()),
            None => format!("case_{index}"),
        };
        // Two labels that sanitize to the same identifier (or a label that
        // collides with a `case_N` default) are disambiguated by index.
        let name = if used_names.contains(&base) {
            format!("{base}_{index}")
        } else {
            base
        };
        used_names.insert(name.clone());
        let test_ident = format_ident!("{name}");

        let args = &case.args;
        let args_rendered = args
            .iter()
            .map(|arg| quote!(#arg).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let label_part = match &case.label {
            Some(label) => format!("{:?}", label.value()),
            None => format!("#{index}"),
        };
        let context_msg = format!("test case {label_part}: {fn_name_str}({args_rendered})");

        let body = if returns_value {
            quote! {
                ::test_better::ContextExt::context(#fn_name(#(#args),*), #context_msg)
            }
        } else {
            quote! { #fn_name(#(#args),*); }
        };

        // `pub(super)` so the generated test stays addressable from the file
        // that wrote the `#[test_case]` (e.g. to drive an `#[ignore]`d failing
        // case directly), without widening visibility any further.
        tests.push(quote! {
            #(#forwarded)*
            #[test]
            pub(super) fn #test_ident() #ret {
                #body
            }
        });
    }

    Ok(quote! {
        mod #fn_name {
            #[allow(unused_imports)]
            use super::*;

            #func

            #(#tests)*
        }
    })
}

/// Generates one `#[test]` per `#[test_case(..)]` line on a function.
///
/// Each attribute lists the positional arguments for one run, optionally
/// followed by `; "label"`. The cases are gathered into a module named for the
/// annotated function, so a labeled case is addressable as
/// `the_fn::the_label`; an unlabeled case becomes `the_fn::case_N`. The
/// original function stays callable inside that module as a helper.
///
/// When the function returns a value (the usual `-> TestResult` shape), each
/// generated test wraps the call in failure context carrying the case label
/// and the rendered arguments, so a failure names the case that produced it.
///
/// ```ignore
/// use test_better::prelude::*;
///
/// #[test_case("alice", 30 ; "common case")]
/// #[test_case("",      0  ; "empty name")]
/// fn validates_user(name: &str, age: u32) -> TestResult {
///     expect!(name.len()).to(ge(age as usize))
/// }
/// ```
///
/// A `#[test_case]` whose argument count does not match the function's
/// parameter count is a compile error, as is trailing junk after the label.
/// Other attributes on the function (`#[ignore]`, doc comments) are forwarded
/// onto every generated test.
#[proc_macro_attribute]
pub fn test_case(attr: TokenStream, item: TokenStream) -> TokenStream {
    let first = match syn::parse::<TestCase>(attr) {
        Ok(case) => case,
        Err(error) => return error.to_compile_error().into(),
    };
    let func = match syn::parse::<ItemFn>(item) {
        Ok(func) => func,
        Err(error) => return error.to_compile_error().into(),
    };
    match test_case_impl(first, func) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
