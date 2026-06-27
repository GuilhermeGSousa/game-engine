# Game Engine TODO list!

- [Tech Demo TODO list](tech-demo-todo.md)
- Simple audio component
- Parallel rendering
- Complete Gameplay crate
- Rename engine (and fine a good name!)
- GLTF loader: parent only the active scene's roots to the spawner entity (currently every parentless node in `document.nodes()` is spawned and parented, which also pulls in other scenes' roots and orphan nodes)