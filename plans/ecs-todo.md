# ECS TODO
---------------

- Replace AnyVec and AnyMap with our own implementations
- Improve logic for finding archetypes:
    - For queries + filters
    - For component insertion/removal through an archetype DAG
- Better caching of matching archetypes for queries
- Add support for component registration (without needing explicit user registration). Used to be able to serialize Worlds, and match a component index to a type.
- Improve cross system communication systems (like Events)