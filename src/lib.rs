//! This crate provides derive and attr macros for use by the [`rudeboy`] crate.
//! Please refer to it for documentation and usage information.
//!
//! [`rudeboy`]: https://docs.rs/rudeboy
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn;
use syn::spanned::Spanned;

fn impl_index_macro(ast: &syn::DeriveInput, gen_userdata: bool) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let struct_ =
        match &ast.data {
            syn::Data::Struct(s) => s,
            _ => return quote_spanned! {
                ast.span() => compile_error!("RudeboyIndex macros can only be applied to structs");
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
            fields.span() => compile_error!("RudeboyIndex macros can only be applied to structs with named fields");
        }
        .into();
    }

    let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();

    let index = quote! {
        impl ::rudeboy::RudeboyIndex for #name {
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
    };

    if gen_userdata {
        quote! {
            #index

            impl ::rlua::UserData for #name {
                fn add_methods<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                    use ::rudeboy::RudeboyIndex;
                    #name::generate_index(methods);
                }
            }
        }
    } else {
        index
    }
}

fn impl_no_index_macro(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    quote! {
        impl ::rudeboy::RudeboyIndex for #name {
            fn generate_index<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(_: &mut M) {
            }
        }
    }
}

/// Generates an index metamethod for a UserData struct type
///
/// The derive macro implements the [`RudeboyIndex`] trait. To add the metamethod
/// to a userdata type, call `Self::generate_index()` in your implementation of
/// `add_methods` for `rlua::UserData`
///
/// [`RudeboyIndex`]: trait.RudeboyIndex.html
#[proc_macro_derive(Index)]
pub fn rlua_index_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_index_macro(&ast, false).into()
}

/// Generates an index metamethod for a UserData struct type and an impl of
/// `rlua::UserData` that calls it
///
/// Use this derive macro when you want a type to have an index but no other
/// methods available in lua
#[proc_macro_derive(IndexSealed)]
pub fn rlua_index_sealed_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Return generated code
    impl_index_macro(&ast, true).into()
}

/// Generates an implementation of [`RudeboyIndex`] that does nothing
///
/// Use this derive macro when you want to use [`Methods`] or [`MethodsSealed`]
/// but do not want your type to have an index metamethod
///
/// [`RudeboyIndex`]: trait.RudeboyIndex.html
/// [`Methods`]: attr.Methods.html
/// [`MethodsSealed`]: attr.MethodsSealed.html
#[proc_macro_derive(NoIndex)]
pub fn rlua_no_index_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_no_index_macro(&ast).into()
}

enum Params<'a> {
    None,
    One {
        name: &'a syn::Ident,
        ty: Box<syn::Type>,
    },
    Multi {
        names: Vec<&'a syn::Ident>,
        tys: Vec<Box<syn::Type>>,
    },
}

struct MethodInfo<'a> {
    pub name: &'a syn::Ident,
    pub is_mut: bool,
    pub params: Params<'a>,
}

fn get_name_and_type_from_fn_arg(
    fn_arg: &syn::FnArg,
) -> Result<(&syn::Ident, Box<syn::Type>), proc_macro2::TokenStream> {
    if let syn::FnArg::Typed(t) = fn_arg {
        let pat: &syn::Pat = t.pat.as_ref();
        let ty = t.ty.clone();
        if let syn::Pat::Ident(i) = pat {
            Ok((&i.ident, ty))
        } else {
            Err(quote_spanned! {
                pat.span() => compiler_error!("Expected an identifier here. This is probably a bug.");
            })
        }
    } else {
        Err(quote_spanned! {
            fn_arg.span() => compile_error!("Expected a typed argument of the form 'ident: Type'. This is a bug.");
        })
    }
}

fn implitem_methods_attr_macro(ast: &syn::ItemImpl, gen_userdata: bool) -> proc_macro2::TokenStream {
    let mut methods = Vec::new();

    for item in &ast.items {
        if let syn::ImplItem::Method(m) = item {
            let signature = &m.sig;
            let name = &signature.ident;
            use syn::FnArg::*;
            let receiver = match signature.receiver() {
                Some(Receiver(rcv)) => rcv,
                Some(Typed(_)) => {
                    return quote_spanned! {
                        signature.span() => compile_error!("Cannot currently handle typed receivers (i.e., a receiver other than &self or &mut self)");
                    }
                }
                None => {
                    return quote_spanned! {
                        signature.span() => compile_error!("Cannot currently handle class level methods");
                    }
                }
            };
            if receiver.reference.is_none() {
                return quote_spanned! {
                    signature.span() => compile_error!("Cannot add a method that moves self");
                };
            }
            let is_mut = receiver.mutability.is_some();

            let inputs_len = signature.inputs.len();
            let params = if inputs_len == 0 {
                return quote_spanned! {
                    signature.span() => compile_error!("Unexpected method with zero parameters");
                };
            } else if inputs_len == 1 {
                Params::None
            } else if inputs_len == 2 {
                // Discard receiver
                let mut input_iter = signature.inputs.iter();
                let _ = input_iter.next().unwrap();
                let input = input_iter.next().unwrap();
                let (name, ty) = match get_name_and_type_from_fn_arg(&input) {
                    Ok((name, ty)) => (name, ty),
                    Err(ts) => return ts,
                };
                Params::One { name, ty }
            } else {
                // Discard receiver
                let mut input_iter = signature.inputs.iter();
                let _ = input_iter.next().unwrap();

                let mut names = Vec::new();
                let mut tys = Vec::new();
                while let Some(input) = input_iter.next() {
                    let (name, ty) = match get_name_and_type_from_fn_arg(&input) {
                        Ok((name, ty)) => (name, ty),
                        Err(ts) => return ts,
                    };
                    names.push(name);
                    tys.push(ty);
                }
                Params::Multi { names, tys }
            };

            methods.push(MethodInfo {
                name,
                is_mut,
                params,
            });
        }
    }

    let mqs: Vec<_> = methods
        .drain(..)
        .map(|m| {
            let call = if m.is_mut {
                quote! {
                    _methods.add_method_mut
                }
            } else {
                quote! {
                    _methods.add_method
                }
            };

            let params_param = match &m.params {
                Params::None => quote!(()),
                Params::One { name, ty } => quote!(#name : #ty),
                Params::Multi { names, tys } => quote! {
                    (
                      #(
                          #names,
                      )*
                    )
                    :
                    (
                      #(
                          #tys,
                      )*
                    )
                },
            };

            let method_params = match &m.params {
                Params::None => quote!(()),
                Params::One { name, .. } => quote!((#name)),
                Params::Multi { names, .. } => quote!((#(#names,)*)),
            };

            let name = m.name;

            quote! {
                #call (stringify!(#name), |_, data, #params_param| {
                    Ok(data.#name #method_params)
                });
            }
        })
        .collect();

    let self_ty = &ast.self_ty;
    let methods = quote! {
        #ast

        impl ::rudeboy::RudeboyMethods for #self_ty {
            fn generate_methods<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(_methods: &mut M) {
                #(
                    #mqs
                )*
            }
        }
    };

    if gen_userdata {
        quote! {
            #methods

            impl ::rlua::UserData for #self_ty {
                fn add_methods<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                    use ::rudeboy::RudeboyIndex;
                    use ::rudeboy::RudeboyMethods;
                    #self_ty::generate_index(methods);
                    #self_ty::generate_methods(methods);
                }
            }
        }
    } else {
        methods
    }
}

fn impl_methods_attr_macro(item: syn::Item, gen_userdata: bool) -> proc_macro2::TokenStream {
    if let syn::Item::Impl(i) = item {
        implitem_methods_attr_macro(&i, gen_userdata)
    } else {
        return quote_spanned! {
            item.span() => compile_error!("Methods macro can only be applied to an inherent impl block");
        };
    }
}

/// Exposes the methods in an impl block for a UserData struct type to lua
///
/// This attribute implements the [`RudeboyMethods`] trait. To add the methods it
/// captured to a userdata type, call `Self::generate_methods()` in your
/// implementation of `add_methods` for `rlua::UserData`
///
/// Note that the struct type itself must implement [`RudeboyIndex`]. Either
/// manually (not recommended) or using the [`Index`] or [`NoIndex`] derive
/// macros. Cannot be combined with [`IndexSealed`]
///
/// [`RudeboyMethods`]: trait.RudeboyMethods.html
/// [`RudeboyIndex`]: trait.RudeboyIndex.html
/// [`Index`]: derive.Index.html
/// [`NoIndex`]: derive.NoIndex.html
/// [`IndexSealed`]: derive.IndexSealed.html
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Methods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Item);
    impl_methods_attr_macro(input, false).into()
}

/// Exposes the methods in an impl block for a UserData struct type to lua and
/// generates an impl of `rlua::UserData` for the type
///
/// Note that the struct type itself must implement [`RudeboyIndex`]. Either
/// manually (not recommended) or using the [`Index`] or [`NoIndex`] derive
/// macros. Cannot be combined with [`IndexSealed`]
///
/// [`RudeboyMethods`]: trait.RudeboyMethods.html
/// [`RudeboyIndex`]: trait.RudeboyIndex.html
/// [`Index`]: derive.Index.html
/// [`NoIndex`]: derive.NoIndex.html
/// [`IndexSealed`]: derive.IndexSealed.html
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn MethodsSealed(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Item);
    impl_methods_attr_macro(input, true).into()
}
