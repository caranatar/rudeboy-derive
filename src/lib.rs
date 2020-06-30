//! This crate provides derive and attr macros for use by the [`rudeboy`] crate.
//! Please refer to it for documentation and usage information.
//!
//! [`rudeboy`]: https://docs.rs/rudeboy
use proc_macro::TokenStream;
use syn;

mod methods;
use methods::impl_methods_attr_macro;

/// Placed on an inherent impl block; generates an impl of [`RudeboyMethods`] to
/// add the contained methods to the exported user data. Takes no parameters.
///
/// [`RudeboyMethods`]: trait.RudeboyMethods.html
#[proc_macro_attribute]
pub fn methods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Item);
    impl_methods_attr_macro(input).into()
}

mod metamethods;
use metamethods::impl_metamethods_attr_macro;

/// Placed on a struct or enum definition; generates an impl of
/// [`RudeboyMetamethods`] to add the specified metamethods to the exported user
/// data.
///
/// Takes any combination of the following parameters:
/// * Add - allows the use of the `+` operator. Uses `std::ops::Add`
/// * BAnd - allows the use of the `&` operator. Uses `std::ops::BitAnd`
/// * BNot - allows the use of the unary `~` operator. Uses `std::ops::Not`
/// * BOr - allows the use of the `|` operator. Uses `std::ops::BitOr`
/// * BXor - allows the use of the binary `~` operator. Uses `std::ops::BitXor`
/// * Div - allows the use of the `/` operator. Uses `std::ops::Div`
/// * Eq - allows the use of the `==` operator. Uses `std::cmp::PartialEq`
/// * Index - allows the use of `.` to retrieve fields. Only usable for structs
/// with named fields
/// * Le - allows the use of the `<=` operator. Uses `std::cmp::PartialOrd`
/// * Lt - allows the use of the `<` operator. Uses `std::cmp::PartialOrd`
/// * Mod - allows the use of the `%` operator. Uses `std::ops::Rem`
/// * Mul - allows the use of the `*` operator. Uses `std::ops::Mul`
/// * Shl - allows the use of the `<<` operator. Uses `std::ops::Shl`
/// * Shr - allows the use of the `>>` operator. Uses `std::ops::Shr`
/// * Sub - allows the use of the binary `-` operator. Uses `std::ops::Sub`
/// * Unm - allows the use of the unary `-` operator. Uses `std::ops::Neg`
///
/// Note: all binary operators currently take a parameter of the same type as the
/// type the metamethod is being added to. This is not obviously not ideal.
///
/// [`RudeboyMetaMethods`]: trait.RudeboyMetaMethods.html
#[proc_macro_attribute]
pub fn metamethods(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::Item);
    use syn::parse::Parser;
    let parser = syn::punctuated::Punctuated::<syn::NestedMeta, syn::Token!(,)>::parse_terminated;
    let parsed_attrs = parser.parse(attr);
    let attrs = match &parsed_attrs {
        Ok(ok) => ok.iter().collect(),
        Err(e) => return e.to_compile_error().into(),
    };
    impl_metamethods_attr_macro(input, attrs).into()
}

mod user_data;
use user_data::impl_user_data_attr_macro;

/// Generates an implementation of `rlua::UserData` for the tagged type
/// definition or the type that matches a tagged impl block.
///
/// Takes zero or more of the following parameters. If given none, then the
/// exported type will have no methods or metamethods available.
/// * MetaMethods - will use the [`RudeboyMetaMethods`] trait to add generated
/// meta methods
/// * Methods - will use the [`RudeboyMethods`] trait to add generated methods
///
/// Note: if you wish to add additional (meta)methods beyond the ones generated
/// by rudeboy, do not use this macro and instead manually call the appropriate
/// trait methods in your implementation of `rlua::UserData`
///
/// [`RudeboyMetaMethods`]: trait.RudeboyMetaMethods.html
/// [`RudeboyMethods`]: trait.RudeboyMethods.html
#[proc_macro_attribute]
pub fn user_data(attr: TokenStream, item: TokenStream) -> TokenStream {
    use syn::parse::Parser;
    let parser = syn::punctuated::Punctuated::<syn::NestedMeta, syn::Token!(,)>::parse_terminated;
    let parsed_attrs = parser.parse(attr);
    let attrs = match &parsed_attrs {
        Ok(ok) => ok.iter().collect(),
        Err(e) => return e.to_compile_error().into(),
    };
    let input = syn::parse_macro_input!(item as syn::Item);
    impl_user_data_attr_macro(input, attrs).into()
}
