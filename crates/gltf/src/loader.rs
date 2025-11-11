use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use animation::{
    clip::{AnimationChanelOutput, AnimationChannel, AnimationClip},
    player::AnimationPlayer,
    target::AnimationTarget,
};
use async_trait::async_trait;
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{Query, query_filter::Without},
    resource::Res,
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
use log::warn;
use render::{
    assets::{
        material::Material, mesh::Mesh, skeleton::Skeleton, texture::Texture, vertex::Vertex,
    },
    components::{
        material_component::MaterialComponent, mesh_component::MeshComponent,
        skeleton_component::SkeletonComponent,
    },
};
use uuid::Uuid;

pub(crate) struct GLTFLoader;

#[derive(Asset)]
pub struct GLTFScene {
    pub(crate) meshes: Vec<GLTFMesh>,
    pub(crate) materials: Vec<AssetHandle<Material>>,
    pub(crate) nodes: Vec<GLTFNode>,
    pub(crate) skeletons: Vec<GLTFSkeleton>,
    pub(crate) animations: Vec<AssetHandle<AnimationClip>>,
    pub(crate) target_id_to_node_idx: HashMap<Uuid, GLTFAnimationTargetInfo>,
    pub(crate) animation_roots: HashSet<usize>,
}

impl GLTFScene {
    pub fn animations(&self) -> &Vec<AssetHandle<AnimationClip>> {
        &self.animations
    }
}

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

pub(crate) struct GLTFNodePathInfo {
    pub(crate) root_node: usize,
    pub(crate) node_path: Vec<Cow<'static, str>>,
}

pub(crate) struct GLTFAnimationTargetInfo {
    pub(crate) node_index: usize,
    pub(crate) root_index: usize,
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

        let mut node_paths = HashMap::new();
        for scene in document.scenes() {
            for root_node in scene.nodes() {
                let root_index = root_node.index();
                collect_paths(
                    &root_node,
                    &[],
                    &root_index,
                    &mut node_paths,
                    &mut HashSet::new(),
                );
            }
        }

        let mut target_id_to_node_idx = HashMap::new();
        let mut animation_roots = HashSet::new();
        let mut animation_clips = Vec::new();
        for animation in document.animations() {
            let mut animation_clip = AnimationClip::default();

            for channel in animation.channels() {
                // let mut animation_channel = AnimationChannel::default();

                let target = channel.target();
                let target_node_idx = target.node().index();
                let channel_reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let time_samples = channel_reader
                    .read_inputs()
                    .map(|inputs| inputs.collect::<Vec<_>>());

                let output_samples = channel_reader.read_outputs().map(|outputs| match outputs {
                    gltf::animation::util::ReadOutputs::Translations(iter) => {
                        AnimationChanelOutput::from_translation(iter)
                    }
                    gltf::animation::util::ReadOutputs::Rotations(rotations) => match rotations {
                        gltf::animation::util::Rotations::I8(_) => todo!(),
                        gltf::animation::util::Rotations::U8(_) => todo!(),
                        gltf::animation::util::Rotations::I16(_) => todo!(),
                        gltf::animation::util::Rotations::U16(_) => todo!(),
                        gltf::animation::util::Rotations::F32(iter) => {
                            AnimationChanelOutput::from_rotation(iter)
                        }
                    },
                    gltf::animation::util::ReadOutputs::Scales(iter) => {
                        AnimationChanelOutput::from_scale(iter)
                    }
                    gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => todo!(),
                });

                let Some((time_samples, outputs)) =
                    time_samples.zip(output_samples).filter(|_| true)
                else {
                    continue;
                };

                let animation_channel = AnimationChannel::new(time_samples, outputs);

                // Generate an id
                if let Some(node_path_info) = node_paths.get(&target_node_idx) {
                    let target_id = paths_to_uuid(&node_path_info.node_path);
                    target_id_to_node_idx.insert(
                        target_id.clone(),
                        GLTFAnimationTargetInfo {
                            node_index: target_node_idx,
                            root_index: node_path_info.root_node,
                        },
                    );
                    animation_roots.insert(node_path_info.root_node);
                    animation_clip.add_channel(target_id, animation_channel);
                } else {
                    warn!("Missing an node name for node {}.", target_node_idx);
                }
            }
            animation_clips.push(load_context.asset_server().add(animation_clip));
        }

        Ok(GLTFScene {
            nodes,
            meshes,
            materials,
            skeletons,
            animations: animation_clips,
            target_id_to_node_idx,
            animation_roots,
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

#[derive(Component)]
pub struct GLTFSpawnedMarker {
    animation_roots: Vec<Entity>,
}

impl GLTFSpawnedMarker {
    pub fn new(animation_roots: Vec<Entity>) -> Self {
        Self { animation_roots }
    }

    pub fn animation_roots(&self) -> &Vec<Entity> {
        &self.animation_roots
    }
}

impl std::ops::Deref for GLTFSpawnerComponent {
    type Target = AssetHandle<GLTFScene>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn spawn_gltf_components(
    mut cmd: CommandQueue,
    gltf_components: Query<(Entity, &GLTFSpawnerComponent), Without<GLTFSpawnedMarker>>,
    gltf_assets: Res<AssetStore<GLTFScene>>,
    animation_assets: Res<AssetStore<AnimationClip>>,
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

            // Insert MeshComponents and AnimationPlayers
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

                if asset.animation_roots.contains(&node_index) {
                    cmd.insert(AnimationPlayer::default(), node_entities[node_index]);
                }
            }

            // Insert animation target components
            for animation_clip in asset
                .animations
                .iter()
                .map(|handle| animation_assets.get(handle))
                .filter_map(|value| value)
            {
                for target_id in animation_clip.target_ids() {
                    if let Some(node_info) = asset.target_id_to_node_idx.get(&*target_id) {
                        let target_component = AnimationTarget {
                            id: target_id.clone(),
                            animator: node_entities[node_info.root_index],
                        };

                        let target_entity = node_entities[node_info.node_index];
                        cmd.insert(target_component, target_entity);
                    }
                }
            }

            cmd.insert(
                GLTFSpawnedMarker::new(
                    asset
                        .animation_roots
                        .iter()
                        .map(|node_index| node_entities[*node_index])
                        .collect(),
                ),
                entity,
            );
            // cmd.remove::<GLTFSpawnerComponent>(entity);
        }
    }
}

pub(crate) fn collect_paths(
    node: &Node,
    current_path: &[Cow<'static, str>],
    root_index: &usize,
    paths: &mut HashMap<usize, GLTFNodePathInfo>,
    visited: &mut HashSet<usize>,
) {
    let mut path = current_path.to_owned();
    let node_name = node
        .name()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("GLTF Node: {}", node.index()));

    path.push(Cow::from(node_name));

    visited.insert(node.index());
    for child in node.children() {
        if !visited.contains(&child.index()) {
            collect_paths(&child, &path, &root_index, paths, visited);
        }
    }
    paths.insert(
        node.index(),
        GLTFNodePathInfo {
            root_node: *root_index,
            node_path: path,
        },
    );
}

pub(crate) fn paths_to_uuid(paths: &[Cow<'static, str>]) -> Uuid {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    paths.join("/").hash(&mut hasher);
    Uuid::from_u128(hasher.finish() as u128)
}
