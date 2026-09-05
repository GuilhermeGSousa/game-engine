use animation::player::AnimationPlayer;
use animation::root::AnimationRootBone;
use ecs::component::scene::{SceneComponent, SceneEntityRef, SceneSpawnContext};
use ecs::{Component, Entity};
use essential::assets::{asset_server::AssetServer, handle::AssetHandle};
use mesh::skeleton::{Skeleton, SkeletonComponent};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Skeleton binding as authored into a scene. Bones are node indices
/// here because `SkeletonComponent` holds `Entity`, which cannot exist at rest.
///
/// Derives `Component` only to satisfy the `SceneComponent: Component` bound —
/// `apply` never inserts one, so no entity ever carries it.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SceneSkeleton {
    pub skeleton: AssetHandle<Skeleton>,
    pub bones: Vec<SceneEntityRef>,
    pub bone_ids: Vec<Uuid>,
    pub root: Option<SceneEntityRef>,
}

impl SceneComponent for SceneSkeleton {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        let bones: Vec<Entity> = self
            .bones
            .iter()
            .filter_map(|reference| ctx.entity_for(*reference))
            .collect();

        let skeleton = match ctx.world().get_resource::<AssetServer>() {
            Some(server) => server.load_by_id(self.skeleton.id()),
            None => self.skeleton.clone(),
        };

        if let Some(root) = self.root.and_then(|reference| ctx.entity_for(reference)) {
            ctx.insert(AnimationRootBone::default(), root);
        }

        ctx.insert(AnimationPlayer::new(bones.len()), entity);
        ctx.insert(
            SkeletonComponent::new(skeleton, bones, self.bone_ids),
            entity,
        );
    }
}

/// The skeleton binding without the player. Serialized onto the appended primitive
/// child nodes of a multi-primitive skinned mesh so each drawable primitive
/// skins from the same bone entities the owning node's `AnimationPlayer` drives.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SceneSkeletonBinding {
    pub skeleton: AssetHandle<Skeleton>,
    pub bones: Vec<SceneEntityRef>,
    pub bone_ids: Vec<Uuid>,
}

impl SceneComponent for SceneSkeletonBinding {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>) {
        let bones: Vec<Entity> = self
            .bones
            .iter()
            .filter_map(|reference| ctx.entity_for(*reference))
            .collect();
        let skeleton = match ctx.world().get_resource::<AssetServer>() {
            Some(server) => server.load_by_id(self.skeleton.id()),
            None => self.skeleton.clone(),
        };
        ctx.insert(
            SkeletonComponent::new(skeleton, bones, self.bone_ids),
            entity,
        );
    }
}
