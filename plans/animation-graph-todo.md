# Animation Graph TODO

- Animation Graph API is backwards. "Root" should be renamed to "output" or "result". "add_node" needs the node to connect into: "parent node" needs to be renamed.
- "add_node" should not return an index. Instead it needs to return a context to which users can add inputs. See state machine api