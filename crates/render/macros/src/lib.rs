extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput, Expr, Field, Ident, Lit, Meta, Type,
};

/// Represents the different types of shader resource bindings supported by the
/// `#[derive(AsBindGroup)]` macro.  Each variant corresponds to one
/// `@group(0)` entry in the generated `wgpu::BindGroupLayout`.
#[derive(Debug)]
enum BindingKind {
    /// A 2D texture (`Option<AssetHandle<Texture>>`). The sampler is placed at binding `index + 1`.
    Texture { index: u32 },
    /// A sampler linked to the texture at `texture_index`.
    Sampler { index: u32 },
    /// A plain `bytemuck::Pod + bytemuck::Zeroable` value uploaded as a uniform buffer.
    Uniform { index: u32 },
}

/// Associates a struct field with its shader binding metadata extracted from
/// the `#[texture(N)]`, `#[sampler(N)]`, or `#[uniform(N)]` attributes.
struct BindingField<'a> {
    field: &'a Field,
    kind: BindingKind,
}

/// Parse the integer literal inside an attribute like `#[texture(0)]`.
fn parse_index_from_attr(attr: &syn::Attribute) -> Option<u32> {
    match &attr.meta {
        Meta::List(list) => {
            let lit: Lit = syn::parse2(list.tokens.clone()).ok()?;
            if let Lit::Int(int) = lit {
                int.base10_parse::<u32>().ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract binding fields from a struct's named fields.
fn collect_binding_fields(fields: &syn::FieldsNamed) -> Vec<BindingField<'_>> {
    let mut bindings = Vec::new();
    for field in &fields.named {
        for attr in &field.attrs {
            if attr.path().is_ident("texture") {
                if let Some(index) = parse_index_from_attr(attr) {
                    bindings.push(BindingField {
                        field,
                        kind: BindingKind::Texture { index },
                    });
                }
            } else if attr.path().is_ident("sampler") {
                if let Some(index) = parse_index_from_attr(attr) {
                    bindings.push(BindingField {
                        field,
                        kind: BindingKind::Sampler { index },
                    });
                }
            } else if attr.path().is_ident("uniform") {
                if let Some(index) = parse_index_from_attr(attr) {
                    bindings.push(BindingField {
                        field,
                        kind: BindingKind::Uniform { index },
                    });
                }
            }
        }
    }
    bindings
}

/// Parse `vertex_shader` / `fragment_shader` from the struct-level `#[material(...)]` attribute.
///
/// Each value may be a string literal (`"..."`) or any Rust expression that
/// evaluates to `&'static str` at compile time, such as `include_str!(...)`.
fn parse_material_attr(attrs: &[syn::Attribute]) -> (Option<Expr>, Option<Expr>) {
    let mut vertex: Option<Expr> = None;
    let mut fragment: Option<Expr> = None;
    for attr in attrs {
        if !attr.path().is_ident("material") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("vertex_shader") {
                let value = meta.value()?;
                let expr: Expr = value.parse()?;
                vertex = Some(expr);
            } else if meta.path.is_ident("fragment_shader") {
                let value = meta.value()?;
                let expr: Expr = value.parse()?;
                fragment = Some(expr);
            }
            Ok(())
        });
    }
    (vertex, fragment)
}

/// Generate the `bind_group_layout` method body.
fn gen_bind_group_layout(bindings: &[BindingField<'_>], struct_name: &Ident) -> TokenStream2 {
    let struct_label = struct_name.to_string() + "_layout";
    let entries: Vec<TokenStream2> = bindings
        .iter()
        .map(|b| match &b.kind {
            BindingKind::Texture { index } => {
                quote! {
                    wgpu::BindGroupLayoutEntry {
                        binding: #index,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    }
                }
            }
            BindingKind::Sampler { index } => {
                quote! {
                    wgpu::BindGroupLayoutEntry {
                        binding: #index,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    }
                }
            }
            BindingKind::Uniform { index } => {
                quote! {
                    wgpu::BindGroupLayoutEntry {
                        binding: #index,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                }
            }
        })
        .collect();

    quote! {
        fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(#struct_label),
                entries: &[ #(#entries),* ],
            })
        }
    }
}

/// Check whether a type looks like `Option<AssetHandle<…>>`.
fn is_option_asset_handle(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(Type::Path(inner))) =
                        args.args.first()
                    {
                        if let Some(inner_seg) = inner.path.segments.last() {
                            return inner_seg.ident == "AssetHandle";
                        }
                    }
                }
            }
        }
    }
    false
}

/// Generate the `create_bind_group` method body.
fn gen_create_bind_group(bindings: &[BindingField<'_>], struct_name: &Ident) -> TokenStream2 {
    let bind_group_label = struct_name.to_string() + "_bind_group";
    let mut entry_stmts: Vec<TokenStream2> = Vec::new();

    for b in bindings {
        let field_name = b.field.ident.as_ref().unwrap();
        match &b.kind {
            BindingKind::Texture { index } => {
                // If the type is Option<AssetHandle<…>> we do a lookup in render_textures.
                if is_option_asset_handle(&b.field.ty) {
                    entry_stmts.push(quote! {
                        if let Some(tex_handle) = &self.#field_name {
                            if let Some(tex) = render_textures.get(&tex_handle.id()) {
                                entries.push(wgpu::BindGroupEntry {
                                    binding: #index,
                                    resource: wgpu::BindingResource::TextureView(&tex.view),
                                });
                            } else {
                                return Err(render::render_asset::AssetPreparationError::NotReady);
                            }
                        } else {
                            entries.push(wgpu::BindGroupEntry {
                                binding: #index,
                                resource: wgpu::BindingResource::TextureView(&dummy_texture.view),
                            });
                        }
                    });
                }
            }
            BindingKind::Sampler { index } => {
                // Find the texture field that was declared before this sampler. We look for the
                // most-recently-seen texture binding whose index is `index - 1`.
                // For robustness we emit code that re-queries the same Option field used for the
                // paired texture.  We track which texture field to pair with by searching backwards.
                let paired_texture: Option<(&Field, u32)> = bindings
                    .iter()
                    .filter_map(|b2| {
                        if let BindingKind::Texture { index: ti } = b2.kind {
                            Some((b2.field, ti))
                        } else {
                            None
                        }
                    })
                    .filter(|(_, ti)| *ti < *index)
                    .last();

                if let Some((tex_field, _)) = paired_texture {
                    let tex_field_name = tex_field.ident.as_ref().unwrap();
                    if is_option_asset_handle(&tex_field.ty) {
                        entry_stmts.push(quote! {
                            if let Some(tex_handle) = &self.#tex_field_name {
                                if let Some(tex) = render_textures.get(&tex_handle.id()) {
                                    entries.push(wgpu::BindGroupEntry {
                                        binding: #index,
                                        resource: wgpu::BindingResource::Sampler(&tex.sampler),
                                    });
                                } else {
                                    return Err(render::render_asset::AssetPreparationError::NotReady);
                                }
                            } else {
                                entries.push(wgpu::BindGroupEntry {
                                    binding: #index,
                                    resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
                                });
                            }
                        });
                    }
                } else {
                    // Standalone sampler — always use dummy.
                    entry_stmts.push(quote! {
                        entries.push(wgpu::BindGroupEntry {
                            binding: #index,
                            resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
                        });
                    });
                }
            }
            BindingKind::Uniform { index } => {
                let buf_label = format!("{}_{}_uniform", struct_name, field_name);
                entry_stmts.push(quote! {
                    let uniform_buf = device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some(#buf_label),
                            contents: bytemuck::cast_slice(&[self.#field_name]),
                            usage: wgpu::BufferUsages::UNIFORM,
                        },
                    );
                    entries.push(wgpu::BindGroupEntry {
                        binding: #index,
                        resource: wgpu::BindingResource::Buffer(
                            uniform_buf.as_entire_buffer_binding(),
                        ),
                    });
                });
            }
        }
    }

    quote! {
        fn create_bind_group(
            &self,
            device: &wgpu::Device,
            render_textures: &render::render_asset::RenderAssets<render::render_asset::render_texture::RenderTexture>,
            dummy_texture: &render::render_asset::render_texture::DummyRenderTexture,
            layout: &wgpu::BindGroupLayout,
        ) -> Result<wgpu::BindGroup, render::render_asset::AssetPreparationError> {
            use wgpu::util::DeviceExt;
            let mut entries: Vec<wgpu::BindGroupEntry<'_>> = Vec::new();
            #(#entry_stmts)*
            Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &entries,
                label: Some(#bind_group_label),
            }))
        }
    }
}

/// `#[derive(AsBindGroup)]` — automatically implement [`AsBindGroup`] for a struct.
///
/// # Field attributes
///
/// | Attribute       | Field type                          | Description                                  |
/// |-----------------|-------------------------------------|----------------------------------------------|
/// | `#[texture(N)]` | `Option<AssetHandle<Texture>>`      | Texture binding at slot *N* (FRAGMENT stage) |
/// | `#[sampler(N)]` | *(any)*                             | Sampler binding at slot *N* (FRAGMENT stage) |
/// | `#[uniform(N)]` | `T: bytemuck::Pod + bytemuck::Zeroable` | Uniform buffer at slot *N*               |
///
/// # Struct attributes
///
/// ```text
/// #[material(vertex_shader = "path/to/vs.wgsl", fragment_shader = "path/to/fs.wgsl")]
/// ```
///
/// When omitted, [`ShaderRef::Default`] is used for both shader stages.
#[proc_macro_derive(AsBindGroup, attributes(texture, sampler, uniform, material))]
pub fn derive_as_bind_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let (vertex_shader_lit, fragment_shader_lit) = parse_material_attr(&input.attrs);

    let vertex_shader_expr = match vertex_shader_lit {
        Some(expr) => quote! { render::assets::material::ShaderRef::Source(#expr) },
        None => quote! { render::assets::material::ShaderRef::Default },
    };
    let fragment_shader_expr = match fragment_shader_lit {
        Some(expr) => quote! { render::assets::material::ShaderRef::Source(#expr) },
        None => quote! { render::assets::material::ShaderRef::Default },
    };

    let named_fields = match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(f) => f,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "AsBindGroup only supports structs with named fields",
                )
                .to_compile_error()
                .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "AsBindGroup only supports structs")
                .to_compile_error()
                .into()
        }
    };

    let bindings = collect_binding_fields(named_fields);

    let layout_fn = gen_bind_group_layout(&bindings, name);
    let bind_group_fn = gen_create_bind_group(&bindings, name);

    let expanded = quote! {
        impl render::assets::material::AsBindGroup for #name {
            fn vertex_shader() -> render::assets::material::ShaderRef {
                #vertex_shader_expr
            }

            fn fragment_shader() -> render::assets::material::ShaderRef {
                #fragment_shader_expr
            }

            #layout_fn

            #bind_group_fn
        }
    };

    expanded.into()
}
