//! Minimal `{{ expr }}` interpolation over raw bytes.
//!
//! Substitutes `{{ expr }}` tokens in a byte slice (typically config file
//! contents read before parsing) with values supplied by a [`Resolver`]. The
//! expression syntax is opaque to this module — the resolver decides what an
//! `expr` means; [`EnvResolver`] reads `env::NAME`, for example. Resolvers
//! compose: a tuple of resolvers is itself a [`Resolver`] that tries each in
//! order, first match wins.
//!
//! ```rust,ignore
//! use conflaguration::template::{EnvResolver, Resolvable};
//!
//! // expands {{ env::HOME }} to the HOME env var, errors on anything unresolved
//! let rendered = "path = {{ env::HOME }}".resolve(&EnvResolver)?;
//! ```

use std::borrow::Cow;

/// Resolves a template expression to its replacement bytes, or `None` if it
/// can't (which strict [`render`] turns into an error and [`render_lenient`]
/// leaves untouched).
pub trait Resolver: Send + Sync {
    /// Look up `expr` (the trimmed contents of a `{{ }}`), returning its bytes.
    fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>>;
}

impl<A: Resolver, B: Resolver> Resolver for (A, B) {
    fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>> {
        self.0.resolve(expr).or_else(|| self.1.resolve(expr))
    }
}

impl<A: Resolver, B: Resolver, C: Resolver> Resolver for (A, B, C) {
    fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>> {
        self.0.resolve(expr)
            .or_else(|| self.1.resolve(expr))
            .or_else(|| self.2.resolve(expr))
    }
}

impl<A: Resolver, B: Resolver, C: Resolver, D: Resolver> Resolver for (A, B, C, D) {
    fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>> {
        self.0.resolve(expr)
            .or_else(|| self.1.resolve(expr))
            .or_else(|| self.2.resolve(expr))
            .or_else(|| self.3.resolve(expr))
    }
}

/// Resolver for `env::NAME` expressions, reading the named environment variable.
pub struct EnvResolver;

impl Resolver for EnvResolver {
    fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>> {
        let var_name = expr.strip_prefix("env::")?;
        environs::resolve::<String>(&[var_name]).ok().map(|val| Cow::Owned(val.into_bytes()))
    }
}

/// Failure from strict [`render`].
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// No resolver produced a value for the expression.
    #[error("unresolved: {expr} at offset {offset}")]
    Unresolved {
        /// The unresolved expression text.
        expr: String,
        /// Byte offset of the opening `{{` in the input.
        offset: usize,
    },

    /// The bytes between `{{` and `}}` were not valid UTF-8.
    #[error("invalid utf-8 in expression at offset {offset}")]
    InvalidExpr {
        /// Byte offset of the opening `{{` in the input.
        offset: usize,
    },
}

/// Expand every `{{ expr }}` token via `resolver`. Returns the input unchanged
/// (borrowed) when it contains no tokens; errors on an unresolved expression or
/// non-UTF-8 token. Unclosed `{{` is left literal.
pub fn render<'a, R: Resolver>(
    input: &'a [u8],
    resolver: &R,
) -> Result<Cow<'a, [u8]>, TemplateError> {
    if !has_template(input) {
        return Ok(Cow::Borrowed(input));
    }

    let mut result = Vec::with_capacity(input.len());
    let mut pos = 0;

    while pos < input.len() {
        if pos + 1 < input.len()
            && input[pos] == b'{'
            && input[pos + 1] == b'{'
            && let Some(end) = find_closing(input, pos + 2)
        {
            let expr = &input[pos + 2..end];
            let expr_str = std::str::from_utf8(expr).map_err(|_| TemplateError::InvalidExpr { offset: pos })?;
            let trimmed = expr_str.trim();
            let value = resolver
                .resolve(trimmed)
                .ok_or_else(|| TemplateError::Unresolved { expr: trimmed.to_string(), offset: pos })?;
            result.extend_from_slice(&value);
            pos = end + 2;
            continue;
        }
        result.push(input[pos]);
        pos += 1;
    }

    Ok(Cow::Owned(result))
}

/// Like [`render`] but never fails: an unresolved or non-UTF-8 token is copied
/// through verbatim (`{{ … }}` and all) instead of producing an error.
pub fn render_lenient<'a, R: Resolver>(
    input: &'a [u8],
    resolver: &R,
) -> Cow<'a, [u8]> {
    if !has_template(input) {
        return Cow::Borrowed(input);
    }

    let mut result = Vec::with_capacity(input.len());
    let mut pos = 0;

    while pos < input.len() {
        if pos + 1 < input.len()
            && input[pos] == b'{'
            && input[pos + 1] == b'{'
            && let Some(end) = find_closing(input, pos + 2)
        {
            let expr = &input[pos + 2..end];
            if let Ok(expr_str) = std::str::from_utf8(expr) {
                match resolver.resolve(expr_str.trim()) {
                    Some(value) => result.extend_from_slice(&value),
                    None => {
                        result.extend_from_slice(b"{{");
                        result.extend_from_slice(expr);
                        result.extend_from_slice(b"}}");
                    }
                }
            } else {
                result.extend_from_slice(&input[pos..end + 2]);
            }
            pos = end + 2;
            continue;
        }
        result.push(input[pos]);
        pos += 1;
    }

    Cow::Owned(result)
}

/// Convenience entry point: call [`render`]/[`render_lenient`] directly on a
/// `str` or `[u8]` rather than passing it as an argument.
pub trait Resolvable {
    /// Strict render — see [`render`].
    fn resolve<R: Resolver>(&self, resolver: &R) -> Result<Cow<'_, [u8]>, TemplateError>;

    /// Lenient render — see [`render_lenient`].
    fn resolve_lenient<R: Resolver>(&self, resolver: &R) -> Cow<'_, [u8]>;
}

impl Resolvable for [u8] {
    fn resolve<R: Resolver>(&self, resolver: &R) -> Result<Cow<'_, [u8]>, TemplateError> {
        render(self, resolver)
    }

    fn resolve_lenient<R: Resolver>(&self, resolver: &R) -> Cow<'_, [u8]> {
        render_lenient(self, resolver)
    }
}

impl Resolvable for str {
    fn resolve<R: Resolver>(&self, resolver: &R) -> Result<Cow<'_, [u8]>, TemplateError> {
        render(self.as_bytes(), resolver)
    }

    fn resolve_lenient<R: Resolver>(&self, resolver: &R) -> Cow<'_, [u8]> {
        render_lenient(self.as_bytes(), resolver)
    }
}

fn has_template(input: &[u8]) -> bool {
    input.windows(2).any(|window| window == b"{{")
}

fn find_closing(input: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 1 < input.len() {
        if input[pos] == b'}' && input[pos + 1] == b'}' {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    struct FixedResolver(&'static [u8]);
    impl Resolver for FixedResolver {
        fn resolve(&self, _expr: &str) -> Option<Cow<'_, [u8]>> {
            Some(Cow::Borrowed(self.0))
        }
    }

    struct NeverResolver;
    impl Resolver for NeverResolver {
        fn resolve(&self, _expr: &str) -> Option<Cow<'_, [u8]>> {
            None
        }
    }

    #[test]
    fn no_templates_borrows() {
        let result = render(b"plain", &NeverResolver).expect("render");
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(&*result, b"plain");
    }

    #[test]
    fn resolved_returns_owned() {
        let result = render(b"{{x}}", &FixedResolver(b"y")).expect("render");
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(&*result, b"y");
    }

    #[test]
    fn env_resolver() {
        unsafe { std::env::set_var("CONFLAG_TPL_TEST", "works") };
        let result = render(b"{{env::CONFLAG_TPL_TEST}}", &EnvResolver).expect("render");
        assert_eq!(&*result, b"works");
    }

    #[test]
    fn unresolved_errors() {
        let err = render(b"{{missing}}", &NeverResolver).unwrap_err();
        assert!(matches!(err, TemplateError::Unresolved { .. }));
    }

    #[test]
    fn lenient_passes_through() {
        let result = render_lenient(b"{{missing}}", &NeverResolver);
        assert_eq!(&*result, b"{{missing}}");
    }

    #[test]
    fn tuple_composition() {
        struct AResolver;
        impl Resolver for AResolver {
            fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>> {
                if expr == "a" { Some(Cow::Borrowed(b"alpha")) } else { None }
            }
        }

        struct BResolver;
        impl Resolver for BResolver {
            fn resolve(&self, expr: &str) -> Option<Cow<'_, [u8]>> {
                if expr == "b" { Some(Cow::Borrowed(b"beta")) } else { None }
            }
        }

        let chain = (AResolver, BResolver);
        assert_eq!(&*render(b"{{a}}", &chain).expect("a"), b"alpha");
        assert_eq!(&*render(b"{{b}}", &chain).expect("b"), b"beta");
    }

    #[test]
    fn first_wins_in_chain() {
        let chain = (FixedResolver(b"first"), FixedResolver(b"second"));
        let result = render(b"{{anything}}", &chain).expect("render");
        assert_eq!(&*result, b"first");
    }

    #[test]
    fn binary_values() {
        struct BinResolver;
        impl Resolver for BinResolver {
            fn resolve(&self, _expr: &str) -> Option<Cow<'_, [u8]>> {
                Some(Cow::Owned(vec![0xFF, 0x00, 0xFE]))
            }
        }

        let result = render(b"[{{blob}}]", &BinResolver).expect("render");
        assert_eq!(&*result, &[b'[', 0xFF, 0x00, 0xFE, b']']);
    }

    #[test]
    fn unclosed_passes_through() {
        let result = render(b"hello {{ no close", &NeverResolver).expect("render");
        assert_eq!(&*result, b"hello {{ no close");
    }

    #[test]
    fn error_offset() {
        let err = render(b"hello {{bad}} world", &NeverResolver).unwrap_err();
        match err {
            TemplateError::Unresolved { expr, offset } => {
                assert_eq!(expr, "bad");
                assert_eq!(offset, 6);
            }
            other => panic!("expected Unresolved, got {other}"),
        }
    }

    #[test]
    fn resolvable_bytes() {
        let result = b"{{x}}".resolve(&FixedResolver(b"y")).expect("resolve");
        assert_eq!(&*result, b"y");
    }

    #[test]
    fn resolvable_str() {
        let result = "{{x}}".resolve(&FixedResolver(b"y")).expect("resolve");
        assert_eq!(&*result, b"y");
    }

    #[test]
    fn resolvable_lenient() {
        let result = "{{missing}}".resolve_lenient(&NeverResolver);
        assert_eq!(&*result, b"{{missing}}");
    }

    #[test]
    fn empty_input() {
        let result = render(b"", &NeverResolver).expect("render");
        assert!(result.is_empty());
        assert!(matches!(result, Cow::Borrowed(_)));
    }
}
