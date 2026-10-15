use liaise_proc_macro::{err_at, err_spanned_ctx, liaise_codes, LiaiseCode, SynBuffer};
use proc_macro2::Span;

const PFX: &str = "TBL";
const LATE: u16 = 30;

liaise_codes! {
    prefix: PFX,
    pub Code {
        First = 1 => "first thing",
        Late = LATE => "a const code",
    }
}

#[test]
fn renders_like_liaise() {
    assert_eq!(Code::First.code(), "TBL0001");
    assert_eq!(Code::First.render(), "[TBL0001] first thing");
    assert_eq!(Code::Late.code_id(), 30);
    assert_eq!(Code::ALL_CODES, &[1, 30]);
}

#[test]
fn errors_carry_the_rendered_code() {
    let e = err_at(Span::call_site(), Code::First);
    assert_eq!(e.to_string(), "[TBL0001] first thing");
    let e = err_spanned_ctx(quote::quote! { x }, Code::Late, "while parsing x");
    assert_eq!(e.to_string(), "[TBL0030] a const code: while parsing x");
}

#[test]
fn buffer_folds_everything_and_reports_once() {
    let mut errs = SynBuffer::new();
    assert!(errs.is_empty());
    assert!(SynBuffer::new().finish().is_ok());
    errs.push_at(Span::call_site(), Code::First);
    errs.push_spanned(quote::quote! { y }, Code::Late);
    let err = errs.finish().unwrap_err();
    let all: Vec<String> = err.into_iter().map(|e| e.to_string()).collect();
    assert_eq!(all, vec!["[TBL0001] first thing", "[TBL0030] a const code"]);
}

#[test]
fn ensure_records_failures_and_keeps_going() {
    use liaise_proc_macro::ensure;
    let mut errs = SynBuffer::new();
    let held = ensure!(errs, 1 + 1 == 2, quote::quote! { a } => Code::First);
    let broke = ensure!(errs, false, quote::quote! { b } => Code::First);
    let broke_ctx = ensure!(errs, false, quote::quote! { c } => Code::Late, "field `c`");
    assert!(held && !broke && !broke_ctx);
    let all: Vec<String> = errs.finish().unwrap_err().into_iter().map(|e| e.to_string()).collect();
    assert_eq!(all, vec!["[TBL0001] first thing", "[TBL0030] a const code: field `c`"]);
}

#[test]
fn absorb_keeps_values_and_collects_errors() {
    let mut errs = SynBuffer::new();
    assert_eq!(errs.absorb(Ok::<_, syn::Error>(7)), Some(7));
    assert_eq!(errs.absorb::<u8>(Err(err_at(Span::call_site(), Code::Late))), None);
    assert_eq!(errs.absorb::<u8>(Err(err_at(Span::call_site(), Code::First))), None);
    assert_eq!(errs.finish().unwrap_err().into_iter().count(), 2);
    assert_eq!(SynBuffer::new().finish_with("built").unwrap(), "built");
}

#[test]
fn collect_all_returns_every_error_or_every_value() {
    use liaise_proc_macro::collect_all;
    assert_eq!(collect_all([Ok::<_, syn::Error>(1), Ok(2)]).unwrap(), vec![1, 2]);
    let e = collect_all([
        Ok(1),
        Err(err_at(Span::call_site(), Code::First)),
        Ok(3),
        Err(err_at(Span::call_site(), Code::Late)),
    ])
    .unwrap_err();
    let all: Vec<String> = e.into_iter().map(|e| e.to_string()).collect();
    assert_eq!(all, vec!["[TBL0001] first thing", "[TBL0030] a const code"]);
}

#[test]
fn every_pushed_error_is_kept() {
    // folding is plain: nothing is merged here (the compiler shows an
    // identical message at an identical place once on its own)
    let mut errs = SynBuffer::new();
    errs.push_at(Span::call_site(), Code::First);
    errs.push_at(Span::call_site(), Code::First);
    let mut folded = err_at(Span::call_site(), Code::Late);
    folded.combine(err_at(Span::call_site(), Code::First));
    errs.push(folded);
    let all: Vec<String> = errs.finish().unwrap_err().into_iter().map(|e| e.to_string()).collect();
    assert_eq!(all, vec!["[TBL0001] first thing", "[TBL0001] first thing", "[TBL0030] a const code", "[TBL0001] first thing"]);
}
