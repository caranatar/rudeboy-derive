use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use std::collections::HashSet;
use syn;
use syn::spanned::Spanned;

#[derive(Eq, PartialEq, Hash)]
enum UserDataAttr {
    MetaMethods,
    Methods,
}

impl UserDataAttr {
    const META_METHODS_IDENT: &'static str = "MetaMethods";
    const METHODS_IDENT: &'static str = "Methods";

    fn try_parse(path: &syn::Path) -> Result<UserDataAttr, TokenStream2> {
        if path.is_ident(Self::META_METHODS_IDENT) {
            Ok(UserDataAttr::MetaMethods)
        } else if path.is_ident(Self::METHODS_IDENT) {
            Ok(UserDataAttr::Methods)
        } else {
            Err(quote_spanned! {
                path.span() => compile_error!("Expected a valid metamethod identifier");
            }
            .into())
        }
    }

    fn get_code(&self, name: TokenStream2) -> TokenStream2 {
        match self {
            UserDataAttr::MetaMethods => quote! {
                use ::rudeboy::RudeboyMetaMethods;
                #name::generate_metamethods(methods);
            },
            UserDataAttr::Methods => quote! {
                use ::rudeboy::RudeboyMethods;
                #name::generate_methods(methods);
            },
        }
    }
}

fn attrs_to_user_data_attrs(
    attrs: Vec<&syn::NestedMeta>,
) -> Result<HashSet<UserDataAttr>, TokenStream2> {
    let mut ret = HashSet::new();
    for attr in attrs {
        use syn::{Meta, NestedMeta};
        ret.insert(match attr {
            NestedMeta::Meta(Meta::Path(p)) => UserDataAttr::try_parse(p)?,
            _ => {
                return Err(quote_spanned! {
                    attr.span() => compile_error!("Expected a valid user_data identifier");
                }
                .into())
            }
        });
    }
    Ok(ret)
}

pub(crate) fn impl_user_data_attr_macro(
    item: syn::Item,
    attrs: Vec<&syn::NestedMeta>,
) -> TokenStream2 {
    let name = if let syn::Item::Impl(i) = &item {
        let self_ty = &i.self_ty;
        quote!(#self_ty)
    } else if let syn::Item::Struct(s) = &item {
        let name = &s.ident;
        quote!(#name)
    } else if let syn::Item::Enum(e) = &item {
        let name = &e.ident;
        quote!(#name)
    } else {
        return quote_spanned! {
            item.span() => compile_error!("user_data macro can only be applied to a struct or an inherent impl block");
        };
    };

    let inner_code: Vec<_> = match attrs_to_user_data_attrs(attrs) {
        Ok(uda) => uda,
        Err(e) => return e,
    }
    .iter()
    .map(|a| a.get_code(name.clone()))
    .collect();

    quote! {
        #item

        impl ::rlua::UserData for #name {
            fn add_methods<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                #( #inner_code )*
            }
        }
    }
}
