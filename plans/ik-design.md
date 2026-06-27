# Inverse Kinematics — Design Notes

Status: **proposal / not implemented**

This document captures what the animation system needs in order to support
inverse kinematics (IK), based on the state of the engine after root-motion
extraction (`AnimationRootBone.displacement`). It is a planning document; nothing
here is built yet.

## Goal

Support runtime IK constraints — at minimum a two-bone analytic solver for limbs
(legs/arms), with a path toward iterative solvers (CCD/FABRIK), look-at, and
foot-locking. IK must compose cleanly with the existing blend-tree animation
output rather than replacing it.

## Where we are today

The current pipeline is purely feed-forward and local-space:

- `Pose` is a flat `Box<[JointPose]>` of **local** TRS values, indexed by bone
  index (`crates/animation/src/pose.rs`).
- `Skeleton` carries only `inverse_bindposes: Box<[Mat4]>`
  (`crates/mesh/src/skeleton.rs`). It has **no parent/topology** information.
- `SkeletonComponent` carries `bones: Vec<Entity>` and `bone_ids: Vec<Uuid>`, in
  bone-index order.
- `AnimationGraphInstance::evaluate` collapses the blend tree to a single local
  pose (`crates/animation/src/graph.rs`). `AnimationPlayer::evaluate` then writes
  that pose straight onto each bone's `Transform`
  (`crates/animation/src/player.rs`).
- World-space transforms (`GlobalTransform`) only exist *after* a separate
  transform-propagation pass that runs once `animate_targets` has written the
  locals. The animation layer never sees model space.
- Bone hierarchy exists at load time (glTF `children: Vec<usize>`) and in the ECS
  entity hierarchy, but **not** in any structure the animation graph can read.
- Root motion is now separated onto `AnimationRootBone.displacement` instead of
  being baked into the root bone's transform.

The root-motion separation is groundwork IK wants (foot-locking and stride
warping need root translation decoupled from the pose), but it does not by itself
add any IK capability.

## Core gaps

### 1. No skeleton topology in the pose/animation layer (blocker)

IK walks a chain of bones, which requires **parent indices in pose-index space**.
Today that mapping lives only in the glTF nodes and the ECS hierarchy, neither of
which the graph can see.

Proposed: extend the `Skeleton` asset (or a parallel structure carried with the
pose) with:

- `parents: Box<[Option<usize>]>` — parent bone index per bone (index space must
  match `inverse_bindposes` / `SkeletonComponent::bones`).
- bind-pose **local** transforms per bone, and/or bone lengths, for solvers and
  for model-space reconstruction.

The glTF loader already resolves bone names (it does so for `root_bone`); the
parent array can be built from the same skin/joint data at load time.

### 2. No model-space pass (blocker)

IK solvers operate in model/world space: gather each chain bone's global
transform, solve, write rotations back to local. The engine only produces global
transforms *after* animation, via `GlobalTransform` propagation — too late, and
on the wrong side of the pose.

Proposed: a conversion utility inside the animation crate that, given a local
pose + topology, produces model-space transforms and writes the solved result
back to local. Run inside the animation step, before transforms are written.

### 3. No post-processing stage and no effector input channel (blocker)

`evaluate` produces one local pose and immediately writes it. IK constraints are
normally post-process nodes or a constraint stack applied to the final blended
pose. Two sub-gaps:

- **No hook** between "graph produced pose" and "write to transforms".
- **No way to feed runtime targets in.** `AnimationGraphContext` carries only
  `animation_clips` + `animation_graphs`. IK needs per-frame inputs: goal
  position/entity, pole vector, chain definition, blend weight, enable flag.

Proposed: an IK constraint component, e.g.

```rust
struct IkConstraint {
    chain_root: /* bone index or id */,
    effector:   /* bone index or id */,
    goal:       Goal,      // world-space Vec3 or target Entity
    pole:       Option<Goal>,
    weight:     f32,       // blend alpha vs. animated pose
    enabled:    bool,
}
```

queried in `animate_targets` and applied as a post-blend pass.

### 4. No per-bone masking / partial application

`Pose::blend` is whole-pose only. IK that affects just a leg chain needs to blend
its result against the animated pose **per bone** (a bone mask). Needed for
clean composition and for layered/additive application generally.

### 5. No semantic bone addressing

Setting up a chain requires naming "left foot / knee pole / hand". We have
`bone_ids: Vec<Uuid>` and flat indices but no exposed name→index / role mapping.
Generalize the loader's existing name lookup and store it.

### 6. Scale / rigid-bone contract (minor)

`JointPose` flags an open question (`// Uniform scaling?`). IK math assumes rigid
bones; non-uniform scale complicates the model-space compose/decompose
round-trip. Decide the scale contract before building solvers on top.

## Proposed sequencing

1. **Topology** — add parent indices + bind locals to `Skeleton`/pose layer.
   Unblocks everything else.
2. **Model-space conversion** — utility for local pose + topology ↔ model space.
3. **Constraint component + post-blend pass** — query an `IkConstraint`-style
   component in `animate_targets`; implement a single **two-bone analytic
   solver** first (legs/arms): simplest, deterministic, covers most game cases.
4. **Bone masking** — so IK blends cleanly against the animated pose.
5. **Richer behaviour** — CCD/FABRIK, look-at, and foot-locking that consumes
   `AnimationRootBone.displacement`.

## Open questions

- Should topology live on the `Skeleton` asset or in a separate
  pose-companion structure shared across instances?
- Goals as world-space `Vec3`, target `Entity`, or both?
- Where exactly does the IK pass run relative to transform propagation — inside
  `animate_targets`, or as a distinct system scheduled between animation and
  propagation?
