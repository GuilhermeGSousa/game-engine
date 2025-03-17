extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro_derive(Component)]
pub fn component(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_component(&ast)
}

fn impl_component(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl Component for #name {
            fn name() -> String {
                String::from(stringify!(#name))
            }
        }

    };
    gen.into()
}

#[proc_macro_derive(Resource)]
pub fn resource(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_resource(&ast)
}

fn impl_resource(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl Resource for #name {
            fn name() -> String {
                String::from(stringify!(#name))
            }
        }

    };
    gen.into()
}
