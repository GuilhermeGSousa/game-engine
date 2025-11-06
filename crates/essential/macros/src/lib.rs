extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro_derive(Asset)]
pub fn asset(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_asset(&ast)
}

fn impl_asset(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl Asset for #name {
            fn name() -> &'static str {
                stringify!(#name)
            }
        }

    };
    gen.into()
}
