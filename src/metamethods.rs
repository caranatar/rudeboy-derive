use std::collections::HashSet;
use quote::{quote, quote_spanned};
use syn;
use syn::spanned::Spanned;
use proc_macro2::TokenStream as TokenStream2;

fn operator_method(name: TokenStream2, rlua_enum: TokenStream2, operator: TokenStream2) -> TokenStream2 {
    quote! {
        fn #name<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_meta_method(::rlua::MetaMethod::#rlua_enum, |ctx, data, other: Self| {
                use ::rlua::ToLua;
                let ret = (*data #operator other);
                Ok(ret.to_lua(ctx))
            });
        }
    }
}

fn unary_operator_method(name: TokenStream2, rlua_enum: TokenStream2, operator: TokenStream2) -> TokenStream2 {
    quote! {
        fn #name<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_meta_method(::rlua::MetaMethod::#rlua_enum, |ctx, data, ()| {
                use ::rlua::ToLua;
                let ret = #operator *data;
                Ok(ret.to_lua(ctx))
            });
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum MetaMethod {
    Add,
    Eq,
    Index,
    Sub,
    Mul,
    Div,
    Mod,
    Unm,
    BAnd,
    BOr,
    BXor,
    BNot,
    Shl,
    Shr,
    Lt,
    Le,
}

impl MetaMethod {
    const ADD_IDENT: &'static str = "Add";
    const EQUALS_IDENT: &'static str = "Eq";
    const INDEX_IDENT: &'static str = "Index";
    const SUB_IDENT: &'static str = "Sub";
    const MUL_IDENT: &'static str = "Mul";
    const DIV_IDENT: &'static str = "Div";
    const MOD_IDENT: &'static str = "Mod";
    const UNM_IDENT: &'static str = "Unm";
    const BAND_IDENT: &'static str = "BAnd";
    const BOR_IDENT: &'static str = "BOr";
    const BXOR_IDENT: &'static str = "BXor";
    const BNOT_IDENT: &'static str = "BNot";
    const SHL_IDENT: &'static str = "Shl";
    const SHR_IDENT: &'static str = "Shr";
    const LT_IDENT: &'static str = "Lt";
    const LE_IDENT: &'static str = "Le";

    fn try_parse(path: &syn::Path) -> Result<MetaMethod, TokenStream2> {
        if path.is_ident(Self::ADD_IDENT) {
            Ok(MetaMethod::Add)
        } else if path.is_ident(Self::EQUALS_IDENT) {
            Ok(MetaMethod::Eq)
        } else if path.is_ident(Self::INDEX_IDENT) {
            Ok(MetaMethod::Index)
        } else if path.is_ident(Self::SUB_IDENT) {
            Ok(MetaMethod::Sub)
        } else if path.is_ident(Self::MUL_IDENT) {
            Ok(MetaMethod::Mul)
        } else if path.is_ident(Self::DIV_IDENT) {
            Ok(MetaMethod::Div)
        } else if path.is_ident(Self::MOD_IDENT) {
            Ok(MetaMethod::Mod)
        } else if path.is_ident(Self::UNM_IDENT) {
            Ok(MetaMethod::Unm)
        } else if path.is_ident(Self::BAND_IDENT) {
            Ok(MetaMethod::BAnd)
        } else if path.is_ident(Self::BOR_IDENT) {
            Ok(MetaMethod::BOr)
        } else if path.is_ident(Self::BXOR_IDENT) {
            Ok(MetaMethod::BXor)
        } else if path.is_ident(Self::BNOT_IDENT) {
            Ok(MetaMethod::BNot)
        } else if path.is_ident(Self::SHL_IDENT) {
            Ok(MetaMethod::Shl)
        } else if path.is_ident(Self::SHR_IDENT) {
            Ok(MetaMethod::Shr)
        } else if path.is_ident(Self::LT_IDENT) {
            Ok(MetaMethod::Lt)
        } else if path.is_ident(Self::LE_IDENT) {
            Ok(MetaMethod::Le)
        } else {
            Err(quote_spanned! {
                path.span() => compile_error!("Expected a valid metamethod identifier");
            }
            .into())
        }
    }
    
    fn get_method(&self, ast: &syn::DeriveInput) -> TokenStream2 {
        match &self {
            MetaMethod::Add => operator_method(quote!(generate_add), quote!(Add), quote!(+)),
            MetaMethod::Eq =>
                operator_method(quote!(generate_eq), quote!(Eq), quote!(==)),
            MetaMethod::Index => {
                let struct_ =
                    match &ast.data {
                        syn::Data::Struct(s) => s,
                        _ => return quote_spanned! {
                            ast.span() => compile_error!("Index metamethod can only be applied to structs");
                        }
                        .into(),
                    };

                let fields = &struct_.fields;

                let mut bad_struct = true;
                if let syn::Fields::Named(_) = fields {
                    bad_struct = false;
                }

                if fields.is_empty() {
                    bad_struct = true;
                }

                if bad_struct {
                    return quote_spanned! {
                        fields.span() => compile_error!("Index metamethod can only be applied to structs with named fields");
                    }
                    .into();
                }

                let field_names: Vec<_> =
                    fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                quote! {
                    fn generate_index<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                        methods.add_meta_method(::rlua::MetaMethod::Index, |ctx, data, index: ::rlua::String| {
                            use ::rlua::ToLua;
                            let index_str = index.to_str()?;
                            #(
                                if index_str == stringify!(#field_names) {
                                    Ok(data.#field_names.clone().to_lua(ctx))
                                } else
                            )*
                            {
                                use ::rlua::ExternalError;
                                Err(format!("No such index: {}", index_str).to_lua_err())
                            }
                        });
                    }
                }
            },
            MetaMethod::Sub => operator_method(quote!(generate_sub), quote!(Sub), quote!(-)),
            MetaMethod::Mul => operator_method(quote!(generate_mul), quote!(Mul), quote!(*)),
            MetaMethod::Div => operator_method(quote!(generate_div), quote!(Div), quote!(/)),
            MetaMethod::Mod => operator_method(quote!(generate_mod), quote!(Mod), quote!(%)),
            MetaMethod::Unm => unary_operator_method(quote!(generate_unm), quote!(Unm), quote!(-)),
            MetaMethod::BAnd => operator_method(quote!(generate_band), quote!(BAnd), quote!(&)),
            MetaMethod::BOr => operator_method(quote!(generate_bor), quote!(BOr), quote!(|)),
            MetaMethod::BXor => operator_method(quote!(generate_bxor), quote!(BXor), quote!(^)),
            MetaMethod::BNot => unary_operator_method(quote!(generate_bnot), quote!(BNot), quote!(!)),
            MetaMethod::Shl => operator_method(quote!(generate_shl), quote!(Shl), quote!(<<)),
            MetaMethod::Shr => operator_method(quote!(generate_shr), quote!(Shr), quote!(>>)),
            MetaMethod::Lt => operator_method(quote!(generate_lt), quote!(Lt), quote!(<)),
            MetaMethod::Le => operator_method(quote!(generate_le), quote!(Le), quote!(<=)),
        }
    }
}

fn attrs_to_metamethods(
    attrs: Vec<&syn::NestedMeta>,
) -> Result<HashSet<MetaMethod>, TokenStream2> {
    let mut metamethods = HashSet::new();
    for attr in attrs {
        use syn::{Meta, NestedMeta};
        metamethods.insert(match attr {
            NestedMeta::Meta(Meta::Path(p)) => MetaMethod::try_parse(p)?,
            _ => {
                return Err(quote_spanned! {
                    attr.span() => compile_error!("Expected a valid metamethod identifier");
                }
                .into())
            }
        });
    }
    Ok(metamethods)
}

pub(crate) fn impl_metamethods_attr_macro(
    item: syn::Item,
    attrs: Vec<&syn::NestedMeta>,
) -> TokenStream2 {
    let di = match &item {
        syn::Item::Struct(s) => syn::DeriveInput::from(s.clone()),
        syn::Item::Enum(e) => syn::DeriveInput::from(e.clone()),
        _ => {
            return quote_spanned! {
                item.span() => compile_error!("metamethods can only be applied to structs and enums");
            }
            .into()
        }
    };
    let name = &di.ident;
    let metamethods: Vec<_> = match attrs_to_metamethods(attrs) {
        Ok(mms) => mms,
        Err(e) => return e,
    }
    .iter()
    .map(|mm| mm.get_method(&di))
    .collect();

    quote! {
        #item

        impl ::rudeboy::RudeboyMetaMethods for #name {
            #( #metamethods )*
        }
    }
}
