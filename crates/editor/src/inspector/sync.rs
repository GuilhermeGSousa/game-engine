//! Reconciles the live target with card/row entities. Snapshots only live on rows;
//! collection creates temporary values which are moved into those components.
use super::*;

use ecs::{component::Tick, entity::hierarchy::ChildOf, query::filter::With};

/// A component card in the inspector's UI hierarchy. Query this component to
/// discover which live world component a card inspects. Its child property rows
/// own the snapshots and widget state.
#[derive(Component, Clone, Copy)]
pub struct InspectedComponent {
    pub entity: Entity,
    pub type_id: TypeId,
    pub name: &'static str,
    body: Entity,
    last_read_tick: Option<Tick>,
    registry_revision: Option<u64>,
}

/// Widget creation is deferred to a regular system with a CommandQueue, keeping
/// the adapter API independent of the exclusive reflection/reconciliation pass.
#[derive(Component)]
pub(super) struct BuildPropertyWidget;

pub(super) fn sync_inspected_components(world: &mut World) {
    let target = world
        .get_resource::<InspectorData>()
        .and_then(|data| data.entity)
        .filter(|&entity| world.entity_is_valid(entity));
    let mut stacks = world.query::<Entity, With<ComponentStack>>();
    let stack = stacks.iter(world).next();
    let mut cards = world.query::<(Entity, &InspectedComponent, Option<&ChildOf>), ()>();
    let cards: Vec<_> = cards
        .iter(world)
        .map(|(entity, card, parent)| (entity, *card, parent.map(ChildOf::parent)))
        .collect();

    // Descriptors are temporary discovery data, never another persistent model.
    let registry = world.get_resource::<InspectorRegistry>();
    let registry_revision = registry.map(InspectorRegistry::revision);
    let mut desired: Vec<_> = target
        .into_iter()
        .flat_map(|entity| world.component_ids(entity))
        .filter_map(|&type_id| {
            let name = registry
                .and_then(|registry| registry.component(type_id))
                .map(|c| c.name)
                .or_else(|| world.type_info(type_id).map(|info| info.short()))?;
            Some((type_id, name))
        })
        .collect();
    desired.sort_by(|(a_id, a), (b_id, b)| a.cmp(b).then(a_id.cmp(b_id)));

    let mut retained = Vec::new();
    for (entity, card, parent) in cards {
        if stack.is_none()
            || parent != stack
            || Some(card.entity) != target
            || !desired.iter().any(|(type_id, _)| *type_id == card.type_id)
        {
            world.despawn_recursive(entity);
        } else {
            retained.push((entity, card));
        }
    }
    let (Some(stack), Some(target)) = (stack, target) else {
        return;
    };
    let Some(theme) = world.get_resource::<UITheme>().cloned() else {
        return;
    };
    let mut ordered = Vec::with_capacity(desired.len());
    for (type_id, name) in desired {
        let (entity, mut card) = retained
            .iter()
            .find(|(_, card)| card.type_id == type_id)
            .copied()
            .unwrap_or_else(|| spawn_card(world, stack, target, type_id, name, &theme));
        ordered.push(entity);
        if card.last_read_tick.is_none()
            || card.registry_revision != registry_revision
            || card
                .last_read_tick
                .is_some_and(|tick| world.component_changed_since(target, type_id, tick))
        {
            let properties = world
                .get_resource::<InspectorRegistry>()
                .and_then(|registry| registry.collect_component(world, target, type_id))
                .unwrap_or_default();
            reconcile_rows(world, &card, properties, &theme);
            card.last_read_tick = Some(world.current_tick());
            card.registry_revision = registry_revision;
            *world
                .get_component_for_entity_mut::<InspectedComponent>(entity)
                .expect("retained card") = card;
        }
    }
    order_children(world, stack, &ordered);
}

fn child_entities(world: &World, parent: Entity) -> Vec<Entity> {
    world
        .get_component_for_entity::<Children>(parent)
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default()
}

fn order_children(world: &mut World, parent: Entity, ordered: &[Entity]) {
    if world
        .get_component_for_entity::<Children>(parent)
        .is_some_and(|children| !children.iter().copied().eq(ordered.iter().copied()))
    {
        world
            .get_component_for_entity_mut::<Children>(parent)
            .expect("existing children")
            .sort_by_key(|child| {
                ordered
                    .iter()
                    .position(|&entity| entity == child)
                    .unwrap_or(usize::MAX)
            });
    }
}

fn reconcile_rows(
    world: &mut World,
    card: &InspectedComponent,
    properties: Vec<Property>,
    theme: &UITheme,
) {
    let visible = !properties.is_empty();
    if world
        .get_component_for_entity::<UINode>(card.body)
        .is_some_and(|node| node.visible != visible)
    {
        world
            .get_component_for_entity_mut::<UINode>(card.body)
            .expect("card body")
            .visible = visible;
    }
    let mut existing = child_entities(world, card.body);
    let mut ordered = Vec::with_capacity(properties.len());
    for property in properties {
        let matching = existing.iter().position(|&entity| {
            world
                .get_component_for_entity::<PropertyRow>(entity)
                .is_some_and(|row| {
                    row.entity == card.entity
                        && row.component == card.type_id
                        && row.path == property.path
                        && row.type_id == property.type_id
                        && row.registration == property.registration
                })
        });
        let row = if let Some(index) = matching {
            let row = existing.swap_remove(index);
            if world.get_component_for_entity::<PropertyRowValue>(row) != Some(&property.value) {
                world.insert(property.value, row);
            }
            row
        } else {
            spawn_row(world, card, property, theme)
        };
        ordered.push(row);
    }
    for row in existing {
        world.despawn_recursive(row);
    }
    order_children(world, card.body, &ordered);
}

fn spawn_card(
    world: &mut World,
    stack: Entity,
    target: Entity,
    type_id: TypeId,
    name: &'static str,
    theme: &UITheme,
) -> (Entity, InspectedComponent) {
    let entity = world.spawn((
        UINode {
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UIRect::axes(theme.spacing_xs + 2.0, theme.spacing_sm),
            ..Default::default()
        },
        UIMaterial {
            corner_radius: theme.radius_md,
            ..UIMaterial::flat(theme.surface_raised)
        },
    ));
    world.add_child(stack, entity);
    let header = world.spawn(UINode {
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        align_items: Some(taffy::AlignItems::Center),
        gap: glam::Vec2::new(theme.spacing_xs + 2.0, 0.0),
        ..Default::default()
    });
    world.add_child(entity, header);
    let label = world.spawn((
        UINode {
            flex_grow: 1.0,
            ..Default::default()
        },
        TextComponent {
            ellipsis: true,
            wrap: false,
            ..text(theme, name)
        },
    ));
    world.add_child(header, label);
    // Component enable/disable is not implemented; keep its visual disabled.
    let toggle = world.spawn((
        UINode {
            width: UIValue::Px(22.0),
            height: UIValue::Px(13.0),
            flex_shrink: 0.0,
            ..Default::default()
        },
        UIMaterial {
            corner_radius: 6.5,
            ..UIMaterial::flat(theme.accent)
        },
        UIDisabled,
    ));
    world.add_child(header, toggle);
    let body = world.spawn(UINode {
        visible: false,
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        gap: glam::Vec2::new(0.0, theme.spacing_xs),
        padding: UIRect {
            top: theme.spacing_xs,
            ..Default::default()
        },
        ..Default::default()
    });
    world.add_child(entity, body);
    let card = InspectedComponent {
        entity: target,
        type_id,
        name,
        body,
        last_read_tick: None,
        registry_revision: None,
    };
    world.insert(card, entity);
    (entity, card)
}

fn spawn_row(
    world: &mut World,
    card: &InspectedComponent,
    property: Property,
    theme: &UITheme,
) -> Entity {
    let target = property.row(card.entity, card.type_id);
    let label = if property.path.depth() == 0 {
        card.name.to_string()
    } else {
        label_for(&property.path)
    };
    let row = world.spawn((
        UINode {
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: Some(taffy::AlignItems::Center),
            gap: glam::Vec2::new(theme.spacing_xs, 0.0),
            ..Default::default()
        },
        target,
        property.value,
        BuildPropertyWidget,
    ));
    world.add_child(card.body, row);
    let label = world.spawn((
        UINode {
            width: UIValue::Px(PROPERTY_LABEL_WIDTH),
            flex_shrink: 0.0,
            ..Default::default()
        },
        TextComponent {
            color: theme.text_muted,
            font_size: theme.font_size_sm,
            line_height: theme.line_height(theme.font_size_sm),
            wrap: false,
            ellipsis: true,
            ..text(theme, &label)
        },
    ));
    world.add_child(row, label);
    row
}

pub(super) fn build_property_widgets(
    rows: Query<(Entity, &PropertyRow, &PropertyRowValue), With<BuildPropertyWidget>>,
    registry: Res<InspectorRegistry>,
    theme: Res<UITheme>,
    mut cmd: CommandQueue,
) {
    for (entity, row, value) in rows.iter() {
        if let Some(editor) = registry
            .editor(row.type_id)
            .filter(|editor| Some(editor.id) == row.registration)
        {
            if let Err(error) = editor.adapter.build(&mut cmd, entity, value, &theme) {
                log::warn!("Unable to build property widget: {error}");
            }
        } else {
            let unsupported = cmd
                .spawn((UINode::default(), text(&theme, "Unsupported type")))
                .entity();
            cmd.add_child(entity, unsupported);
        }
        cmd.remove::<BuildPropertyWidget>(entity);
    }
}
