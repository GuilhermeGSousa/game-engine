use animation::clip::{AnimationChannel, AnimationClip};
use async_trait::async_trait;
use ecs::{
    command::CommandQueue, component::Component, entity::Entity, query::Query, resource::Res,
};
use essential::{
    assets::{
        Asset, LoadableAsset, asset_loader::AssetLoader, asset_server::AssetServer,
        asset_store::AssetStore, handle::AssetHandle,
    },
    transform::Transform,
};
use glam::Mat4;
use gltf::{Node, Primitive, buffer::Data};

use image::ImageBuffer;
use render::{
    assets::{
        material::Material, mesh::Mesh, skeleton::Skeleton, texture::Texture, vertex::Vertex,
    },
    components::{
        material_component::MaterialComponent, mesh_component::MeshComponent,
        skeleton_component::SkeletonComponent,
    },
};
pub(crate) struct GLTFLoader;

pub struct GLTFScene {
    pub(crate) meshes: Vec<GLTFMesh>,
    pub(crate) materials: Vec<AssetHandle<Material>>,
    pub(crate) nodes: Vec<GLTFNode>,
    pub(crate) skeletons: Vec<GLTFSkeleton>,
    pub(crate) animations: Vec<AssetHandle<AnimationClip>>,
}

impl Asset for GLTFScene {}

pub struct GLTFMesh {
    pub(crate) primitives: Vec<AssetHandle<Mesh>>,
    pub(crate) materials: Vec<Option<usize>>,
}

pub struct GLTFNode {
    pub(crate) children: Vec<usize>,
    pub(crate) mesh: Option<usize>,
    pub(crate) skeleton: Option<usize>,
    pub(crate) transform: Transform,
}

pub struct GLTFSkeleton {
    pub(crate) bones: Vec<usize>,
    pub(crate) skeleton: AssetHandle<Skeleton>,
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
        for data in images {
            let image = match data.format {
                gltf::image::Format::R8 => image::DynamicImage::ImageLuma8(
                    ImageBuffer::from_vec(data.width, data.height, data.pixels)
                        .expect("Out of memory loading image."),
                ),
                gltf::image::Format::R8G8 => image::DynamicImage::ImageLumaA8(
                    ImageBuffer::from_vec(data.width, data.height, data.pixels)
                        .expect("Out of memory loading image."),
                ),
                gltf::image::Format::R8G8B8 => image::DynamicImage::ImageRgb8(
                    ImageBuffer::from_vec(data.width, data.height, data.pixels)
                        .expect("Out of memory loading image."),
                ),
                gltf::image::Format::R8G8B8A8 => image::DynamicImage::ImageRgba8(
                    ImageBuffer::from_vec(data.width, data.height, data.pixels)
                        .expect("Out of memory loading image."),
                ),
                _ => panic!("Image format usupported (for now)"),
            };
            let texture = Texture::from_dynamic_image(image);
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

        let mut skeletons = Vec::new();
        for skin in document.skins() {
            if let Some(inverse_bind_matrices) = skin
                .reader(|buffer| Some(&buffers[buffer.index()]))
                .read_inverse_bind_matrices()
                .map(|iter| {
                    iter.map(|pose| Mat4::from_cols_array_2d(&pose))
                        .collect::<Vec<_>>()
                        .into()
                })
            {
                let skeleton = load_context.asset_server().add(inverse_bind_matrices);
                let bones: Vec<usize> = skin.joints().map(|j| j.index()).collect();
                skeletons.push(GLTFSkeleton { bones, skeleton });
            }
        }

        let mut animation_clips = Vec::new();
        for animation in document.animations() {
            let mut animation_clip = AnimationClip::default();

            for channel in animation.channels() {
                let animation_channel = AnimationChannel::default();

                let target = channel.target();
                let target_node = target.node().index();
                let target_property = target.property();
                let channel_reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let time_samples = channel_reader
                    .read_inputs()
                    .map(|inputs| inputs.collect::<Vec<_>>());

                let time_samples = channel_reader.read_outputs().map(|outputs| match outputs {
                    gltf::animation::util::ReadOutputs::Translations(iter) => todo!(),
                    gltf::animation::util::ReadOutputs::Rotations(rotations) => todo!(),
                    gltf::animation::util::ReadOutputs::Scales(iter) => todo!(),
                    gltf::animation::util::ReadOutputs::MorphTargetWeights(
                        morph_target_weights,
                    ) => todo!(),
                });

                animation_clip.add_channel(animation_channel);
            }
            animation_clips.push(load_context.asset_server().add(animation_clip));
        }

        Ok(GLTFScene {
            nodes,
            meshes,
            materials,
            skeletons,
            animations: animation_clips,
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
            children: gltf_node.children().map(|node| node.index()).collect(),
            mesh: gltf_node.mesh().map(|mesh| mesh.index()),
            transform: Transform::from_matrix(&gltf_transform.matrix()),
            skeleton: gltf_node.skin().map(|skin| skin.index()),
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

pub(crate) fn spawn_gltf_components(
    mut cmd: CommandQueue,
    gltf_components: Query<(Entity, &GLTFSpawnerComponent)>,
    gltf_assets: Res<AssetStore<GLTFScene>>,
) {
    for (entity, component) in gltf_components.iter() {
        if let Some(asset) = gltf_assets.get(component) {
            let mut node_entities = Vec::new();

            // Spawn all nodes
            for gltf_node in &asset.nodes {
                let current_entity = cmd.spawn(gltf_node.transform.clone());
                node_entities.push(current_entity);
            }

            // Parent all nodes
            for (node_index, gltf_node) in asset.nodes.iter().enumerate() {
                for child in &gltf_node.children {
                    cmd.add_child(node_entities[node_index], node_entities[*child]);
                }
            }

            // Insert MeshComponents
            for (node_index, gltf_node) in asset.nodes.iter().enumerate() {
                if let Some(gltf_mesh_index) = gltf_node.mesh {
                    let gltf_mesh = &asset.meshes[gltf_mesh_index];

                    let mut primitives = gltf_mesh.primitives.iter().zip(&gltf_mesh.materials);

                    if let Some((first_mesh, material_index)) = primitives.next() {
                        if let Some(material_index) = material_index {
                            cmd.insert(
                                MeshComponent {
                                    handle: first_mesh.clone(),
                                },
                                node_entities[node_index],
                            );

                            cmd.insert(
                                MaterialComponent {
                                    handle: asset.materials[*material_index].clone(),
                                },
                                node_entities[node_index],
                            );
                        }
                    }

                    // TODO: Spawn the rest of the primitives as children
                }

                if let Some(skeleton_index) = gltf_node.skeleton {
                    let gltf_skeleton = &asset.skeletons[skeleton_index];
                    let skeleton_component = SkeletonComponent::new(
                        gltf_skeleton.skeleton.clone(),
                        gltf_skeleton
                            .bones
                            .iter()
                            .map(|bone_index| node_entities[*bone_index])
                            .collect::<Vec<_>>(),
                    );
                    cmd.insert(skeleton_component, node_entities[node_index]);
                }
            }

            cmd.remove::<GLTFSpawnerComponent>(entity);
        }
    }
}
