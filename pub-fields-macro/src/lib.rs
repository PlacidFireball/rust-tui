use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Fields};

/// Attribute macro that rewrites a named-field struct so that every field is `pub`.
///
/// Fields that are already `pub` are left unchanged (idempotent).
/// Applying this macro to a tuple struct, unit struct, or enum is a compile error.
///
/// # Example
///
/// ```rust
/// use pub_fields_macro::pub_fields;
///
/// #[pub_fields]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// // Equivalent to:
/// // pub struct Config {
/// //     pub host: String,
/// //     pub port: u16,
/// // }
/// ```
#[proc_macro_attribute]
pub fn pub_fields(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);

    let fields = match &mut input.data {
        syn::Data::Struct(s) => match &mut s.fields {
            Fields::Named(f) => f,
            Fields::Unnamed(_) => {
                return syn::Error::new_spanned(
                    &input.ident,
                    "#[pub_fields] does not support tuple structs",
                )
                .to_compile_error()
                .into()
            }
            Fields::Unit => {
                return syn::Error::new_spanned(
                    &input.ident,
                    "#[pub_fields] does not support unit structs",
                )
                .to_compile_error()
                .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input.ident,
                "#[pub_fields] can only be applied to structs",
            )
            .to_compile_error()
            .into()
        }
    };

    for field in fields.named.iter_mut() {
        field.vis = parse_quote!(pub);
    }

    quote!(#input).into()
}
