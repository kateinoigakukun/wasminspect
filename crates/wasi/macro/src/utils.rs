use proc_macro2::{Literal, TokenTree};
use std::path::PathBuf;

pub(crate) fn witx_path_from_arg(arg: TokenTree) -> PathBuf {
    let string;

    if let TokenTree::Literal(literal) = arg {
        let parsed = parse_string_literal(literal);

        string = parsed;
    } else {
        panic!("arguments must be string literals");
    }

    let root = PathBuf::from(std::env::var("WASI_ROOT").expect("WASI_ROOT"));
    return root.join(&string);
}

fn parse_string_literal(literal: Literal) -> String {
    let s = literal.to_string();
    assert!(
        s.starts_with('"') && s.ends_with('"'),
        "string literal must be enclosed in double-quotes"
    );

    let trimmed = s[1..s.len() - 1].to_owned();
    assert!(
        !trimmed.contains('"'),
        "string literal must not contain embedded quotes for now"
    );
    assert!(
        !trimmed.contains('\\'),
        "string literal must not contain embedded backslashes for now"
    );

    trimmed
}

pub(crate) fn witx_target_module_map_ident(arg: TokenTree) -> String {
    if let TokenTree::Ident(id) = arg {
        return id.to_string();
    } else {
        panic!("arguments must be string literals");
    }
}
