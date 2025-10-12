use async_trait::async_trait;
use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res,
};
use essential::{
    assets::{
        asset_loader::AssetLoader, asset_server::AssetServer, asset_store::AssetStore,
        handle::AssetHandle, Asset, LoadableAsset,
    },
    transform::Transform,
};
use gltf::{buffer::Data, Node, Primitive};

use crate::{
    assets::{material::Material, mesh::Mesh, texture::Texture, vertex::Vertex},
    components::{material_component::MaterialComponent, mesh_component::MeshComponent},
};
pub(crate) struct GLTFLoader;

pub struct GLTFScene {
    pub(crate) meshes: Vec<GLTFMesh>,
    pub(crate) materials: Vec<AssetHandle<Material>>,
    pub(crate) nodes: Vec<GLTFNode>,
}

impl Asset for GLTFScene {}

pub struct GLTFMesh {
    pub(crate) primitives: Vec<AssetHandle<Mesh>>,
    pub(crate) materials: Vec<Option<usize>>,
}

pub struct GLTFNode {
    pub(crate) children: Vec<GLTFNode>,
    pub(crate) mesh: Option<usize>,
    pub(crate) transform: Transform,
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

        let nodes = document
            .nodes()
            .map(|node| GLTFLoader::extract_node(&node))
            .collect();

        let mut textures = Vec::new();
        for image in images {
            let texture = Texture::from_gltf_data(image);
            textures.push(load_context.asset_server().add(texture));
        }

        let mut materials = Vec::new();
        for gltf_material in document.materials() {
            let material = Material::new(
                gltf_material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map(|texture| textures[texture.texture().index()].clone()),
                gltf_material
                    .normal_texture()
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

        Ok(GLTFScene {
            nodes,
            meshes,
            materials,
        })
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

    fn extract_node(gltf_node: &Node) -> GLTFNode {
        let gltf_transform = gltf_node.transform();

        GLTFNode {
            children: gltf_node
                .children()
                .map(|node| Self::extract_node(&node))
                .collect(),
            mesh: gltf_node.mesh().map(|mesh| mesh.index()),
            transform: Transform::from_matrix(&gltf_transform.matrix()),
        }
    }
}

#[derive(Component)]
pub struct GLTFSpawnerComponent(pub AssetHandle<GLTFScene>);

impl std::ops::Deref for GLTFSpawnerComponent {
    type Target = AssetHandle<GLTFScene>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn spawn_gltf_component(
    mut cmd: CommandQueue,
    gltf_components: Query<(Entity, &GLTFSpawnerComponent)>,
    gltf_assets: Res<AssetStore<GLTFScene>>,
) {
    for (entity, component) in gltf_components.iter() {
        if let Some(asset) = gltf_assets.get(component) {
            for node in &asset.nodes {
                spawn_gltf_node(&asset, &node, &mut cmd);
            }

            cmd.remove::<GLTFSpawnerComponent>(entity);
        }
    }
}

fn spawn_gltf_node(gltf_scene: &GLTFScene, gltf_node: &GLTFNode, cmd: &mut CommandQueue) -> Entity {
    // Spawn node
    let parent_entity = cmd.spawn(gltf_node.transform.clone());

    if let Some(mesh_index) = gltf_node.mesh {
        let gltf_meshes = &gltf_scene.meshes[mesh_index];

        for (mesh, material_index) in gltf_meshes.primitives.iter().zip(&gltf_meshes.materials) {
            if let Some(material_index) = material_index {
                let child = cmd.spawn((
                    gltf_node.transform.clone(),
                    MeshComponent {
                        handle: mesh.clone(),
                    },
                    MaterialComponent {
                        handle: gltf_scene.materials[*material_index].clone(),
                    },
                ));

                cmd.add_child(parent_entity, child);
            }
        }
    }

    // Spawn children
    for gltf_child in &gltf_node.children {
        let child_entity = spawn_gltf_node(gltf_scene, gltf_child, cmd);
        cmd.add_child(parent_entity, child_entity);
    }

    parent_entity
}
