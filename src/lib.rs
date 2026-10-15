//! The liaise methodology inside a proc macro, with no dependency on `liaise`.
//!
//! `liaise` depends on `liaise_derive`, so a proc-macro crate (including
//! `liaise_derive` itself) cannot depend on `liaise` for its own errors. This
//! crate is the half a proc macro needs:
//!
//! - [`liaise_codes!`] declares the crate's error codes in one table: prefix,
//!   then `Variant = code => "message"` lines, as `register_liaise_errors!`
//!   did before it needed the derive;
//! - [`LiaiseCode`] renders them exactly as `liaise` does: `[PREFIX0003] message`;
//! - [`SynBuffer`] folds many `syn::Error`s into one, and the `err_*` helpers
//!   turn a code and a span or tokens into a `syn::Error`;
//! - [`ensure!`] checks one rule into the buffer and keeps going;
//! - [`collect_all`] runs every step and returns every value or every error.
//!
//! Dependencies: `syn`, `quote`, `proc-macro2`. Nothing else.
//!
//! ```
//! use liaise_proc_macro::{liaise_codes, LiaiseCode, SynBuffer};
//!
//! liaise_codes! {
//!     prefix: "DEMO",
//!     pub(crate) Code {
//!         OnlyEnums = 1 => "can only be derived for enums",
//!         MissingCode = 2 => "missing code = <u16> on variant",
//!     }
//! }
//!
//! assert_eq!(Code::MissingCode.render(), "[DEMO0002] missing code = <u16> on variant");
//!
//! let mut errs = SynBuffer::new();
//! errs.push_at(proc_macro2::Span::call_site(), Code::OnlyEnums);
//! errs.push_at(proc_macro2::Span::call_site(), Code::MissingCode);
//! assert!(errs.finish().is_err());
//! ```

use core::fmt::Display;
use proc_macro2::Span;
use quote::ToTokens;
use syn::Error;

/// A diagnostic code a proc macro raises. Rendered as `[PREFIX0003] message`,
/// the same text `liaise::Liaise::render` produces, so the two read alike.
pub trait LiaiseCode: Copy {
    /// Registry prefix, e.g. `"LIAISE"`.
    const PREFIX: &'static str;
    /// The numeric code within the prefix.
    fn code_id(self) -> u16;
    /// The message. Static: proc-macro messages are written once, in the table.
    fn message(self) -> &'static str;
    /// `PREFIX0003`.
    fn code(self) -> String {
        format!("{}{:04}", Self::PREFIX, self.code_id())
    }
    /// `[PREFIX0003] message`.
    fn render(self) -> String {
        format!("[{}] {}", self.code(), self.message())
    }
}

/// Declare a proc macro's error codes in one table.
///
/// ```
/// # use liaise_proc_macro::{liaise_codes, LiaiseCode};
/// liaise_codes! {
///     prefix: "SHELL",
///     pub Code {
///         MissingCmd = 1 => "missing #[shell(cmd = \"...\")] on struct",
///         OnlyStructs = 2 => "only structs are supported",
///     }
/// }
/// assert_eq!(Code::OnlyStructs.code(), "SHELL0002");
/// ```
///
/// Codes and the prefix may be literals or constants. Duplicate codes are a
/// compile error (`ALL_CODES` is checked at compile time).
#[macro_export]
macro_rules! liaise_codes {
    (
        prefix: $prefix:expr,
        $vis:vis $name:ident {
            $($variant:ident = $code:expr => $msg:expr),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $name {
            $($variant),*
        }

        impl $name {
            /// Every code in the table, in declaration order.
            pub const ALL_CODES: &'static [u16] = &[ $($code),* ];
        }

        // a free const is evaluated by `cargo check` too, not only at codegen
        const _: () = $crate::__assert_unique_codes($name::ALL_CODES);

        impl $crate::LiaiseCode for $name {
            const PREFIX: &'static str = $prefix;
            fn code_id(self) -> u16 {
                match self { $(Self::$variant => $code),* }
            }
            fn message(self) -> &'static str {
                match self { $(Self::$variant => $msg),* }
            }
        }
    };
}

/// Check one rule, record its code in the buffer if it fails, and keep going.
///
/// `ensure!(errs, cond, tokens => Code)` pushes `Code` at `tokens` when `cond`
/// is false; `ensure!(errs, cond, tokens => Code, ctx)` adds context after the
/// message. The macro is an expression: `true` when the rule held, so a check
/// that leaves nothing to build from can skip the item:
///
/// ```
/// # use liaise_proc_macro::{ensure, liaise_codes, SynBuffer};
/// liaise_codes! { prefix: "DEMO", Code { NeedsBool = 1 => "needs a bool" } }
/// let mut errs = SynBuffer::new();
/// let ty = proc_macro2::Ident::new("u8", proc_macro2::Span::call_site());
/// let ok = ensure!(errs, ty == "bool", &ty => Code::NeedsBool);
/// assert!(!ok);
/// assert_eq!(errs.finish().unwrap_err().to_string(), "[DEMO0001] needs a bool");
/// ```
#[macro_export]
macro_rules! ensure {
    ($errs:expr, $cond:expr, $at:expr => $code:expr) => {{
        let held: bool = $cond;
        if !held {
            $errs.push_spanned($at, $code);
        }
        held
    }};
    ($errs:expr, $cond:expr, $at:expr => $code:expr, $ctx:expr) => {{
        let held: bool = $cond;
        if !held {
            $errs.push_spanned_ctx($at, $code, $ctx);
        }
        held
    }};
}

/// Compile-time duplicate check used by [`liaise_codes!`]. Not public API.
#[doc(hidden)]
pub const fn __assert_unique_codes(codes: &[u16]) {
    let mut i = 0;
    while i < codes.len() {
        let mut j = i + 1;
        while j < codes.len() {
            assert!(codes[i] != codes[j], "liaise_codes!: duplicate code in table");
            j += 1;
        }
        i += 1;
    }
}

/// An error at a span.
pub fn err_at<C: LiaiseCode>(span: Span, code: C) -> Error {
    Error::new(span, code.render())
}

/// An error at a span, with extra context after the message.
pub fn err_at_ctx<C: LiaiseCode>(span: Span, code: C, ctx: impl Display) -> Error {
    Error::new(span, format!("{}: {}", code.render(), ctx))
}

/// An error pointing at `tokens`.
pub fn err_spanned<C: LiaiseCode>(tokens: impl ToTokens, code: C) -> Error {
    Error::new_spanned(tokens, code.render())
}

/// An error pointing at `tokens`, with extra context after the message.
pub fn err_spanned_ctx<C: LiaiseCode>(tokens: impl ToTokens, code: C, ctx: impl Display) -> Error {
    Error::new_spanned(tokens, format!("{}: {}", code.render(), ctx))
}

/// Run every step: all the values if each one worked, otherwise every error
/// together. The collecting form of `.collect::<Result<Vec<_>, _>>()`, which
/// stops at the first.
pub fn collect_all<T>(results: impl IntoIterator<Item = Result<T, Error>>) -> Result<Vec<T>, Error> {
    let mut errs = SynBuffer::new();
    let values: Vec<T> = results.into_iter().filter_map(|r| errs.absorb(r)).collect();
    errs.finish_with(values)
}

/// Folds many `syn::Error`s into one, so a macro reports everything wrong at
/// once instead of the first thing. `finish` is the single exit.
#[derive(Default)]
pub struct SynBuffer {
    err: Option<Error>,
}

impl SynBuffer {
    pub fn new() -> Self {
        Self { err: None }
    }
    pub fn is_empty(&self) -> bool {
        self.err.is_none()
    }
    /// Add `e` (and every error folded into it). An identical message at an
    /// identical place is shown once by the compiler itself.
    pub fn push(&mut self, e: Error) {
        match &mut self.err {
            Some(existing) => existing.combine(e),
            None => self.err = Some(e),
        }
    }
    pub fn push_at<C: LiaiseCode>(&mut self, span: Span, code: C) {
        self.push(err_at(span, code));
    }
    pub fn push_at_ctx<C: LiaiseCode>(&mut self, span: Span, code: C, ctx: impl Display) {
        self.push(err_at_ctx(span, code, ctx));
    }
    pub fn push_spanned<C: LiaiseCode>(&mut self, tokens: impl ToTokens, code: C) {
        self.push(err_spanned(tokens, code));
    }
    pub fn push_spanned_ctx<C: LiaiseCode>(&mut self, tokens: impl ToTokens, code: C, ctx: impl Display) {
        self.push(err_spanned_ctx(tokens, code, ctx));
    }
    /// Take a step that returns `Result`: its value if it worked, otherwise its
    /// error goes in the buffer and the caller gets `None` and keeps going.
    pub fn absorb<T>(&mut self, result: Result<T, Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(e) => {
                self.push(e);
                None
            }
        }
    }
    /// `Ok(value)` if nothing was pushed, else the folded error.
    pub fn finish_with<T>(self, value: T) -> Result<T, Error> {
        self.finish().map(|()| value)
    }
    /// `Ok(())` if nothing was pushed, else the folded error.
    pub fn finish(self) -> Result<(), Error> {
        match self.err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
    /// The folded error as `compile_error!` tokens, or empty if nothing was pushed.
    pub fn into_compile_error(self) -> proc_macro2::TokenStream {
        match self.err {
            Some(e) => e.to_compile_error(),
            None => proc_macro2::TokenStream::new(),
        }
    }
}
