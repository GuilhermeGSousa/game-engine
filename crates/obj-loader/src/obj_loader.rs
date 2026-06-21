use std::io::{BufRead, BufReader, Cursor};

use anyhow::Context;
use async_trait::async_trait;
use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res,
};
use essential::{
    assets::{
        Asset, AssetPath, LoadableAsset, asset_loader::AssetLoader, asset_server::AssetLoadContext,
        asset_store::AssetStore, handle::AssetHandle, utils::load_to_string,
    },
    transform::Transform,
};
use glam::{Quat, Vec3};
use mesh::mesh::MeshComponent;
use tobj::Model;

use render::{
    assets::{mesh::Mesh, vertex::Vertex},
    components::material::MaterialComponent,
};

use crate::mtl_loader::MTLMaterial;

pub(crate) struct OBJLoader;

#[derive(Asset)]
pub struct OBJAsset {
    meshes: Vec<OBJMesh>,
    materials: Vec<AssetHandle<MTLMaterial>>,
}

impl OBJAsset {
    /// Returns a slice of the meshes contained in this OBJ file.
    pub fn meshes(&self) -> &[OBJMesh] {
        &self.meshes
    }
}

#[derive(Asset)]
pub struct OBJMesh {
    pub handle: AssetHandle<Mesh>,
    pub material_index: Option<usize>,
}

impl LoadableAsset for OBJAsset {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(OBJLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {}
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[allow(deprecated)]
impl AssetLoader for OBJLoader {
    type Asset = OBJAsset;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_setting: <Self::Asset as LoadableAsset>::UsageSettings,
    ) -> anyhow::Result<Self::Asset> {
        let obj_text = load_to_string(path.clone()).await?;
        let obj_cursor = Cursor::new(obj_text);

        let obj_parent = path
            .to_path()
            .parent()
            .context("OBJ path has no parent directory")?
            .to_path_buf();

        let mat_handles = BufReader::new(obj_cursor.clone())
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| {
                if line.starts_with("mtllib") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        let mtl_path = obj_parent.join(parts[1]);
                        Some(load_context.asset_server().load::<MTLMaterial>(mtl_path))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let (models, _) = tobj::load_obj_buf_async(
            &mut BufReader::new(obj_cursor),
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            move |_| async move { Err(tobj::LoadError::GenericFailure) },
        )
        .await
        .with_context(|| format!("failed to parse OBJ file '{}'", path.to_path().display()))?;

        let meshes = models
            .iter()
            .map(|m: &Model| {
                let mut requires_normal_computation = false;
                let mut vertices = (0..m.mesh.positions.len() / 3)
                    .map(|vertex_index| {
                        let uv_coords = match m.mesh.texcoords.len() {
                            0 => [0.0, 0.0],
                            _ => [
                                m.mesh.texcoords[vertex_index * 2],
                                m.mesh.texcoords[vertex_index * 2 + 1],
                            ],
                        };

                        let normal = match m.mesh.normals.len() {
                            0 => {
                                requires_normal_computation = true;
                                [0.0, 0.0, 1.0]
                            }
                            _ => [
                                m.mesh.normals[vertex_index * 3],
                                m.mesh.normals[vertex_index * 3 + 1],
                                m.mesh.normals[vertex_index * 3 + 2],
                            ],
                        };
                        Vertex {
                            pos_coords: [
                                m.mesh.positions[vertex_index * 3],
                                m.mesh.positions[vertex_index * 3 + 1],
                                m.mesh.positions[vertex_index * 3 + 2],
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

                if requires_normal_computation {
                    OBJLoader::compute_normals(m, &mut vertices);
                }

                let mut mesh = Mesh {
                    vertices,
                    indices: m.mesh.indices.clone(),
                };

                if requires_normal_computation {
                    mesh.compute_normals();
                }

                mesh.compute_tangents();
                let handle = load_context.asset_server().add(mesh);

                OBJMesh {
                    handle,
                    material_index: m.mesh.material_id,
                }
            })
            .collect::<Vec<_>>();

        Ok(OBJAsset {
            meshes,
            materials: mat_handles,
        })
    }
}

impl OBJLoader {
    #[deprecated(note = "use Mesh::compute_normals instead")]
    fn compute_normals(model: &Model, vertices: &mut [Vertex]) {
        let mut triangles_included = vec![0; vertices.len()];

        model.mesh.indices.chunks(3).for_each(|index_chunk| {
            let pos0: Vec3 = vertices[index_chunk[0] as usize].pos_coords.into();
            let pos1: Vec3 = vertices[index_chunk[1] as usize].pos_coords.into();
            let pos2: Vec3 = vertices[index_chunk[2] as usize].pos_coords.into();

            let normal = (pos1 - pos0).cross(pos2 - pos0).normalize();

            vertices[index_chunk[0] as usize].normal =
                (normal + Vec3::from(vertices[index_chunk[0] as usize].normal)).into();
            vertices[index_chunk[1] as usize].normal =
                (normal + Vec3::from(vertices[index_chunk[1] as usize].normal)).into();
            vertices[index_chunk[2] as usize].normal =
                (normal + Vec3::from(vertices[index_chunk[2] as usize].normal)).into();

            triangles_included[index_chunk[0] as usize] += 1;
            triangles_included[index_chunk[1] as usize] += 1;
            triangles_included[index_chunk[2] as usize] += 1;
        });

        for (i, n) in triangles_included.into_iter().enumerate() {
            let denom = 1.0 / n as f32;
            vertices[i].normal = (Vec3::from(vertices[i].normal) * denom).normalize().into();
        }
    }
}

#[derive(Component)]
pub struct OBJSpawnerComponent(pub AssetHandle<OBJAsset>);

impl std::ops::Deref for OBJSpawnerComponent {
    type Target = AssetHandle<OBJAsset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn spawn_obj_component(
    mut cmd: CommandQueue,
    objs: Query<(Entity, &OBJSpawnerComponent)>,
    obj_assets: Res<AssetStore<OBJAsset>>,
    mtl_assets: Res<AssetStore<MTLMaterial>>,
) {
    for (entity, component) in objs.iter() {
        if let Some(asset) = obj_assets.get(component) {
            for mesh in &asset.meshes {
                let child_entity = *cmd
                    .spawn((
                        MeshComponent {
                            handle: mesh.handle.clone(),
                        },
                        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
                        MaterialComponent {
                            handle: mtl_assets
                                .get(&asset.materials[mesh.material_index.unwrap_or(0)])
                                .unwrap()
                                .material
                                .clone(),
                        },
                    ))
                    .entity();

                cmd.add_child(entity, child_entity);
            }

            cmd.remove::<OBJSpawnerComponent>(entity);
        }
    }
}
