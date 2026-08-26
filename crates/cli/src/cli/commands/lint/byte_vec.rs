// Copyright 2025 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

//! `template::byte-vec` — flags `Vec<u8>` in types that cross the CBOR wire.
//!
//! `minicbor` (and `serde`) encode a `Vec<u8>` as a CBOR array of integers — `Array(Int(1),
//! Int(2), ...)` — which costs up to two bytes per byte plus per-element decoding, instead of the
//! dedicated CBOR byte string (major type 2). Template state is stored, and template arguments are
//! sent, in that encoding, so the difference shows up in substate size and transaction size.
//!
//! The check parses the crate's sources and reports:
//!   * fields of CBOR/serde-encoded types — the `pub` structs and enums inside a `#[template]`
//!     module (the macro derives `minicbor::Encode`/`Decode` for those) and any type deriving an
//!     encoding trait itself,
//!   * arguments and return types of the `pub` functions in a `#[template]` module.
//!
//! A field that already carries `#[cbor(with = ...)]`, `#[serde(with = ...)]` or a `skip` is left
//! alone.

use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{
    Attribute, Fields, FnArg, GenericArgument, ImplItem, Item, Pat, PathArguments, ReturnType, Type, Visibility,
};
use tokio::fs;

use super::{Finding, Severity, display_path};

const CODE: &str = "template::byte-vec";

/// Encoding derive names that put a type on the CBOR wire. Matched on the last path segment, so
/// `minicbor::Encode`, `tari_bor::Encode` and a plain `Encode` all count.
const ENCODING_DERIVES: &[&str] = &["Encode", "Decode", "CborLen", "Serialize", "Deserialize"];

pub(super) async fn check_byte_vecs(crate_dir: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in rust_sources(&crate_dir.join("src")).await {
        let Ok(src) = fs::read_to_string(&path).await else {
            continue;
        };
        // Sources that do not parse are the compiler's (and clippy's) to complain about.
        findings.extend(scan_source(&src, &display_path(&path)));
    }
    findings
}

/// Every `.rs` file under `dir`, in a stable order.
async fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(mut entries) = fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if entry.file_type().await.is_ok_and(|t| t.is_dir()) {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// How the offending type is encoded, which decides the attribute we suggest alongside `Bytes`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Encoding {
    Cbor,
    Serde,
}

impl Encoding {
    fn attribute(self) -> &'static str {
        match self {
            Encoding::Cbor => "#[cbor(with = \"minicbor::bytes\")]",
            Encoding::Serde => "#[serde(with = \"tari_template_lib::types::bytes\")]",
        }
    }
}

fn scan_source(src: &str, location: &str) -> Vec<Finding> {
    let Ok(file) = syn::parse_file(src) else {
        return Vec::new();
    };
    let mut scan = Scan {
        location,
        findings: Vec::new(),
    };
    scan.items(&file.items, None);
    scan.findings
}

/// Set while walking the body of a `#[template]` module.
#[derive(Clone, Copy)]
struct TemplateCtx {
    /// `#[template(skip_cbor_derives)]` — the author writes the derives, so a bare `pub struct` in
    /// the module is not necessarily encoded.
    skip_cbor_derives: bool,
}

struct Scan<'a> {
    location: &'a str,
    findings: Vec<Finding>,
}

impl Scan<'_> {
    fn items(&mut self, items: &[Item], template: Option<TemplateCtx>) {
        for item in items {
            match item {
                Item::Mod(item) => {
                    let template = template_ctx(&item.attrs).or(template);
                    if let Some((_, items)) = &item.content {
                        self.items(items, template);
                    }
                },
                Item::Struct(item) => {
                    if let Some(encoding) = encoding_of(&item.attrs, &item.vis, template) {
                        self.fields(&item.ident.to_string(), &item.fields, encoding);
                    }
                },
                Item::Enum(item) => {
                    if let Some(encoding) = encoding_of(&item.attrs, &item.vis, template) {
                        for variant in &item.variants {
                            let owner = format!("{}::{}", item.ident, variant.ident);
                            self.fields(&owner, &variant.fields, encoding);
                        }
                    }
                },
                // Only functions inside a `#[template]` module are called across the ABI boundary.
                Item::Impl(item) if template.is_some() => {
                    let owner = type_name(&item.self_ty);
                    for item in &item.items {
                        if let ImplItem::Fn(func) = item
                            && matches!(func.vis, Visibility::Public(_))
                        {
                            self.signature(&owner, &func.sig);
                        }
                    }
                },
                _ => {},
            }
        }
    }

    fn fields(&mut self, owner: &str, fields: &Fields, encoding: Encoding) {
        for (idx, field) in fields.iter().enumerate() {
            if is_exempt(&field.attrs) || !contains_byte_vec(&field.ty) {
                continue;
            }
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| idx.to_string(), ToString::to_string);
            let ty = render(&field.ty);
            let mut help = format!(
                "Use `Bytes` from the template prelude, which encodes as a CBOR byte string:\n\n    \
                 {name}: {},",
                ty.replace("Vec<u8>", "Bytes")
            );
            if attribute_applies(&field.ty, encoding) {
                help.push_str(&format!(
                    "\n\nor keep `Vec<u8>` and let the derive pick the byte encoding:\n\n    {}\n    {name}: {ty},",
                    encoding.attribute()
                ));
            }
            self.findings.push(
                Finding::new(
                    Severity::Warning,
                    CODE,
                    format!(
                        "`{owner}.{name}: {ty}` — a `Vec<u8>` field encodes as a CBOR array of integers \
                         (`Array(Int, Int, ...)`), up to twice the size of a byte string"
                    ),
                    self.at(field.span()),
                )
                .with_help(help),
            );
        }
    }

    fn signature(&mut self, owner: &str, sig: &syn::Signature) {
        let name = &sig.ident;
        for arg in &sig.inputs {
            let FnArg::Typed(arg) = arg else {
                continue;
            };
            if !contains_byte_vec(&arg.ty) {
                continue;
            }
            let arg_name = match &*arg.pat {
                Pat::Ident(pat) => pat.ident.to_string(),
                pat => squash(&pat.to_token_stream().to_string()),
            };
            let ty = render(&arg.ty);
            self.findings.push(
                Finding::new(
                    Severity::Warning,
                    CODE,
                    format!(
                        "argument `{arg_name}: {ty}` of `{owner}::{name}` — call arguments are CBOR-encoded, and a \
                         `Vec<u8>` encodes as an array of integers rather than a byte string"
                    ),
                    self.at(arg.span()),
                )
                .with_help(argument_help(&arg_name, &ty)),
            );
        }

        if let ReturnType::Type(_, ty) = &sig.output
            && contains_byte_vec(ty)
        {
            let rendered = render(ty);
            self.findings.push(
                Finding::new(
                    Severity::Warning,
                    CODE,
                    format!(
                        "`{owner}::{name}` returns `{rendered}` — the return value is CBOR-encoded, and a `Vec<u8>` \
                         encodes as an array of integers rather than a byte string"
                    ),
                    self.at(sig.output.span()),
                )
                .with_help(format!(
                    "Return `Bytes` from the template prelude instead:\n\n    pub fn {name}(..) -> {}",
                    rendered.replace("Vec<u8>", "Bytes")
                )),
            );
        }
    }

    fn at(&self, span: proc_macro2::Span) -> String {
        format!("{}:{}", self.location, span.start().line)
    }
}

fn argument_help(name: &str, ty: &str) -> String {
    format!(
        "Take `Bytes` from the template prelude instead — a function argument has no `#[cbor(..)]` attribute to fall \
         back on:\n\n    {name}: {}\n\nUse `Bytes::as_slice()`/`into_vec()` to get at the bytes, and \
         `Bytes::from_vec(..)` to build one.",
        ty.replace("Vec<u8>", "Bytes")
    )
}

/// `Some(_)` when values of the type reach the CBOR wire: a `pub` item in a `#[template]` module
/// (the macro derives the codec for those) or any item deriving an encoding trait itself.
fn encoding_of(attrs: &[Attribute], vis: &Visibility, template: Option<TemplateCtx>) -> Option<Encoding> {
    let derived = derived_encoding(attrs);
    if derived.is_some() {
        return derived;
    }
    match template {
        Some(ctx) if !ctx.skip_cbor_derives && matches!(vis, Visibility::Public(_)) => Some(Encoding::Cbor),
        _ => None,
    }
}

fn derived_encoding(attrs: &[Attribute]) -> Option<Encoding> {
    let mut encoding = None;
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let derives = attr.to_token_stream().to_string();
        for name in ENCODING_DERIVES {
            if !derives.contains(name) {
                continue;
            }
            // A type deriving both is encoded by minicbor on the template side, so the CBOR
            // attribute is the one to suggest.
            if *name == "Serialize" || *name == "Deserialize" {
                encoding.get_or_insert(Encoding::Serde);
            } else {
                encoding = Some(Encoding::Cbor);
            }
        }
    }
    encoding
}

/// `#[template]` / `#[template(..)]`, matched on the last path segment so an unimported macro path
/// (`#[tari_template_lib::prelude::template]`) is recognised too.
fn template_ctx(attrs: &[Attribute]) -> Option<TemplateCtx> {
    attrs
        .iter()
        .find(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "template")
        })
        .map(|attr| TemplateCtx {
            skip_cbor_derives: attr.to_token_stream().to_string().contains("skip_cbor_derives"),
        })
}

/// A field that already chooses its own codec, or is not encoded at all, is not our business.
fn is_exempt(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        if !path.is_ident("cbor") && !path.is_ident("serde") {
            return false;
        }
        let tokens = attr.to_token_stream().to_string();
        tokens.contains("with") || tokens.contains("skip")
    })
}

/// `#[cbor(with = "minicbor::bytes")]` handles `Vec<u8>` and `Option<Vec<u8>>`; the serde helper
/// module only handles a plain `Vec<u8>`. Anything deeper (`Vec<Vec<u8>>`, `HashMap<_, Vec<u8>>`,
/// ...) can only be fixed by changing the type.
fn attribute_applies(ty: &Type, encoding: Encoding) -> bool {
    if is_byte_vec(ty) {
        return true;
    }
    if encoding == Encoding::Serde {
        return false;
    }
    let Some(args) = generic_args(ty) else {
        return false;
    };
    matches!(ty, Type::Path(path)
        if path.path.segments.last().is_some_and(|s| s.ident == "Option")
            && args.len() == 1
            && args.iter().copied().any(is_byte_vec))
}

fn contains_byte_vec(ty: &Type) -> bool {
    if is_byte_vec(ty) {
        return true;
    }
    match ty {
        Type::Path(_) => generic_args(ty).is_some_and(|args| args.iter().copied().any(contains_byte_vec)),
        Type::Tuple(tuple) => tuple.elems.iter().any(contains_byte_vec),
        Type::Reference(inner) => contains_byte_vec(&inner.elem),
        Type::Slice(inner) => contains_byte_vec(&inner.elem),
        Type::Array(inner) => contains_byte_vec(&inner.elem),
        Type::Paren(inner) => contains_byte_vec(&inner.elem),
        Type::Group(inner) => contains_byte_vec(&inner.elem),
        _ => false,
    }
}

fn is_byte_vec(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    // `Vec<u8>`, `std::vec::Vec<u8>`, `alloc::vec::Vec<u8>`.
    if !path.path.segments.last().is_some_and(|s| s.ident == "Vec") {
        return false;
    }
    generic_args(ty).is_some_and(|args| args.len() == 1 && matches!(args[0], Type::Path(p) if p.path.is_ident("u8")))
}

fn generic_args(ty: &Type) -> Option<Vec<&Type>> {
    let Type::Path(path) = ty else {
        return None;
    };
    let PathArguments::AngleBracketed(args) = &path.path.segments.last()?.arguments else {
        return None;
    };
    Some(
        args.args
            .iter()
            .filter_map(|arg| match arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
    )
}

/// Types are printed from tokens, which spaces every token out — squash it back into something
/// that reads like source.
/// The `Foo` of `impl Foo`, for the message; anything exotic falls back to the printed type.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map_or_else(|| render(ty), |segment| segment.ident.to_string()),
        ty => render(ty),
    }
}

fn render(ty: &Type) -> String {
    squash(&ty.to_token_stream().to_string())
}

fn squash(tokens: &str) -> String {
    tokens.replace(' ', "").replace(',', ", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(src: &str) -> Vec<Finding> {
        scan_source(src, "src/lib.rs")
    }

    const TEMPLATE_WITH_BYTE_VEC: &str = r#"
#[template]
mod my_template {
    use super::*;

    pub struct MyTemplate {
        data: Vec<u8>,
    }

    impl MyTemplate {
        pub fn new(data: Vec<u8>) -> Self {
            Self { data }
        }

        pub fn get(&self) -> Vec<u8> {
            self.data.clone()
        }
    }
}
"#;

    #[test]
    fn flags_field_argument_and_return_type() {
        let findings = scan(TEMPLATE_WITH_BYTE_VEC);
        assert_eq!(findings.len(), 3, "{findings:?}");
        assert!(findings.iter().all(|f| f.severity == Severity::Warning));
        assert!(
            findings[0].message.contains("`MyTemplate.data: Vec<u8>`"),
            "{findings:?}"
        );
        assert!(findings[1].message.contains("argument `data: Vec<u8>`"), "{findings:?}");
        assert!(findings[2].message.contains("returns `Vec<u8>`"), "{findings:?}");
    }

    #[test]
    fn field_help_offers_bytes_and_the_cbor_attribute() {
        let findings = scan(TEMPLATE_WITH_BYTE_VEC);
        let help = findings[0].help.as_ref().unwrap();
        assert!(help.contains("data: Bytes,"), "{help}");
        assert!(help.contains(r#"#[cbor(with = "minicbor::bytes")]"#), "{help}");
    }

    #[test]
    fn argument_help_only_offers_bytes() {
        let findings = scan(TEMPLATE_WITH_BYTE_VEC);
        let help = findings[1].help.as_ref().unwrap();
        assert!(help.contains("data: Bytes"), "{help}");
        assert!(!help.contains("#[cbor(with"), "{help}");
    }

    #[test]
    fn locations_point_at_the_line() {
        let findings = scan(TEMPLATE_WITH_BYTE_VEC);
        assert_eq!(findings[0].location, "src/lib.rs:7");
    }

    #[test]
    fn bytes_is_clean() {
        let findings = scan(
            r#"
#[template]
mod my_template {
    pub struct MyTemplate {
        data: Bytes,
    }

    impl MyTemplate {
        pub fn new(data: Bytes) -> Self { Self { data } }
    }
}
"#,
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn cbor_with_attribute_exempts_a_field() {
        let findings = scan(
            r#"
#[template]
mod my_template {
    pub struct MyTemplate {
        #[cbor(with = "minicbor::bytes")]
        data: Vec<u8>,
        #[serde(with = "tari_template_lib::types::bytes")]
        other: Vec<u8>,
        #[cbor(skip)]
        cache: Vec<u8>,
    }
}
"#,
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn nested_byte_vecs_are_flagged_without_the_attribute_suggestion() {
        let findings = scan(
            r#"
#[template]
mod my_template {
    pub struct MyTemplate {
        chunks: Vec<Vec<u8>>,
        by_key: BTreeMap<String, Vec<u8>>,
        maybe: Option<Vec<u8>>,
    }
}
"#,
        );
        assert_eq!(findings.len(), 3, "{findings:?}");
        assert!(!findings[0].help.as_ref().unwrap().contains("#[cbor(with"));
        assert!(!findings[1].help.as_ref().unwrap().contains("#[cbor(with"));
        // `#[cbor(with = "minicbor::bytes")]` does handle `Option<Vec<u8>>`.
        assert!(findings[2].help.as_ref().unwrap().contains("#[cbor(with"));
    }

    #[test]
    fn derived_types_outside_a_template_module_are_checked() {
        let findings = scan(
            r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Payload {
    data: Vec<u8>,
}

#[derive(minicbor::Encode, minicbor::Decode)]
pub struct CborPayload {
    #[n(0)]
    data: Vec<u8>,
}

pub struct NotEncoded {
    data: Vec<u8>,
}
"#,
        );
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(
            findings[0]
                .help
                .as_ref()
                .unwrap()
                .contains(r#"#[serde(with = "tari_template_lib::types::bytes")]"#)
        );
        assert!(
            findings[1]
                .help
                .as_ref()
                .unwrap()
                .contains(r#"#[cbor(with = "minicbor::bytes")]"#)
        );
    }

    #[test]
    fn the_serde_helper_is_only_suggested_for_a_plain_vec() {
        let findings = scan(
            r#"
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Payload {
    maybe: Option<Vec<u8>>,
}
"#,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(!findings[0].help.as_ref().unwrap().contains("#[serde(with"));
    }

    #[test]
    fn enum_variants_are_checked() {
        let findings = scan(
            r#"
#[template]
mod my_template {
    pub enum State {
        Empty,
        Filled { data: Vec<u8> },
        Tuple(Vec<u8>),
    }
}
"#,
        );
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings[0].message.contains("`State::Filled.data"), "{findings:?}");
        assert!(findings[1].message.contains("`State::Tuple.0"), "{findings:?}");
    }

    #[test]
    fn private_items_in_a_template_module_are_not_encoded() {
        let findings = scan(
            r#"
#[template]
mod my_template {
    struct Helper {
        data: Vec<u8>,
    }

    pub struct MyTemplate {
        vault: Vault,
    }

    impl MyTemplate {
        fn internal(data: Vec<u8>) {}
    }
}
"#,
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn skip_cbor_derives_relies_on_the_authors_derives() {
        let findings = scan(
            r#"
#[template(skip_cbor_derives)]
mod my_template {
    pub struct MyTemplate {
        data: Vec<u8>,
    }
}
"#,
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn unparseable_source_is_ignored() {
        assert!(scan("pub struct { oops").is_empty());
    }
}
