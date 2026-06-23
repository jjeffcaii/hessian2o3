use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Fields, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
    parse_macro_input,
};

#[proc_macro_derive(HessianObject, attributes(hessian))]
pub fn derive_hessian_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let class_name = extract_class(&input)?;

    let named_fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(Error::new_spanned(
                    name,
                    "HessianObject only supports named-field structs",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                name,
                "HessianObject only supports structs",
            ));
        }
    };

    let mut java_names: Vec<String> = Vec::new();
    let mut rust_idents: Vec<&syn::Ident> = Vec::new();

    for field in named_fields {
        let ident = field.ident.as_ref().unwrap();
        let java = extract_rename(&field.attrs)?.unwrap_or_else(|| ident.to_string());
        java_names.push(java);
        rust_idents.push(ident);
    }

    let field_serializers = rust_idents.iter().map(|ident| {
        quote! {
            ::hessian2o3::HessianSerialize::hessian_serialize(&self.#ident, w, ctx)?;
        }
    });

    Ok(quote! {
        impl ::hessian2o3::HessianSerialize for #name {
            fn hessian_serialize<W: ::std::io::Write>(
                &self,
                w: &mut W,
                ctx: &mut ::hessian2o3::codec::Context,
            ) -> ::std::io::Result<()> {
                ::hessian2o3::codec::begin_object(
                    w,
                    ctx,
                    #class_name,
                    &[#(#java_names),*],
                )?;
                #(#field_serializers)*
                ::std::result::Result::Ok(())
            }
        }
    })
}

fn extract_class(input: &DeriveInput) -> syn::Result<String> {
    for attr in &input.attrs {
        if attr.path.is_ident("hessian") {
            if let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() {
                for item in &nested {
                    if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) = item
                    {
                        if path.is_ident("class") {
                            return Ok(s.value());
                        }
                    }
                }
            }
        }
    }
    Err(Error::new_spanned(
        &input.ident,
        "HessianObject requires #[hessian(class = \"...\")]",
    ))
}

fn extract_rename(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path.is_ident("hessian") {
            if let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() {
                for item in &nested {
                    if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) = item
                    {
                        if path.is_ident("rename") {
                            return Ok(Some(s.value()));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}
