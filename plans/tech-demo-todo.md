## Scope
------------------
### Third Person Controller

### Physics for Movement and Environmental

This one is pretty straight forward, to be able to have a simple character moving about the environment we'll need:
- Line traces
- Static and dynamic colliders, for player capsule and environmental collisions

### Better debug tools

The debug gizmos seriously needs to be fleshed out. As it stands it is buggy and incomplete:
- Add more shapes
- Figure out how to handle their lifetimes

## Implementation
-------------------------------
### Animation
#### Inverse Kinematics

See [TODO](plans/ik-design) for more information on implementation details.
#### Layered Animations

#### 2D Blend Spaces

#### Better node inputs

Currently, the node index must be known to be able to pass data to it. This could be more ergonomic. A couple of ways to do this:
- To have some scratch space that all nodes read from.

#### GLTF Loading ergonomic

The path from a loaded GLTF to a in-game entity can still be improved. This is especially the case when setting up state machines that use animations from other GLTF files.

### Physics

Integrate Jolt bindings for this engine's physics engine. 