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
