use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use ecs::{
    entity::Entity,
    query::{query_filter::Changed, Query},
    resource::{Res, ResMut, Resource},
    Without,
};
use essential::{
    assets::AssetId,
    transform::{GlobalTransform, GlobalTransformRaw},
};

use crate::{
    components::{
        material::{MaterialComponent, RenderMaterialComponent},
        mesh::RenderMeshInstance,
        render_entity::RenderEntity,
        skeleton::RenderSkeletonComponent,
    },
    device::RenderDevice,
    queue::RenderQueue,
    Material,
};

const INITIAL_CAPACITY: u32 = 4;

fn stride() -> u64 {
    std::mem::size_of::<GlobalTransformRaw>() as u64
}

// Identifies where a single entity's transform currently lives within an
// `InstanceBatches<M>`.
struct InstanceSlot {
    key: (AssetId, AssetId),
    index: u32,
}

// A single shared GPU-visible instance buffer for all (non-skinned) entities that
// share `(mesh_asset_id, material_asset_id)`. `len` is the number of occupied
// slots and is what gets passed as the instance count to `draw_indexed`.
pub(crate) struct InstanceBatch {
    pub(crate) buffer: wgpu::Buffer,
    capacity: u32,
    pub(crate) len: u32,
}

impl InstanceBatch {
    fn new(device: &RenderDevice, capacity: u32) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Batch Buffer"),
            size: capacity as u64 * stride(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity,
            len: 0,
        }
    }
}

// Per-material-type instance batch table, keyed by `(mesh_asset_id, material_asset_id)`.
//
// Membership (which entities belong to which batch) is tracked persistently in
// `slots` so that `sync_instance_membership` only has to touch the batches whose
// membership actually changed this frame, and `sync_instance_transforms` can write
// a single entity's slot directly without touching the rest of its batch.
pub(crate) struct InstanceBatches<M: 'static> {
    batches: HashMap<(AssetId, AssetId), InstanceBatch>,
    slots: HashMap<Entity, InstanceSlot>,
    _marker: PhantomData<fn() -> M>,
}

impl<M: 'static> InstanceBatches<M> {
    pub(crate) fn new() -> Self {
        Self {
            batches: HashMap::new(),
            slots: HashMap::new(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&(AssetId, AssetId), &InstanceBatch)> {
        self.batches.iter()
    }

    // Rebuilds one batch's GPU buffer from scratch with exactly `members`'
    // transforms (growing the buffer first if it doesn't have enough capacity),
    // and records each member's new slot.
    fn upsert_batch(
        &mut self,
        key: (AssetId, AssetId),
        members: &[(Entity, GlobalTransformRaw)],
        device: &RenderDevice,
        queue: &RenderQueue,
    ) {
        let needed = members.len() as u32;

        let needs_new_buffer = match self.batches.get(&key) {
            Some(batch) => needed > batch.capacity,
            None => true,
        };

        if needs_new_buffer {
            let capacity = needed.max(INITIAL_CAPACITY).next_power_of_two();
            self.batches
                .insert(key, InstanceBatch::new(device, capacity));
        }

        let batch = self
            .batches
            .get_mut(&key)
            .expect("batch was just inserted or already present");

        let raws: Vec<GlobalTransformRaw> = members.iter().map(|(_, raw)| *raw).collect();
        queue.write_buffer(&batch.buffer, 0, bytemuck::cast_slice(&raws));
        batch.len = needed;

        for (index, (entity, _)) in members.iter().enumerate() {
            self.slots.insert(
                *entity,
                InstanceSlot {
                    key,
                    index: index as u32,
                },
            );
        }
    }
}

impl<M: 'static> Default for InstanceBatches<M> {
    fn default() -> Self {
        Self::new()
    }
}

// Manual Resource impl, same reason as `MaterialPipeline<M>`:
// `#[derive(Resource)]` doesn't handle `PhantomData<fn() -> M>`.
impl<M: 'static> Resource for InstanceBatches<M> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

// Diffs this frame's live (non-skinned) mesh instances against `InstanceBatches<M>`'s
// tracked membership and rebuilds only the batches whose membership actually
// changed (entities added, removed, or moved to a different mesh/material). This
// is the only system that structurally mutates `InstanceBatches<M>` (insert /
// remove / grow); per-frame transform updates for already-batched entities are
// handled separately by `sync_instance_transforms`.
pub(crate) fn sync_instance_membership<M: Material>(
    render_mesh_query: Query<
        (Entity, &RenderMeshInstance, &RenderMaterialComponent<M>),
        Without<RenderSkeletonComponent>,
    >,
    mut batches: ResMut<InstanceBatches<M>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let mut groups: HashMap<(AssetId, AssetId), Vec<(Entity, GlobalTransformRaw)>> = HashMap::new();
    let mut current_keys: HashMap<Entity, (AssetId, AssetId)> = HashMap::new();

    for (entity, mesh_instance, mat_comp) in render_mesh_query.iter() {
        let key = (mesh_instance.mesh_asset_id, mat_comp.material_asset_id);
        groups
            .entry(key)
            .or_default()
            .push((entity, mesh_instance.transform_raw));
        current_keys.insert(entity, key);
    }

    let mut dirty_keys: HashSet<(AssetId, AssetId)> = HashSet::new();

    for (entity, slot) in batches.slots.iter() {
        match current_keys.get(entity) {
            Some(key) if *key == slot.key => {}
            _ => {
                dirty_keys.insert(slot.key);
            }
        }
    }
    for (entity, key) in current_keys.iter() {
        match batches.slots.get(entity) {
            Some(slot) if slot.key == *key => {}
            _ => {
                dirty_keys.insert(*key);
            }
        }
    }

    if dirty_keys.is_empty() {
        return;
    }

    batches
        .slots
        .retain(|_, slot| !dirty_keys.contains(&slot.key));

    for key in &dirty_keys {
        match groups.get(key) {
            Some(members) if !members.is_empty() => {
                batches.upsert_batch(*key, members, &device, &queue);
            }
            _ => {
                batches.batches.remove(key);
            }
        }
    }
}

// Mirrors `mesh_changed`'s `Changed<(Transform,)>` pattern, but writes the updated
// matrix directly into the entity's slot in its shared instance batch buffer
// instead of a dedicated per-entity buffer. Entities that haven't been assigned a
// slot yet (brand new this frame) are skipped here — `sync_instance_membership`
// picks them up later in the same frame using `RenderMeshInstance::transform_raw`.
pub(crate) fn sync_instance_transforms<M: Material>(
    meshes: Query<
        (&MaterialComponent<M>, &GlobalTransform, &RenderEntity),
        Changed<(GlobalTransform,)>,
    >,
    batches: Res<InstanceBatches<M>>,
    queue: Res<RenderQueue>,
) {
    for (_, transform, render_entity) in meshes.iter() {
        if let Some(slot) = batches.slots.get(&**render_entity) {
            if let Some(batch) = batches.batches.get(&slot.key) {
                queue.write_buffer(
                    &batch.buffer,
                    slot.index as u64 * stride(),
                    bytemuck::cast_slice(&[transform.to_raw()]),
                );
            }
        }
    }
}
