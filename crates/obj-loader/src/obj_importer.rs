//! Offline importer that relocates the runtime `OBJLoader`/`MTLLoader` parsing
//! into the asset-cook pipeline. A single `.obj` (plus the `.mtl` named in its
//! first `mtllib` line) is split into independently-cooked `mesh/*`, a single
//! `material/<mtl stem>`, and one flat `scene` sub-asset, cross-referenced by
//! stable `AssetId`.

use std::path::Path;

use asset_cook::{ImportContext, ImportError, Importer, hash_file_contents};
use color::Color;
use essential::assets::AssetId;
use essential::assets::handle::AssetHandle;
use essential::transform::Transform;
use mesh::mesh::Mesh;
use mesh::vertex::Vertex;
use render::assets::material::StandardMaterial;
use scene::scene::{Scene, SceneNode};

pub struct ObjImporter;

impl Importer for ObjImporter {
    fn supported_extensions(&self) -> &'static [&'static str] {
        &["obj"]
    }

    fn import(&self, source_path: &Path, ctx: &mut ImportContext) -> Result<(), ImportError> {
        let (models, _materials) = tobj::load_obj(
            source_path,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
        )
        .map_err(|err| ImportError::MalformedSource {
            source_path: source_path.to_path_buf(),
            message: format!("failed to parse OBJ file: {err}"),
        })?;

        let mtl_stem = import_material(source_path, ctx)?;

        for (index, model) in models.iter().enumerate() {
            let mesh = build_mesh(&model.mesh);
            ctx.emit(&format!("mesh/{index}"), &mesh)?;
        }

        let nodes = models
            .iter()
            .enumerate()
            .map(|(index, model)| SceneNode {
                name: model.name.clone(),
                transform: Transform::default(),
                children: vec![],
                mesh: Some(AssetHandle::weak(
                    ctx.sub_asset_id(&format!("mesh/{index}")),
                )),
                material: mtl_stem
                    .as_ref()
                    .map(|stem| AssetHandle::weak(ctx.sub_asset_id(&format!("material/{stem}")))),
            })
            .collect();

        ctx.emit("scene", &Scene { nodes })?;

        Ok(())
    }
}

/// Parses the first `mtllib`-referenced `.mtl`, tracks it (and any texture
/// files it names) as build dependencies, and emits a single collapsed
/// `material/<stem>` sub-asset. Returns the stem, or `None` when the OBJ names
/// no material library.
fn import_material(
    source_path: &Path,
    ctx: &mut ImportContext,
) -> Result<Option<String>, ImportError> {
    let obj_text =
        std::fs::read_to_string(source_path).map_err(|err| ImportError::SourceUnreadable {
            source_path: source_path.to_path_buf(),
            message: err.to_string(),
        })?;

    let Some(mtl_name) = obj_text.lines().find_map(|line| {
        if line.starts_with("mtllib") {
            line.split_whitespace().nth(1).map(str::to_string)
        } else {
            None
        }
    }) else {
        return Ok(None);
    };

    let mtl_dir = source_path.parent().unwrap_or_else(|| Path::new(""));
    let mtl_path = mtl_dir.join(&mtl_name);

    let mtl_hash = hash_file_contents(&mtl_path)?;
    ctx.track_dependency(mtl_path.clone(), mtl_hash);

    let (mats, _name_to_index) =
        tobj::load_mtl(&mtl_path).map_err(|err| ImportError::MalformedSource {
            source_path: mtl_path.clone(),
            message: format!("failed to parse MTL file: {err}"),
        })?;

    // Collapse every `newmtl` entry into one StandardMaterial — a verbatim
    // port of the runtime MTLLoader quirk, deliberately preserved here.
    let mut material = StandardMaterial::new(None, None);
    for m in mats {
        if let Some(diffuse_texture) = m.diffuse_texture {
            // TODO(asset-import-pipeline): MTL texture paths are assumed
            // relative to the manifest root, and the standalone ImageImporter
            // always cooks sRGB, so a normal map wired this way loses linear
            // sampling.
            track_texture_dependency(ctx, mtl_dir, &diffuse_texture);
            material.set_base_color_texture(AssetHandle::weak(AssetId::from_path(&format!(
                "{diffuse_texture}#main"
            ))));
        }

        if let Some(normal_texture) = m.normal_texture {
            track_texture_dependency(ctx, mtl_dir, &normal_texture);
            material.set_normal_texture(AssetHandle::weak(AssetId::from_path(&format!(
                "{normal_texture}#main"
            ))));
        }

        if let Some(diffuse) = m.diffuse {
            material.set_base_color_factor(Color::rgba(diffuse[0], diffuse[1], diffuse[2], 1.0));
        }

        if let Some(shininess) = m.shininess {
            // Map Blinn-Phong shininess to an equivalent GGX roughness.
            material.set_roughness_factor((2.0 / (shininess + 2.0)).sqrt().clamp(0.045, 1.0));
        }
    }

    let mtl_stem = mtl_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "material".to_string());

    ctx.emit(&format!("material/{mtl_stem}"), &material)?;

    Ok(Some(mtl_stem))
}

fn track_texture_dependency(ctx: &mut ImportContext, mtl_dir: &Path, texture_name: &str) {
    let texture_path = mtl_dir.join(texture_name);
    if let Ok(hash) = hash_file_contents(&texture_path) {
        ctx.track_dependency(texture_path, hash);
    }
}

/// Per-vertex assembly plus the normal/tangent fallback — a verbatim port of
/// the runtime `OBJLoader::load` mesh path. `single_index` + `triangulate`
/// guarantee `positions`/`texcoords`/`normals` are parallel per-vertex arrays
/// and `indices` are triangle lists.
fn build_mesh(mesh_data: &tobj::Mesh) -> Mesh {
    let mut requires_normal_computation = false;

    let vertices = (0..mesh_data.positions.len() / 3)
        .map(|vertex_index| {
            let uv_coords = match mesh_data.texcoords.len() {
                0 => [0.0, 0.0],
                _ => [
                    mesh_data.texcoords[vertex_index * 2],
                    mesh_data.texcoords[vertex_index * 2 + 1],
                ],
            };

            let normal = match mesh_data.normals.len() {
                0 => {
                    requires_normal_computation = true;
                    [0.0, 0.0, 1.0]
                }
                _ => [
                    mesh_data.normals[vertex_index * 3],
                    mesh_data.normals[vertex_index * 3 + 1],
                    mesh_data.normals[vertex_index * 3 + 2],
                ],
            };

            Vertex {
                pos_coords: [
                    mesh_data.positions[vertex_index * 3],
                    mesh_data.positions[vertex_index * 3 + 1],
                    mesh_data.positions[vertex_index * 3 + 2],
                ],
                uv_coords,
                normal,
                tangent: [0.0; 3],
                bitangent: [0.0; 3],
                bone_indices: [0; Vertex::MAX_AFFECTED_BONES],
                bone_weights: [0.0; Vertex::MAX_AFFECTED_BONES],
            }
        })
        .collect::<Vec<_>>();

    let mut mesh = Mesh {
        vertices,
        indices: mesh_data.indices.clone(),
    };

    if requires_normal_computation {
        mesh.compute_normals();
    }
    mesh.compute_tangents();

    mesh
}
