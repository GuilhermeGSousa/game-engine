"""Game-engine components — author ECS components on objects for GLTF export.

Add components (by name + JSON data) to any object in a sidebar panel; the
add-on mirrors them into an ``obj["components"]`` custom property, which the
glTF exporter writes into each node's ``extras`` (requires *Include > Custom
Properties* in the export dialog, on by default).  The engine's GLTF loader
then turns those entries into real ECS components at spawn time.

See ``core.py`` for the extras contract and the bpy-free helpers.
"""

import bpy
from bpy.app.handlers import persistent

from . import core

bl_info = {
    "name": "Game Engine Components",
    "author": "game-engine",
    "version": (0, 1, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar (N) > Components",
    "description": "Author ECS components on objects, exported via GLTF node extras",
    "category": "Object",
}

# When True, ``sync_object`` is a no-op.  Used while bulk-loading items from an
# existing custom property so per-item update callbacks don't thrash the sync.
_suspend_sync = False


# --------------------------------------------------------------------------- #
# Sync: UI list  <->  obj["components"] custom property
# --------------------------------------------------------------------------- #
def sync_object(obj):
    """Rebuild ``obj["components"]`` from the object's ``game_components`` list.

    Returns a list of human-readable errors (empty on success).  Removes the
    custom property entirely when there are no valid components.
    """
    if _suspend_sync or obj is None:
        return []
    items = [(it.name, it.data) for it in obj.game_components]
    components, errors = core.assemble_components(items)
    try:
        if components:
            obj[core.RESERVED_KEY] = components
        elif core.RESERVED_KEY in obj:
            del obj[core.RESERVED_KEY]
    except (TypeError, ValueError) as exc:
        # Blender custom properties cannot store JSON ``null``; surface it
        # rather than crashing the edit.
        errors.append(
            "Could not write components — Blender custom properties don't "
            "support JSON null: {}".format(exc)
        )
    return errors


def backfill_object(obj):
    """Populate the UI list from an existing ``obj["components"]`` property."""
    global _suspend_sync
    if core.RESERVED_KEY not in obj:
        return
    pairs = core.split_components(obj[core.RESERVED_KEY])
    _suspend_sync = True
    try:
        obj.game_components.clear()
        for name, data_text in pairs:
            item = obj.game_components.add()
            item.name = name
            item.data = data_text
    finally:
        _suspend_sync = False


def _on_item_update(self, context):
    """Validate this item's JSON and re-sync the owning object."""
    ok, err = core.validate_json(self.data)
    new_error = "" if ok else err
    if self.error != new_error:
        self.error = new_error
    sync_object(self.id_data)


# --------------------------------------------------------------------------- #
# Data model
# --------------------------------------------------------------------------- #
class GameComponentItem(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty(
        name="Component",
        description="Component type name (matches the Rust type registered with register_type)",
        default="NewComponent",
        update=_on_item_update,
    )
    data: bpy.props.StringProperty(
        name="Data",
        description="Component fields as a JSON object; use {} for a marker component",
        default="{}",
        update=_on_item_update,
    )
    error: bpy.props.StringProperty(
        name="Error",
        description="Last JSON validation error (empty when valid)",
        default="",
    )


# --------------------------------------------------------------------------- #
# UI
# --------------------------------------------------------------------------- #
class GAME_UL_components(bpy.types.UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row(align=True)
        status = 'ERROR' if item.error else 'CHECKMARK'
        row.label(text=item.name or "(unnamed)", icon=status)


class VIEW3D_PT_game_components(bpy.types.Panel):
    bl_label = "Components"
    bl_idname = "VIEW3D_PT_game_components"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "Components"

    def draw(self, context):
        layout = self.layout
        obj = context.object
        if obj is None:
            layout.label(text="No active object")
            return

        row = layout.row()
        row.template_list(
            "GAME_UL_components", "",
            obj, "game_components",
            obj, "game_components_index",
            rows=3,
        )
        col = row.column(align=True)
        col.operator("object.game_component_add", icon='ADD', text="")
        col.operator("object.game_component_remove", icon='REMOVE', text="")
        col.separator()
        col.operator("object.game_component_sync", icon='FILE_REFRESH', text="")
        col.operator("object.game_component_copy_to_selected", icon='PASTEDOWN', text="")

        idx = obj.game_components_index
        if 0 <= idx < len(obj.game_components):
            item = obj.game_components[idx]
            box = layout.box()
            box.prop(item, "name")
            box.prop(item, "data")
            if item.error:
                box.label(text=item.error, icon='ERROR')

        layout.separator()
        layout.label(text="Export: enable Include > Custom Properties", icon='INFO')


# --------------------------------------------------------------------------- #
# Operators
# --------------------------------------------------------------------------- #
class OBJECT_OT_game_component_add(bpy.types.Operator):
    bl_idname = "object.game_component_add"
    bl_label = "Add Component"
    bl_description = "Add a component to the active object"
    bl_options = {'REGISTER', 'UNDO'}

    @classmethod
    def poll(cls, context):
        return context.object is not None

    def execute(self, context):
        obj = context.object
        item = obj.game_components.add()
        item.name = "NewComponent"
        item.data = "{}"
        obj.game_components_index = len(obj.game_components) - 1
        sync_object(obj)
        return {'FINISHED'}


class OBJECT_OT_game_component_remove(bpy.types.Operator):
    bl_idname = "object.game_component_remove"
    bl_label = "Remove Component"
    bl_description = "Remove the selected component from the active object"
    bl_options = {'REGISTER', 'UNDO'}

    @classmethod
    def poll(cls, context):
        obj = context.object
        return obj is not None and 0 <= obj.game_components_index < len(obj.game_components)

    def execute(self, context):
        obj = context.object
        obj.game_components.remove(obj.game_components_index)
        obj.game_components_index = min(
            obj.game_components_index, len(obj.game_components) - 1
        )
        sync_object(obj)
        return {'FINISHED'}


class OBJECT_OT_game_component_sync(bpy.types.Operator):
    bl_idname = "object.game_component_sync"
    bl_label = "Sync Components"
    bl_description = "Write components to the export custom property now"
    bl_options = {'REGISTER'}

    @classmethod
    def poll(cls, context):
        return context.object is not None

    def execute(self, context):
        errors = sync_object(context.object)
        if errors:
            self.report({'WARNING'}, "; ".join(errors))
        else:
            self.report({'INFO'}, "Components synced")
        return {'FINISHED'}


class OBJECT_OT_game_component_copy_to_selected(bpy.types.Operator):
    bl_idname = "object.game_component_copy_to_selected"
    bl_label = "Copy Components to Selected"
    bl_description = "Copy the active object's components to all other selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    @classmethod
    def poll(cls, context):
        return context.object is not None and len(context.selected_objects) > 1

    def execute(self, context):
        src = context.object
        pairs = [(it.name, it.data) for it in src.game_components]
        for obj in context.selected_objects:
            if obj is src:
                continue
            obj.game_components.clear()
            for name, data_text in pairs:
                item = obj.game_components.add()
                item.name = name
                item.data = data_text
            sync_object(obj)
        self.report({'INFO'}, "Copied components to selected objects")
        return {'FINISHED'}


# --------------------------------------------------------------------------- #
# Handlers
# --------------------------------------------------------------------------- #
@persistent
def _save_pre(_dummy):
    for obj in bpy.data.objects:
        if len(obj.game_components) > 0 or core.RESERVED_KEY in obj:
            sync_object(obj)


@persistent
def _load_post(_dummy):
    for obj in bpy.data.objects:
        if len(obj.game_components) == 0 and core.RESERVED_KEY in obj:
            backfill_object(obj)


# --------------------------------------------------------------------------- #
# Registration
# --------------------------------------------------------------------------- #
_CLASSES = (
    GameComponentItem,
    GAME_UL_components,
    VIEW3D_PT_game_components,
    OBJECT_OT_game_component_add,
    OBJECT_OT_game_component_remove,
    OBJECT_OT_game_component_sync,
    OBJECT_OT_game_component_copy_to_selected,
)


def register():
    for cls in _CLASSES:
        bpy.utils.register_class(cls)
    bpy.types.Object.game_components = bpy.props.CollectionProperty(type=GameComponentItem)
    bpy.types.Object.game_components_index = bpy.props.IntProperty(default=0)
    if _save_pre not in bpy.app.handlers.save_pre:
        bpy.app.handlers.save_pre.append(_save_pre)
    if _load_post not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(_load_post)


def unregister():
    if _save_pre in bpy.app.handlers.save_pre:
        bpy.app.handlers.save_pre.remove(_save_pre)
    if _load_post in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(_load_post)
    del bpy.types.Object.game_components_index
    del bpy.types.Object.game_components
    for cls in reversed(_CLASSES):
        bpy.utils.unregister_class(cls)


if __name__ == "__main__":
    register()
