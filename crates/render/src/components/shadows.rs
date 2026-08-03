use std::{marker::PhantomData, ops::Deref};

use derive_more::Deref;
use ecs::{
    component::Component,
    entity::Entity,
    resource::{Res, ResMut, Resource},
};
use wgpu::BindGroupDescriptor;

use crate::{
    components::light::{push_render_light_to_gpu, LightType, RenderLight},
    device::RenderDevice,
    layouts::{PointShadowLayout, SpotDirectionalShadowLayout},
};

// Cap on simultaneous shadow-casting lights, per pool (spot+directional
// share one pool; point lights have their own — see
// `RenderSpotDirectionalShadowMaps`/`RenderPointShadowMaps`). This is just a
// safety ceiling now, not the allocated size: each pool's actual GPU texture
// tracks real demand via `reconcile_capacity`.
pub(crate) const MAX_SHADOW_CASTERS: u32 = 128;

const SHADOW_MAP_SIZE: u32 = 1024;

// Minimum consecutive frames a pool's usage must stay below its current
// capacity before it actually shrinks. Growth is never delayed — an
// under-sized texture is a correctness bug, not just a memory tradeoff — but
// shrinking immediately would mean a texture recreation on every single
// caster removal, so this absorbs that churn (lights toggling shadows,
// respawning, etc). ~5s at 60fps; tune freely.
const SHADOW_MAP_SHRINK_DELAY_FRAMES: u32 = 300;

// Not `#[derive(Component)]`: needs a custom `on_remove` to free its slot
// (in `RenderSpotDirectionalShadowMaps` or `RenderPointShadowMaps`, depending
// on the light's type) and compact that pool, mirroring `RenderLight`
// (components/light.rs).
#[derive(Clone, Copy, Deref)]
pub struct RenderShadowCasterSlot(pub(crate) u32);

impl RenderShadowCasterSlot {
    pub(crate) fn update_slot(&mut self, new_slot: u32) {
        self.0 = new_slot;
    }
}

impl Component for RenderShadowCasterSlot {
    fn name() -> &'static str {
        std::any::type_name::<RenderShadowCasterSlot>()
    }

    fn on_remove() -> Option<ecs::component::ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let Some(&slot) =
                world.get_component_for_entity::<RenderShadowCasterSlot>(context.entity)
            else {
                return;
            };

            let is_point = world
                .get_component_for_entity::<RenderLight>(context.entity)
                .is_some_and(|light| light.light_type == LightType::Point.index());

            let moved_entity = if is_point {
                let Some(shadow_maps) = world.get_resource_mut::<RenderPointShadowMaps>() else {
                    return;
                };
                shadow_maps.swap_remove_caster(&slot)
            } else {
                let Some(shadow_maps) = world.get_resource_mut::<RenderSpotDirectionalShadowMaps>()
                else {
                    return;
                };
                shadow_maps.swap_remove_caster(&slot)
            };

            if let Some(render_light) =
                world.get_component_for_entity_mut::<RenderLight>(context.entity)
            {
                render_light.shadow_layer = -1;
            }
            push_render_light_to_gpu(&world, context.entity);

            if let Some(moved_entity) = moved_entity {
                if let Some(moved_shadow_slot) =
                    world.get_component_for_entity_mut::<RenderShadowCasterSlot>(moved_entity)
                {
                    moved_shadow_slot.update_slot(*slot);
                }
                if let Some(moved_light) =
                    world.get_component_for_entity_mut::<RenderLight>(moved_entity)
                {
                    moved_light.shadow_layer = *slot as i32;
                }
                push_render_light_to_gpu(&world, moved_entity);
            }
        })
    }
}

// Distinguishes the two shadow-map pool "shapes" that `ShadowMapPool<K>`
// supports. Spot and directional lights each need a single 2D view per
// caster; point lights are omnidirectional and need a full cube (6 views)
// per caster. Everything else — slot allocation, swap-remove compaction,
// grow/shrink-with-hysteresis — is identical between them, so that's all
// implemented once against this trait instead of twice.
pub(crate) trait ShadowMapKind: 'static {
    type Layout: Deref<Target = wgpu::BindGroupLayout>;
    const VIEWS_PER_CASTER: u32;
    const ARRAY_VIEW_DIMENSION: wgpu::TextureViewDimension;
    const LABEL: &'static str;
}

pub(crate) struct SpotDirectionalShadowKind;

impl ShadowMapKind for SpotDirectionalShadowKind {
    type Layout = SpotDirectionalShadowLayout;
    const VIEWS_PER_CASTER: u32 = 1;
    const ARRAY_VIEW_DIMENSION: wgpu::TextureViewDimension = wgpu::TextureViewDimension::D2Array;
    const LABEL: &'static str = "spot_directional_shadow_maps";
}

pub(crate) struct PointShadowKind;

impl ShadowMapKind for PointShadowKind {
    type Layout = PointShadowLayout;
    const VIEWS_PER_CASTER: u32 = 6;
    const ARRAY_VIEW_DIMENSION: wgpu::TextureViewDimension = wgpu::TextureViewDimension::CubeArray;
    const LABEL: &'static str = "point_shadow_maps";
}

// Slot-allocated shadow-map depth-texture array — see `ShadowMapKind` for
// what varies between `RenderSpotDirectionalShadowMaps` (`@group(4)`) and
// `RenderPointShadowMaps` (`@group(5)`). Slots are allocated on demand and
// kept packed via swap-remove, same as `RenderLights::slots`
// (components/light.rs). `capacity` (the actual GPU texture's per-caster
// view count) tracks real demand: it grows immediately when `slots` outgrows
// it, but only shrinks after `SHADOW_MAP_SHRINK_DELAY_FRAMES` consecutive
// frames of lower usage — see `reconcile_capacity`.
pub(crate) struct ShadowMapPool<K: ShadowMapKind> {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) texture: wgpu::Texture,
    // One view per underlying texture layer: a single caster view for
    // `SpotDirectionalShadowKind`, or one cube-face view for `PointShadowKind`.
    pub(crate) views: Vec<wgpu::TextureView>,
    pub(crate) slots: Vec<Entity>,
    capacity: u32,
    frames_below_capacity: u32,
    _kind: PhantomData<fn() -> K>,
}

// Manual impl, like `MaterialPipeline<M>` (material_plugin.rs) —
// #[derive(Resource)] doesn't handle generics/`PhantomData`.
impl<K: ShadowMapKind> Resource for ShadowMapPool<K> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<K: ShadowMapKind> ShadowMapPool<K> {
    pub(crate) fn new(device: &wgpu::Device, layout: &K::Layout) -> Self {
        let (texture, views, bind_group) = Self::build_gpu_resources(device, layout, 1);

        Self {
            bind_group,
            texture,
            views,
            slots: Vec::new(),
            capacity: 1,
            frames_below_capacity: 0,
            _kind: PhantomData,
        }
    }

    fn build_gpu_resources(
        device: &wgpu::Device,
        layout: &K::Layout,
        capacity: u32,
    ) -> (wgpu::Texture, Vec<wgpu::TextureView>, wgpu::BindGroup) {
        let view_count = capacity * K::VIEWS_PER_CASTER;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(K::LABEL),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: view_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Sampled as a whole in the main lighting pass.
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(K::LABEL),
            dimension: Some(K::ARRAY_VIEW_DIMENSION),
            ..Default::default()
        });

        // One single-layer view per underlying texture layer, to target as
        // a depth attachment when rendering into it.
        let views = (0..view_count)
            .map(|layer| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(K::LABEL),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(K::LABEL),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(K::LABEL),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        (texture, views, bind_group)
    }

    // `None` once all `MAX_SHADOW_CASTERS` casters are in use.
    pub(crate) fn push_caster(&mut self, entity: Entity) -> Option<u32> {
        if self.slots.len() as u32 >= MAX_SHADOW_CASTERS {
            return None;
        }
        self.slots.push(entity);
        Some(self.slots.len() as u32 - 1)
    }

    pub(crate) fn swap_remove_caster(&mut self, slot: &RenderShadowCasterSlot) -> Option<Entity> {
        let index = **slot as usize;
        self.slots.swap_remove(index);
        self.slots.get(index).copied()
    }

    // Grows immediately to fit `slots`; shrinks only after
    // `SHADOW_MAP_SHRINK_DELAY_FRAMES` consecutive frames of lower usage.
    pub(crate) fn reconcile_capacity(&mut self, device: &wgpu::Device, layout: &K::Layout) {
        let needed = (self.slots.len() as u32).max(1);

        if needed > self.capacity {
            self.resize_to(device, layout, needed);
        } else if needed < self.capacity {
            self.frames_below_capacity += 1;
            if self.frames_below_capacity >= SHADOW_MAP_SHRINK_DELAY_FRAMES {
                self.resize_to(device, layout, needed);
            }
        } else {
            self.frames_below_capacity = 0;
        }
    }

    fn resize_to(&mut self, device: &wgpu::Device, layout: &K::Layout, capacity: u32) {
        let (texture, views, bind_group) = Self::build_gpu_resources(device, layout, capacity);
        self.texture = texture;
        self.views = views;
        self.bind_group = bind_group;
        self.capacity = capacity;
        self.frames_below_capacity = 0;
    }
}

pub(crate) type RenderSpotDirectionalShadowMaps = ShadowMapPool<SpotDirectionalShadowKind>;
pub(crate) type RenderPointShadowMaps = ShadowMapPool<PointShadowKind>;

// Reconciles both shadow-map pools' GPU texture capacity to actual demand
// once per frame. Runs after commands from `RenderLight::on_add`/
// `RenderShadowCasterSlot::on_remove` have flushed, so if several casters are
// added or removed in the same tick, this only resizes once for the lot —
// not once per entity.
pub(crate) fn resize_shadow_maps(
    mut spot_directional_shadow_maps: ResMut<RenderSpotDirectionalShadowMaps>,
    mut point_shadow_maps: ResMut<RenderPointShadowMaps>,
    device: Res<RenderDevice>,
    spot_directional_shadow_layout: Res<SpotDirectionalShadowLayout>,
    point_shadow_layout: Res<PointShadowLayout>,
) {
    spot_directional_shadow_maps.reconcile_capacity(&device, &spot_directional_shadow_layout);
    point_shadow_maps.reconcile_capacity(&device, &point_shadow_layout);
}
