//! Offline importer that relocates the parsing half of the old runtime
//! `GLTFLoader` into the asset-cook pipeline. A single `.gltf`/`.glb` is
//! split into independently-cooked `mesh/*`, `material/*`, `texture/*` and
//! `scene` sub-assets, cross-referenced by stable `AssetId`.
//!
// TODO(follow-up): skeleton, animation, camera, light, and Blender-extras component
// data are not yet ported from the original runtime GLTFLoader (removed in the
// Task 14b cutover — see git history for the reference implementation).

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, bail};
use asset_cook::{ImportContext, ImportError, Importer, hash_file_contents};
use color::Color;
use ecs::component::Component;
use essential::assets::{AssetId, handle::AssetHandle};
use essential::transform::Transform;
use glam::Vec3;
use gltf::{Primitive, buffer::Data};
use image::ImageBuffer;
use mesh::mesh::{Mesh, MeshComponent};
use mesh::vertex::Vertex;
use render::assets::cooked_texture::CookedTexture;
use render::assets::material::StandardMaterial;
use render::assets::texture::Texture;
use render::components::material::MaterialComponent;
use render::components::render_entity::SyncWithRenderWorld;
use scene::scene::{Scene, SceneNode};
use serde::Serialize;

pub struct GltfImporter;

/// One emitted primitive of a glTF mesh: the sub-asset indices of its cooked
/// `mesh/*` geometry and the `material/*` it draws with.
struct PrimRef {
    mesh_sub_asset: usize,
    material_sub_asset: usize,
}

/// Dedup key for the textures a source file needs: one cooked `texture/*`
/// sub-asset per (source image, colour space) pair. `srgb` orders
/// `false < true`, keeping cooked-output iteration order stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TextureKey {
    image_index: usize,
    srgb: bool,
}

impl Importer for GltfImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["gltf", "glb"]
    }

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        let (document, buffers, images) =
            gltf::import(source_path).map_err(|e| ImportError::MalformedSource {
                source_path: source_path.to_path_buf(),
                message: e.to_string(),
            })?;

        // Track every external `.bin`/image `uri` the document pulls in, so
        // editing one and re-running `cook` re-imports instead of shipping
        // stale cooked output. Embedded `data:` URIs need no tracking — a
        // change there changes the `.gltf`'s own hash.
        let source_dir = source_path.parent().unwrap_or_else(|| Path::new(""));
        for buffer in document.buffers() {
            if let gltf::buffer::Source::Uri(uri) = buffer.source() {
                track_external(ctx, source_dir, uri);
            }
        }
        for image in document.images() {
            if let gltf::image::Source::Uri { uri, .. } = image.source() {
                track_external(ctx, source_dir, uri);
            }
        }

        // Decode every embedded/external image up front; textures are emitted
        // later, once the materials loop has recorded which (image, color
        // space) pairs are actually referenced.
        let mut decoded_images: Vec<image::DynamicImage> = Vec::new();
        for (image_index, data) in images.into_iter().enumerate() {
            let dyn_img =
                dynamic_image_from_gltf(data).map_err(|e| ImportError::MalformedSource {
                    source_path: source_path.to_path_buf(),
                    message: format!("failed to load GLTF image at index {image_index}: {e}"),
                })?;
            decoded_images.push(dyn_img);
        }

        // Color textures (base color, emissive) are sRGB-encoded while data
        // textures (normal, metallic-roughness, occlusion) are linear. An
        // image can be referenced in both roles, so texture sub-assets are
        // keyed per (image, color space) pair.
        let mut needed_textures: BTreeSet<TextureKey> = BTreeSet::new();
        let mut materials: Vec<StandardMaterial> = Vec::new();
        {
            let mut texture_ref =
                |texture: gltf::Texture<'_>, srgb: bool| -> AssetHandle<Texture> {
                    let image_index = texture.source().index();
                    needed_textures.insert(TextureKey { image_index, srgb });
                    AssetHandle::weak(ctx.sub_asset_id(&texture_name(image_index, srgb)))
                };

            for gltf_material in document.materials() {
                let pbr = gltf_material.pbr_metallic_roughness();
                let mut material = StandardMaterial::new(
                    pbr.base_color_texture()
                        .map(|info| texture_ref(info.texture(), true)),
                    gltf_material
                        .normal_texture()
                        .map(|info| texture_ref(info.texture(), false)),
                );

                material.set_base_color_factor(Color::from(pbr.base_color_factor()));
                material.set_metallic_factor(pbr.metallic_factor());
                material.set_roughness_factor(pbr.roughness_factor());
                material.set_emissive_factor(Vec3::from_array(gltf_material.emissive_factor()));

                if let Some(info) = pbr.metallic_roughness_texture() {
                    material.set_metallic_roughness_texture(texture_ref(info.texture(), false));
                }
                if let Some(info) = gltf_material.emissive_texture() {
                    material.set_emissive_texture(texture_ref(info.texture(), true));
                }
                if let Some(info) = gltf_material.occlusion_texture() {
                    material.set_occlusion_strength(info.strength());
                    material.set_occlusion_texture(texture_ref(info.texture(), false));
                }

                if gltf_material.alpha_mode() == gltf::material::AlphaMode::Mask {
                    // GLTF spec default for alphaCutoff is 0.5 when alpha_mode is MASK.
                    material.set_alpha_cutoff(gltf_material.alpha_cutoff().unwrap_or(0.5));
                }

                materials.push(material);
            }
        }

        for key in &needed_textures {
            let rgba = decoded_images[key.image_index].to_rgba8();
            let cooked = CookedTexture {
                width: rgba.width(),
                height: rgba.height(),
                srgb: key.srgb,
                pixels: rgba.into_raw(),
            };
            ctx.emit(&texture_name(key.image_index, key.srgb), &cooked)?;
        }

        for (index, material) in materials.iter().enumerate() {
            ctx.emit(&format!("material/{index}"), material)?;
        }

        // Primitives with no material index share one default `StandardMaterial`
        // emitted after the real ones (index == document.materials().count()).
        let default_material_index = materials.len();
        let mut default_material_used = false;

        let mut mesh_counter: usize = 0;
        let mut mesh_prims: Vec<Vec<PrimRef>> = Vec::new();
        for mesh in document.meshes() {
            let mut prims = Vec::new();
            for gltf_primitive in mesh.primitives() {
                let m = load_primitive(source_path, mesh.name(), &buffers, &gltf_primitive)?;
                ctx.emit(&format!("mesh/{mesh_counter}"), &m)?;

                let material_sub_asset = match gltf_primitive.material().index() {
                    Some(material_index) => material_index,
                    None => {
                        default_material_used = true;
                        default_material_index
                    }
                };
                prims.push(PrimRef {
                    mesh_sub_asset: mesh_counter,
                    material_sub_asset,
                });
                mesh_counter += 1;
            }
            mesh_prims.push(prims);
        }

        if default_material_used {
            ctx.emit(
                &format!("material/{default_material_index}"),
                &StandardMaterial::default(),
            )?;
        }

        // Node walk. `document.nodes()` yields nodes in index order, so the
        // first `document.nodes().count()` entries of `nodes` line up 1:1 with
        // glTF node indices and every `children` index into that prefix stays
        // valid. Extra primitive child-nodes (for multi-primitive meshes) are
        // appended afterwards. Every runtime concern is emitted as a
        // `SerializedComponent`; referenced ids are recorded as we go.
        let mut nodes: Vec<SceneNode> = Vec::new();
        let mut referenced_assets: Vec<AssetId> = Vec::new();
        for gltf_node in document.nodes() {
            let mut scene_node = SceneNode {
                name: gltf_node.name().map(str::to_string).unwrap_or_default(),
                children: gltf_node.children().map(|c| c.index()).collect(),
                components: Vec::new(),
            };
            push_node_component(
                &mut scene_node,
                &Transform::from_matrix(&gltf_node.transform().matrix()),
            )?;
            nodes.push(scene_node);
        }

        for gltf_node in document.nodes() {
            let Some(gltf_mesh) = gltf_node.mesh() else {
                continue;
            };
            let node_index = gltf_node.index();
            let prims = &mesh_prims[gltf_mesh.index()];

            match prims.len() {
                0 => {}
                1 => {
                    let p = &prims[0];
                    let mesh_id = ctx.sub_asset_id(&format!("mesh/{}", p.mesh_sub_asset));
                    let material_id =
                        ctx.sub_asset_id(&format!("material/{}", p.material_sub_asset));

                    push_mesh_components(&mut nodes[node_index], mesh_id, material_id)?;
                    referenced_assets.push(mesh_id);
                    referenced_assets.push(material_id);
                }
                _ => {
                    let node_name = nodes[node_index].name.clone();
                    for (k, p) in prims.iter().enumerate() {
                        let child_index = nodes.len();
                        let mesh_id = ctx.sub_asset_id(&format!("mesh/{}", p.mesh_sub_asset));
                        let material_id =
                            ctx.sub_asset_id(&format!("material/{}", p.material_sub_asset));

                        let mut child = SceneNode {
                            name: format!("{node_name}.primitive{k}"),
                            children: Vec::new(),
                            components: Vec::new(),
                        };
                        push_node_component(&mut child, &Transform::IDENTITY)?;
                        push_mesh_components(&mut child, mesh_id, material_id)?;
                        referenced_assets.push(mesh_id);
                        referenced_assets.push(material_id);

                        nodes.push(child);
                        nodes[node_index].children.push(child_index);
                    }
                }
            }
        }

        ctx.emit(
            "scene",
            &Scene {
                nodes,
                referenced_assets,
            },
        )?;

        Ok(())
    }
}

/// Serializes `component` onto `node`, mapping the serde failure into an
/// `ImportError` tagged against the `scene` sub-asset.
fn push_node_component<T: Serialize + Component>(
    node: &mut SceneNode,
    component: &T,
) -> Result<(), ImportError> {
    node.push_component(component)
        .map_err(|err| ImportError::SerializationFailed {
            sub_asset_name: "scene".to_string(),
            message: err.to_string(),
        })
}

/// Pushes the mesh/material/render-sync trio a drawable node carries.
fn push_mesh_components(
    node: &mut SceneNode,
    mesh_id: AssetId,
    material_id: AssetId,
) -> Result<(), ImportError> {
    push_node_component(
        node,
        &MeshComponent {
            handle: AssetHandle::weak(mesh_id),
        },
    )?;
    push_node_component(
        node,
        &MaterialComponent::<StandardMaterial> {
            handle: AssetHandle::weak(material_id),
        },
    )?;
    push_node_component(node, &SyncWithRenderWorld)
}

/// Resolves one glTF external resource `uri` against the source file's
/// directory and records it as a cook dependency. `data:` URIs are embedded,
/// not external, so they are skipped; an unreadable path is silently ignored
/// (a genuinely missing external file surfaces from `gltf::import` itself).
fn track_external(ctx: &mut ImportContext, source_dir: &Path, uri: &str) {
    if uri.starts_with("data:") {
        return;
    }
    let path = source_dir.join(percent_decode(uri));
    if let Ok(hash) = hash_file_contents(&path) {
        ctx.track_dependency(path, hash);
    }
}

/// Minimal `%XX` percent-decoder for glTF `uri` fields, which are otherwise
/// plain relative file paths (occasionally with escaped spaces). Bytes that
/// are not a well-formed `%XX` escape pass through untouched.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn texture_name(image_index: usize, srgb: bool) -> String {
    if srgb {
        format!("texture/{image_index}")
    } else {
        format!("texture/{image_index}_linear")
    }
}

fn dynamic_image_from_gltf(data: gltf::image::Data) -> anyhow::Result<image::DynamicImage> {
    let (width, height, format) = (data.width, data.height, data.format);
    let buffer_error = || {
        format!(
            "image buffer does not match dimensions {}x{} for format {:?}",
            width, height, format
        )
    };

    let image = match format {
        gltf::image::Format::R8 => image::DynamicImage::ImageLuma8(
            ImageBuffer::from_vec(width, height, data.pixels).with_context(buffer_error)?,
        ),
        gltf::image::Format::R8G8 => image::DynamicImage::ImageLumaA8(
            ImageBuffer::from_vec(width, height, data.pixels).with_context(buffer_error)?,
        ),
        gltf::image::Format::R8G8B8 => image::DynamicImage::ImageRgb8(
            ImageBuffer::from_vec(width, height, data.pixels).with_context(buffer_error)?,
        ),
        gltf::image::Format::R8G8B8A8 => image::DynamicImage::ImageRgba8(
            ImageBuffer::from_vec(width, height, data.pixels).with_context(buffer_error)?,
        ),
        format => bail!("unsupported GLTF image format {:?}", format),
    };

    Ok(image)
}

fn load_primitive(
    source_path: &Path,
    mesh_name: Option<&str>,
    buffers: &[Data],
    gltf_primitive: &Primitive,
) -> Result<Mesh, ImportError> {
    let context = || {
        format!(
            "primitive {} of mesh '{}'",
            gltf_primitive.index(),
            mesh_name.unwrap_or("<unnamed>")
        )
    };
    let malformed = |message: &str| ImportError::MalformedSource {
        source_path: source_path.to_path_buf(),
        message: format!("{}: {message}", context()),
    };
    let missing = |message: &str| ImportError::MissingRequiredData {
        source_path: source_path.to_path_buf(),
        message: format!("{}: {message}", context()),
    };

    let mut primitive = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    let reader = gltf_primitive.reader(|buffer| Some(&buffers[buffer.index()]));

    primitive.indices = match reader
        .read_indices()
        .ok_or_else(|| missing("GLTF primitive has no indices"))?
    {
        gltf::mesh::util::ReadIndices::U8(iter) => iter.map(|i| i as u32).collect(),
        gltf::mesh::util::ReadIndices::U16(iter) => iter.map(|i| i as u32).collect(),
        gltf::mesh::util::ReadIndices::U32(iter) => iter.collect(),
    };
    primitive.vertices = reader
        .read_positions()
        .ok_or_else(|| missing("GLTF primitive has no vertex positions"))?
        .map(|pos| Vertex {
            pos_coords: pos,
            uv_coords: [0.0; 2],
            normal: [0.0, 0.0, 1.0],
            tangent: [0.0; 3],
            bitangent: [0.0; 3],
            bone_indices: [0; Vertex::MAX_AFFECTED_BONES],
            bone_weights: [0.0; Vertex::MAX_AFFECTED_BONES],
        })
        .collect();

    if let Some(uv_0) = reader.read_tex_coords(0) {
        match uv_0 {
            gltf::mesh::util::ReadTexCoords::F32(iter) => {
                iter.enumerate().for_each(|(index, uvs)| {
                    primitive.vertices[index].uv_coords = uvs;
                });
            }
            _ => {
                return Err(malformed(
                    "unsupported GLTF texture coordinate format (expected F32)",
                ));
            }
        }
    }

    if let Some(normals) = reader.read_normals() {
        normals.enumerate().for_each(|(index, normal)| {
            primitive.vertices[index].normal = normal;
        });
    }

    if let Some(tangents) = reader.read_tangents() {
        tangents.enumerate().for_each(|(index, tangent)| {
            let vertex = &mut primitive.vertices[index];
            vertex.tangent = [tangent[0], tangent[1], tangent[2]];
            // GLTF tangents are vec4; w stores the handedness sign used
            // to reconstruct the bitangent.
            let bitangent = Vec3::from_array(vertex.normal)
                .cross(Vec3::new(tangent[0], tangent[1], tangent[2]))
                * tangent[3];
            vertex.bitangent = bitangent.to_array();
        });
    }

    if let Some(joints_0) = reader.read_joints(0) {
        match joints_0 {
            gltf::mesh::util::ReadJoints::U8(iter) => {
                iter.enumerate().for_each(|(index, joint)| {
                    primitive.vertices[index].bone_indices = [
                        joint[0].into(),
                        joint[1].into(),
                        joint[2].into(),
                        joint[3].into(),
                    ];
                })
            }
            gltf::mesh::util::ReadJoints::U16(iter) => {
                iter.enumerate().for_each(|(index, joint)| {
                    primitive.vertices[index].bone_indices = [
                        joint[0].into(),
                        joint[1].into(),
                        joint[2].into(),
                        joint[3].into(),
                    ];
                })
            }
        }
    }

    if let Some(weights_0) = reader.read_weights(0) {
        match weights_0 {
            gltf::mesh::util::ReadWeights::F32(iter) => {
                iter.enumerate().for_each(|(index, weight)| {
                    primitive.vertices[index].bone_weights = weight;
                });
            }
            _ => {
                return Err(malformed(
                    "unsupported GLTF bone weight format (expected F32)",
                ));
            }
        }
    }

    Ok(primitive)
}
