# liaise_proc_macro

The liaise methodology inside a proc macro, with no dependency on `liaise`.

`liaise` depends on `liaise_derive`, so a proc-macro crate (including
`liaise_derive` itself) cannot use `liaise` for its own errors. This crate is
the half a proc macro needs, and nothing more:

- `liaise_codes!` declares the error codes in one table: prefix, then
  `Variant = code => "message"` lines. Codes and prefix may be constants.
  Duplicate codes fail to compile.
- `LiaiseCode` renders a code exactly as `liaise` does: `[PREFIX0003] message`.
- `SynBuffer` folds many `syn::Error`s into one so a macro reports everything
  wrong at once. `err_at`, `err_at_ctx`, `err_spanned`, `err_spanned_ctx` turn
  a code plus a span or tokens into a `syn::Error`.

```rust
use liaise_proc_macro::{liaise_codes, LiaiseCode, SynBuffer};

liaise_codes! {
    prefix: "SHELL",
    pub(crate) Code {
        MissingCmd  = 1 => "missing #[shell(cmd = \"...\")] on struct",
        OnlyStructs = 2 => "only structs are supported",
    }
}

fn check(input: &syn::DeriveInput) -> Result<(), syn::Error> {
    let mut errs = SynBuffer::new();
    if !matches!(input.data, syn::Data::Struct(_)) {
        errs.push_spanned(&input.ident, Code::OnlyStructs);
    }
    errs.finish()
}
```

Dependencies: `syn`, `quote`, `proc-macro2`.

`liaise_derive` uses this for its own diagnostics. If you want the full
liaise surface (severity, reports, tokens, evidence) in a non-proc-macro crate,
use `liaise` itself.

## License

MIT or Apache-2.0, at your option.
