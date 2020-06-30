use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn;
use syn::spanned::Spanned;

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
) -> Result<(&syn::Ident, Box<syn::Type>), TokenStream2> {
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

fn implitem_methods_attr_macro(ast: &syn::ItemImpl) -> TokenStream2 {
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
                    ( #( #names, )* ) : ( #( #tys, )* )
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
    quote! {
        #ast

        impl ::rudeboy::RudeboyMethods for #self_ty {
            fn generate_methods<'lua, M: ::rlua::UserDataMethods<'lua, Self>>(_methods: &mut M) {
                #( #mqs )*
            }
        }
    }
}

pub(crate) fn impl_methods_attr_macro(item: syn::Item) -> TokenStream2 {
    if let syn::Item::Impl(i) = item {
        implitem_methods_attr_macro(&i)
    } else {
        return quote_spanned! {
            item.span() => compile_error!("Methods macro can only be applied to an inherent impl block");
        };
    }
}
