"""Game-engine components — author ECS components on objects for GLTF export.

Add components to any object in a sidebar panel; each component is a list of
typed fields (key + type + value) edited with proper widgets — no hand-written
JSON.  The add-on mirrors them into an ``obj["components"]`` custom property,
which the glTF exporter writes into each node's ``extras`` (requires *Include >
Custom Properties* in the export dialog, on by default).  The engine's GLTF
loader then turns those entries into real ECS components at spawn time.

See ``core.py`` for the extras contract and the bpy-free helpers.
"""

import json

import bpy
from bpy.app.handlers import persistent

from . import core

bl_info = {
    "name": "Game Engine Components",
    "author": "game-engine",
    "version": (0, 2, 0),
    "blender": (4, 0, 0),
    "location": "View3D > Sidebar (N) > Components",
    "description": "Author ECS components on objects, exported via GLTF node extras",
    "category": "Object",
}

_FIELD_TYPE_ITEMS = [
    ("FLOAT", "Float", "A floating-point number", 0),
    ("INT", "Int", "A whole number", 1),
    ("BOOL", "Bool", "A true/false toggle", 2),
    ("STRING", "String", "Text", 3),
    ("VEC3", "Vec3", "Three floats [x, y, z]", 4),
    ("COLOR", "Color", "An RGBA color", 5),
    ("JSON", "JSON", "Raw JSON — for nested objects or anything else", 6),
]

# When True, ``sync_object`` is a no-op.  Used while bulk-loading fields from an
# existing custom property so per-field update callbacks don't thrash the sync.
_suspend_sync = False


# --------------------------------------------------------------------------- #
# Sync: UI  <->  obj["components"] custom property
# --------------------------------------------------------------------------- #
def _field_neutral(field):
    """Convert a ``GameComponentField`` to the plain dict ``core`` expects."""
    ftype = field.type
    if ftype == "FLOAT":
        value = field.float_value
    elif ftype == "INT":
        value = field.int_value
    elif ftype == "BOOL":
        value = field.bool_value
    elif ftype == "STRING":
        value = field.string_value
    elif ftype == "VEC3":
        value = list(field.vec3_value)
    elif ftype == "COLOR":
        value = list(field.color_value)
    else:  # JSON — raw text
        value = field.string_value
    return {"key": field.key, "type": ftype, "value": value}


def sync_object(obj):
    """Rebuild ``obj["components"]`` from the object's ``game_components``.

    Returns a list of human-readable errors (empty on success).  Per-component
    errors are also written onto each item's ``error`` for inline display.
    """
    if _suspend_sync or obj is None:
        return []

    components = {}
    errors = []
    for item in obj.game_components:
        name = (item.name or "").strip()
        data, item_errors = core.component_from_fields(_field_neutral(f) for f in item.fields)
        if not name:
            item_errors = ["Component has no name"] + item_errors
        joined = "; ".join(item_errors)
        if item.error != joined:
            item.error = joined
        if not name:
            errors.extend(item_errors)
            continue
        if name in components:
            errors.append("Duplicate component '{}' (last one wins)".format(name))
        errors.extend("{}: {}".format(name, e) for e in item_errors)
        components[name] = data

    try:
        if components:
            obj[core.RESERVED_KEY] = components
        elif core.RESERVED_KEY in obj:
            del obj[core.RESERVED_KEY]
    except (TypeError, ValueError) as exc:
        errors.append(
            "Could not write components — Blender custom properties don't "
            "support JSON null: {}".format(exc)
        )
    return errors


def _set_field(field, key, ftype, value):
    field.key = key
    field.type = ftype
    if ftype == "FLOAT":
        field.float_value = float(value)
    elif ftype == "INT":
        field.int_value = int(value)
    elif ftype == "BOOL":
        field.bool_value = bool(value)
    elif ftype == "STRING":
        field.string_value = str(value)
    elif ftype == "VEC3":
        field.vec3_value = value
    elif ftype == "COLOR":
        field.color_value = value
    else:  # JSON
        field.string_value = value if isinstance(value, str) else json.dumps(value)


def backfill_object(obj):
    """Populate the UI from an existing ``obj["components"]`` property."""
    global _suspend_sync
    if core.RESERVED_KEY not in obj:
        return
    plain = core.to_plain(obj[core.RESERVED_KEY])
    if not isinstance(plain, dict):
        return
    _suspend_sync = True
    try:
        obj.game_components.clear()
        for name, data in plain.items():
            item = obj.game_components.add()
            item.name = name
            if isinstance(data, dict):
                for key, ftype, value in core.fields_from_component(data):
                    _set_field(item.fields.add(), key, ftype, value)
    finally:
        _suspend_sync = False


def _resync(self, context):
    """Update callback shared by every editable property."""
    sync_object(self.id_data)


# --------------------------------------------------------------------------- #
# Data model
# --------------------------------------------------------------------------- #
class GameComponentField(bpy.types.PropertyGroup):
    key: bpy.props.StringProperty(name="Key", description="Field name", update=_resync)
    type: bpy.props.EnumProperty(
        name="Type", items=_FIELD_TYPE_ITEMS, default="FLOAT", update=_resync
    )
    float_value: bpy.props.FloatProperty(name="Value", update=_resync)
    int_value: bpy.props.IntProperty(name="Value", update=_resync)
    bool_value: bpy.props.BoolProperty(name="Value", update=_resync)
    string_value: bpy.props.StringProperty(name="Value", update=_resync)
    vec3_value: bpy.props.FloatVectorProperty(name="Value", size=3, update=_resync)
    color_value: bpy.props.FloatVectorProperty(
        name="Value", subtype="COLOR", size=4, min=0.0, max=1.0,
        default=(1.0, 1.0, 1.0, 1.0), update=_resync,
    )


class GameComponentItem(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty(
        name="Component",
        description="Component type name (matches the Rust type registered with register_type)",
        default="NewComponent",
        update=_resync,
    )
    fields: bpy.props.CollectionProperty(type=GameComponentField)
    error: bpy.props.StringProperty(default="")


def _value_prop_name(ftype):
    return {
        "FLOAT": "float_value", "INT": "int_value", "BOOL": "bool_value",
        "STRING": "string_value", "VEC3": "vec3_value", "COLOR": "color_value",
        "JSON": "string_value",
    }[ftype]


# --------------------------------------------------------------------------- #
# UI
# --------------------------------------------------------------------------- #
class GAME_UL_components(bpy.types.UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row(align=True)
        status = 'ERROR' if item.error else 'DOT'
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
        if not (0 <= idx < len(obj.game_components)):
            return
        item = obj.game_components[idx]

        box = layout.box()
        box.prop(item, "name")

        if len(item.fields) == 0:
            box.label(text="No fields (marker component)", icon='RADIOBUT_OFF')

        for i, field in enumerate(item.fields):
            frow = box.row(align=True)
            frow.prop(field, "key", text="")
            frow.prop(field, "type", text="")
            frow.prop(field, _value_prop_name(field.type), text="")
            frow.operator("object.game_component_field_remove", text="", icon='X').index = i

        box.operator("object.game_component_field_add", icon='ADD', text="Add Field")

        if item.error:
            box.label(text=item.error, icon='ERROR')

        layout.separator()
        layout.label(text="Export: enable Include > Custom Properties", icon='INFO')


# --------------------------------------------------------------------------- #
# Operators
# --------------------------------------------------------------------------- #
def _active_item(context):
    obj = context.object
    if obj is None:
        return None
    idx = obj.game_components_index
    if 0 <= idx < len(obj.game_components):
        return obj.game_components[idx]
    return None


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


class OBJECT_OT_game_component_field_add(bpy.types.Operator):
    bl_idname = "object.game_component_field_add"
    bl_label = "Add Field"
    bl_description = "Add a field to the active component"
    bl_options = {'REGISTER', 'UNDO'}

    @classmethod
    def poll(cls, context):
        return _active_item(context) is not None

    def execute(self, context):
        item = _active_item(context)
        field = item.fields.add()
        field.key = "field"
        sync_object(context.object)
        return {'FINISHED'}


class OBJECT_OT_game_component_field_remove(bpy.types.Operator):
    bl_idname = "object.game_component_field_remove"
    bl_label = "Remove Field"
    bl_description = "Remove this field from the active component"
    bl_options = {'REGISTER', 'UNDO'}

    index: bpy.props.IntProperty()

    def execute(self, context):
        item = _active_item(context)
        if item is not None and 0 <= self.index < len(item.fields):
            item.fields.remove(self.index)
            sync_object(context.object)
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
        global _suspend_sync
        src = context.object
        for obj in context.selected_objects:
            if obj is src:
                continue
            _suspend_sync = True
            try:
                obj.game_components.clear()
                for src_item in src.game_components:
                    dst_item = obj.game_components.add()
                    dst_item.name = src_item.name
                    for src_field in src_item.fields:
                        _set_field(
                            dst_item.fields.add(),
                            src_field.key,
                            src_field.type,
                            _field_neutral(src_field)["value"],
                        )
            finally:
                _suspend_sync = False
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
    GameComponentField,
    GameComponentItem,
    GAME_UL_components,
    VIEW3D_PT_game_components,
    OBJECT_OT_game_component_add,
    OBJECT_OT_game_component_remove,
    OBJECT_OT_game_component_field_add,
    OBJECT_OT_game_component_field_remove,
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
