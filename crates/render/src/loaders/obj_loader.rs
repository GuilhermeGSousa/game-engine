use std::io::{BufRead, BufReader, Cursor};

use async_trait::async_trait;
use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res,
};
use essential::{
    assets::{
        asset_loader::AssetLoader, asset_server::AssetLoadContext, asset_store::AssetStore,
        handle::AssetHandle, utils::load_to_string, Asset, AssetPath, LoadableAsset,
    },
    transform::Transform,
};
use glam::{Quat, Vec2, Vec3};
use tobj::Model;

use crate::{
    assets::{material::Material, mesh::Mesh, vertex::Vertex},
    components::{
        material_component::MaterialComponent, mesh_component::MeshComponent,
        render_entity::RenderEntity,
    },
};

pub(crate) struct OBJLoader;

pub struct OBJAsset {
    meshes: Vec<OBJMesh>,
    materials: Vec<AssetHandle<Material>>,
}

pub struct OBJMesh {
    pub handle: AssetHandle<Mesh>,
    pub material_index: Option<usize>,
}

impl Asset for OBJAsset {}

impl LoadableAsset for OBJAsset {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(OBJLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
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
    ) -> Result<Self::Asset, ()> {
        let obj_text = load_to_string(path.clone()).await?;
        let obj_cursor = Cursor::new(obj_text);

        let mat_handles = BufReader::new(obj_cursor.clone())
            .lines()
            .filter_map(Result::ok)
            .filter_map(|line| {
                if line.starts_with("mtllib") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        let mtl_path = path.to_path().parent().unwrap().join(parts[1]);
                        Some(load_context.asset_server().load::<Material>(mtl_path))
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
        .map_err(|_| ())?;

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
                            uv_coords: uv_coords,
                            normal: normal,
                            tangent: [0.0; 3],
                            bitangent: [0.0; 3],
                            bone_indices: [0; Vertex::MAX_AFFECTED_BONES],
                            bone_weights: [0.0; Vertex::MAX_AFFECTED_BONES],
                        }
                    })
                    .collect::<Vec<_>>();

                if requires_normal_computation {
                    OBJLoader::compute_normals(&m, &mut vertices);
                }

                OBJLoader::compute_tangents(&m, &mut vertices);

                let handle = load_context.asset_server().add(Mesh {
                    vertices,
                    indices: m.mesh.indices.clone(),
                });

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
    fn compute_normals(model: &Model, vertices: &mut Vec<Vertex>) {
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

    fn compute_tangents(model: &Model, vertices: &mut Vec<Vertex>) {
        if model.mesh.texcoords.is_empty() {
            for v in vertices.iter_mut() {
                let normal = Vec3::from(v.normal);
                let t = if normal.x.abs() > normal.y.abs() {
                    Vec3::new(normal.z, 0.0, -normal.x).normalize()
                } else {
                    Vec3::new(0.0, -normal.z, normal.y).normalize()
                };
                v.tangent = t.into();
                v.bitangent = normal.cross(t).into();
            }
            return;
        }

        let mut triangles_included = vec![0; vertices.len()];

        model.mesh.indices.chunks(3).for_each(|index_chunk| {
            let v0 = vertices[index_chunk[0] as usize];
            let v1 = vertices[index_chunk[1] as usize];
            let v2 = vertices[index_chunk[2] as usize];

            let pos0: Vec3 = v0.pos_coords.into();
            let pos1: Vec3 = v1.pos_coords.into();
            let pos2: Vec3 = v2.pos_coords.into();

            let uv0: Vec2 = v0.uv_coords.into();
            let uv1: Vec2 = v1.uv_coords.into();
            let uv2: Vec2 = v2.uv_coords.into();

            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

            vertices[index_chunk[0] as usize].tangent =
                (tangent + Vec3::from(vertices[index_chunk[0] as usize].tangent)).into();
            vertices[index_chunk[1] as usize].tangent =
                (tangent + Vec3::from(vertices[index_chunk[1] as usize].tangent)).into();
            vertices[index_chunk[2] as usize].tangent =
                (tangent + Vec3::from(vertices[index_chunk[2] as usize].tangent)).into();

            vertices[index_chunk[0] as usize].bitangent =
                (bitangent + Vec3::from(vertices[index_chunk[0] as usize].bitangent)).into();
            vertices[index_chunk[1] as usize].bitangent =
                (bitangent + Vec3::from(vertices[index_chunk[1] as usize].bitangent)).into();
            vertices[index_chunk[2] as usize].bitangent =
                (bitangent + Vec3::from(vertices[index_chunk[2] as usize].bitangent)).into();

            triangles_included[index_chunk[0] as usize] += 1;
            triangles_included[index_chunk[1] as usize] += 1;
            triangles_included[index_chunk[2] as usize] += 1;
        });

        for (i, n) in triangles_included.into_iter().enumerate() {
            let denom = 1.0 / n as f32;
            let v = &mut vertices[i];
            v.tangent = (Vec3::from(v.tangent) * denom).into();
            v.bitangent = (Vec3::from(v.bitangent) * denom).into();
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
) {
    for (entity, component) in objs.iter() {
        if let Some(asset) = obj_assets.get(component) {
            for mesh in &asset.meshes {
                let child_entity = cmd.spawn((
                    MeshComponent {
                        handle: mesh.handle.clone(),
                    },
                    Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
                    MaterialComponent {
                        handle: asset.materials[mesh.material_index.unwrap_or(0)].clone(),
                    },
                ));

                cmd.add_child(entity, child_entity);
            }

            cmd.remove::<OBJSpawnerComponent>(entity);
        }
    }
}
