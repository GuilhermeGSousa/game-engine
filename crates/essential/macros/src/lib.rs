extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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

#[proc_macro_derive(Blendable)]
pub fn lerp_derive(input: TokenStream) -> TokenStream {
    // 1. Parse the input Rust code (the struct) into a syntax tree
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // 2. Check that we are deriving on a Struct
    let fields_processing = match ast.data {
        Data::Struct(data_struct) => match data_struct.fields {
            // Case A: Named fields (e.g., struct Point { x: f32, y: f32 })
            Fields::Named(fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_type = &f.ty;
                    // Generate: field_name: self.field_name.lerp(other.field_name, t)
                    quote! {
                        #field_name: #field_type::interpolate(from.#field_name, other.#field_name, t)
                    }
                });
                quote! {
                    { #(#recurse),* }
                }
            }
            // Case B: Unnamed/Tuple fields (e.g., struct Point(f32, f32))
            Fields::Unnamed(fields) => {
                let recurse = fields.unnamed.iter().enumerate().map(|(i, field)| {
                    let index = syn::Index::from(i);
                    let field_type = &field.ty;
                    quote! {
                        #field_type::interpolate(from.#index, other.#index, t)
                    }
                });
                quote! {
                    ( #(#recurse),* )
                }
            }
            // Case C: Unit structs (e.g., struct Unit;)
            Fields::Unit => quote! { {} },
        },
        _ => panic!("Lerp derive macro only supports Structs"),
    };

    // 3. Generate the implementation
    let gen = quote! {
        impl Blendable for #name {
            fn interpolate(from: Self, other: Self, t: f32) -> Self {
                #name #fields_processing
            }
        }
    };

    // 4. Return the generated code
    gen.into()
}

#[proc_macro_derive(AsAny)]
pub fn any_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let gen = quote! {

        impl AsAny for #name {
            fn as_any(&self) -> &dyn Any
            {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any
            {
                self
            }
        }

    };
    gen.into()
}
