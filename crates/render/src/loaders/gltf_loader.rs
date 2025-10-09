use async_trait::async_trait;
use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res,
};
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetServer, asset_store::AssetStore,
    handle::AssetHandle, Asset, LoadableAsset,
};
use gltf::{buffer::Data, Primitive};

use crate::assets::{
    material::Material,
    mesh::Mesh,
    texture::{Texture, TextureUsageSettings},
    vertex::Vertex,
};
pub(crate) struct GLTFLoader;

pub struct GLTFScene {
    pub(crate) meshes: Vec<GLTFMesh>,
    pub(crate) materials: Vec<AssetHandle<Material>>,
}

impl Asset for GLTFScene {}

pub struct GLTFMesh {
    pub(crate) primitives: Vec<AssetHandle<Mesh>>,
    pub(crate) materials: Vec<Option<usize>>,
}

impl LoadableAsset for GLTFScene {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(GLTFLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for GLTFLoader {
    type Asset = GLTFScene;

    async fn load(
        &self,
        path: essential::assets::AssetPath<'static>,
        load_context: &mut essential::assets::asset_server::AssetLoadContext,
        _usage_setting: <Self::Asset as essential::assets::LoadableAsset>::UsageSettings,
    ) -> Result<Self::Asset, ()> {
        let (document, buffers, images) = gltf::import(path.to_path()).unwrap();

        let mut textures = Vec::new();
        for image in images {
            let texture = Texture::from_bytes(&image.pixels, TextureUsageSettings::default())?;
            textures.push(load_context.asset_server().add(texture));
        }

        let mut materials = Vec::new();
        for gltf_material in document.materials() {
            let material = Material::new(
                gltf_material
                    .occlusion_texture()
                    .map(|texture| textures[texture.texture().index()].clone()),
                gltf_material
                    .occlusion_texture()
                    .map(|texture| textures[texture.texture().index()].clone()),
            );

            materials.push(load_context.asset_server().add(material));
        }

        let mut meshes = Vec::new();
        for mesh in document.meshes() {
            let mut primitives = Vec::new();
            let mut materials = Vec::new();
            for gltf_primitive in mesh.primitives() {
                primitives.push(GLTFLoader::load_primitive(
                    &buffers,
                    &gltf_primitive,
                    load_context.asset_server(),
                )?);
                materials.push(gltf_primitive.material().index());
            }

            meshes.push(GLTFMesh {
                primitives,
                materials,
            });
        }

        Ok(GLTFScene { meshes, materials })
    }
}

impl GLTFLoader {
    fn load_primitive(
        buffers: &Vec<Data>,
        gltf_primitive: &Primitive,
        asset_server: &AssetServer,
    ) -> Result<AssetHandle<Mesh>, ()> {
        let mut primitive = Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
        };

        let reader = gltf_primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        primitive.indices = match reader.read_indices().ok_or(())? {
            gltf::mesh::util::ReadIndices::U8(iter) => iter.map(|i| i as u32).collect(),
            gltf::mesh::util::ReadIndices::U16(iter) => iter.map(|i| i as u32).collect(),
            gltf::mesh::util::ReadIndices::U32(iter) => iter.collect(),
        };
        primitive.vertices = reader
            .read_positions()
            .ok_or(())?
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
                _ => return Err(()),
            }
        }

        if let Some(normals) = reader.read_normals() {
            normals.enumerate().for_each(|(index, normal)| {
                primitive.vertices[index].normal = normal;
            });
        }

        if let Some(tangents) = reader.read_tangents() {
            tangents.enumerate().for_each(|(index, tangent)| {
                primitive.vertices[index].tangent = [tangent[0], tangent[1], tangent[2]];
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
                _ => return Err(()),
            }
        }

        Ok(asset_server.add(primitive))
    }
}

#[derive(Component)]
pub struct GLTFSpawnerComponent(pub AssetHandle<GLTFScene>);

pub(crate) fn spawn_gltf_component(
    mut cmd: CommandQueue,
    objs: Query<(Entity, &GLTFSpawnerComponent)>,
    gltf_assets: Res<AssetStore<GLTFScene>>,
) {
}
