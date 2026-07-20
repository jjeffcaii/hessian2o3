use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Lit, LitStr, Token, parse_macro_input};

#[proc_macro_derive(HessianSerialize, attributes(hessian))]
pub fn derive_hessian_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse(&input).map(|m| expand_serialize(&m)) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(HessianDeserialize, attributes(hessian))]
pub fn derive_hessian_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match parse(&input).map(|m| expand_deserialize(&m)) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// The struct shape both derives operate on, parsed once.
struct Model {
    name: syn::Ident,
    class_name: String,
    java_names: Vec<String>,
    rust_idents: Vec<syn::Ident>,
    field_types: Vec<syn::Type>,
    /// Per-field: `true` when the field carries `#[hessian(date)]`, meaning an
    /// `i64` (Unix millis) is encoded as the Hessian date wire type.
    is_date: Vec<bool>,
}

fn parse(input: &DeriveInput) -> syn::Result<Model> {
    let name = input.ident.clone();

    let class_name = extract_class(input)?;

    let named_fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(Error::new_spanned(
                    &name,
                    "Hessian only supports named-field structs",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(&name, "Hessian only supports structs"));
        }
    };

    let mut java_names: Vec<String> = Vec::new();
    let mut rust_idents: Vec<syn::Ident> = Vec::new();
    let mut field_types: Vec<syn::Type> = Vec::new();
    let mut is_date: Vec<bool> = Vec::new();

    for field in named_fields {
        let ident = field.ident.as_ref().unwrap();
        let java = extract_rename(&field.attrs)?.unwrap_or_else(|| ident.to_string());
        java_names.push(java);
        rust_idents.push(ident.clone());
        field_types.push(field.ty.clone());
        is_date.push(has_flag(&field.attrs, "date")?);
    }

    Ok(Model {
        name,
        class_name,
        java_names,
        rust_idents,
        field_types,
        is_date,
    })
}

fn expand_serialize(model: &Model) -> proc_macro2::TokenStream {
    let Model {
        name,
        class_name,
        java_names,
        rust_idents,
        is_date,
        ..
    } = model;

    let field_serializers = rust_idents.iter().zip(is_date).map(|(ident, &is_date)| {
        if is_date {
            // `#[hessian(date)]`: encode the `i64` millis as a Hessian date.
            quote! {
                ::hessian2::codec::Encoder::put_date_millis(w, self.#ident)?;
            }
        } else {
            quote! {
                ::hessian2::HSerialize::hessian_serialize(&self.#ident, w)?;
            }
        }
    });

    quote! {
        impl ::hessian2::HSerialize for #name {
            fn hessian_serialize<W: ::std::io::Write>(
                &self,
                w: &mut ::hessian2::codec::Encoder<W>,
            ) -> ::hessian2::Result<()> {
                w.begin_object(
                    #class_name,
                    &[#(#java_names),*],
                )?;
                #(#field_serializers)*
                ::hessian2::Result::Ok(())
            }
        }
    }
}

fn expand_deserialize(model: &Model) -> proc_macro2::TokenStream {
    let Model {
        name,
        java_names,
        rust_idents,
        field_types,
        ..
    } = model;

    quote! {
        impl ::hessian2::HDeserialize for #name {
            fn hessian_deserialize<__R: ::std::io::Read>(
                de: &mut ::hessian2::de::Deserializer<__R>,
            ) -> ::hessian2::Result<Self> {
                let mut obj = de.begin_object()?;
                #(
                    let mut #rust_idents: ::std::option::Option<#field_types> =
                        ::std::option::Option::None;
                )*
                while let ::std::option::Option::Some(field) = obj.next_field() {
                    match ::std::convert::AsRef::<str>::as_ref(&field) {
                        #(
                            #java_names => {
                                #rust_idents = ::std::option::Option::Some(obj.value()?);
                            }
                        )*
                        _ => {
                            obj.skip_value()?;
                        }
                    }
                }
                ::hessian2::Result::Ok(Self {
                    #(
                        #rust_idents: #rust_idents.ok_or_else(|| {
                            ::hessian2::Error::IO(::std::io::Error::new(
                                ::std::io::ErrorKind::InvalidData,
                                ::std::concat!("missing field `", #java_names, "`"),
                            ))
                        })?,
                    )*
                })
            }
        }

        // Routes `deserialize!` for this type through the Hessian path (rule 1).
        impl ::hessian2::AutoDeserialize for #name {
            fn auto_deserialize<__R: ::std::io::Read>(
                mut reader: __R,
            ) -> ::hessian2::Result<Self> {
                ::hessian2::hessian::hessian_from_reader(&mut reader)
            }
        }
    }
}

fn extract_hessian_str_arg(attrs: &[syn::Attribute], key: &str) -> syn::Result<Option<String>> {
    let mut found = None;
    for attr in attrs {
        if !attr.path().is_ident("hessian") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                let s: LitStr = meta.value()?.parse()?;
                found = Some(s.value());
            } else if meta.input.peek(Token![=]) {
                // consume and ignore other `key = value` args in this list
                let _: Lit = meta.value()?.parse()?;
            }
            Ok(())
        })?;
    }
    Ok(found)
}

fn extract_class(input: &DeriveInput) -> syn::Result<String> {
    extract_hessian_str_arg(&input.attrs, "class")?.ok_or_else(|| {
        Error::new_spanned(&input.ident, "Hessian requires #[hessian(class = \"...\")]")
    })
}

fn extract_rename(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    extract_hessian_str_arg(attrs, "rename")
}

/// Detects a bare boolean flag inside `#[hessian(...)]`, e.g. `#[hessian(date)]`.
/// Ignores `key = "value"` entries so it composes with `rename` etc.
fn has_flag(attrs: &[syn::Attribute], key: &str) -> syn::Result<bool> {
    let mut found = false;
    for attr in attrs {
        if !attr.path().is_ident("hessian") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.input.peek(Token![=]) {
                // a `key = value` entry: consume and ignore its value
                let _: Lit = meta.value()?.parse()?;
            } else if meta.path.is_ident(key) {
                found = true;
            }
            Ok(())
        })?;
    }
    Ok(found)
}
