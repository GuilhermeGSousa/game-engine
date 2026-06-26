# Game Engine TODO list!

- Add simple audio component
- Parallel rendering
- Finish implementing blend trees
- Add IK support (see `docs/ik-design.md`)
- Complete Gameplay crate
- Replace Color usage to use the new Color crate
- Improve error handling when loading assets
- Rename engine (and fine a good name!)
- GLTF loader: parent only the active scene's roots to the spawner entity (currently every parentless node in `document.nodes()` is spawned and parented, which also pulls in other scenes' roots and orphan nodes)