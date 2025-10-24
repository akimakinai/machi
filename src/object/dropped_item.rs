use avian3d::prelude::*;
use bevy::{
    ecs::{entity::EntityHashSet, error::HandleError as _, relationship::RelatedSpawner},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    inventory::Inventory,
    item::{ItemId, ItemStack},
    pause::PausableSystems,
    physics::GameLayer,
};

pub struct DroppedItemPlugin;

impl Plugin for DroppedItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DroppedItemAssets>()
            .add_systems(Update, (merge_items, pickup_items).in_set(PausableSystems));
    }
}

#[derive(Component)]
pub struct DroppedItem {
    item_stack: ItemStack,
}

#[derive(Component)]
struct ItemSensor;

pub fn dropped_item_bundle(
    item_stack: ItemStack,
    item_stack_obj_assets: &DroppedItemAssets,
    overrides: impl Bundle,
) -> Result<impl Bundle> {
    let item_id = item_stack.item_id;

    if item_stack.quantity() == 0 {
        return Err("Cannot create DroppedItem with quantity 0".into());
    }

    let num_cubes = (item_stack.quantity() as f32).log2().ceil() as u32 + 1;

    let cloned_mesh = item_stack_obj_assets.mesh.clone();
    let cloned_material = item_stack_obj_assets
        .material_map
        .get(&item_id)
        .cloned()
        .unwrap_or_default();
    let spawn_blocks = move |parent: &mut RelatedSpawner<ChildOf>| {
        for i in 0..num_cubes {
            let offset = Vec3::new(
                (rand::random::<f32>() - 0.5) * 0.2,
                (rand::random::<f32>() - 0.5) * 0.2,
                (rand::random::<f32>() - 0.5) * 0.2,
            );
            parent.spawn((
                Transform::from_translation(offset + Vec3::Y * (i as f32 * 0.05)),
                Mesh3d(cloned_mesh.clone()),
                MeshMaterial3d(cloned_material.clone()),
            ));
        }
    };

    // Sensor to detect collisions for merging and pickup
    let sensor = (
        Name::new("DroppedItem Sensor"),
        ItemSensor,
        Sphere::new(0.5).collider(),
        Sensor,
        CollisionEventsEnabled,
        CollisionLayers::new(
            [GameLayer::Object],
            [GameLayer::Terrain, GameLayer::Character, GameLayer::Object],
        ),
    );

    Ok((
        Name::new(format!("DroppedItem ({:?})", item_stack)),
        DroppedItem { item_stack },
        Sphere::new(0.2).collider(),
        CollisionLayers::new(
            [GameLayer::Object],
            [GameLayer::Terrain, GameLayer::Character],
        ),
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED,
        Visibility::Visible,
        overrides,
        Children::spawn((Spawn(sensor), SpawnWith(spawn_blocks))),
    ))
}

pub fn spawn_dropped_item(item_stack: ItemStack, overrides: impl Bundle) -> impl Command {
    (move |world: &mut World| -> Result<()> {
        let assets = world.resource::<DroppedItemAssets>();
        let b = dropped_item_bundle(item_stack, assets, overrides)?;
        world.spawn(b);
        Ok(())
    })
    .handle_error()
}

#[derive(Resource)]
pub struct DroppedItemAssets {
    // TODO: mesh should also be a HashMap
    mesh: Handle<Mesh>,
    material_map: HashMap<ItemId, Handle<StandardMaterial>>,
}

impl FromWorld for DroppedItemAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh = meshes.add(Mesh::from(Cuboid::from_length(0.2)));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mut material_map = HashMap::new();
        material_map.insert(
            ItemId(1),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                ..default()
            }),
        );
        material_map.insert(
            ItemId(2),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            }),
        );

        DroppedItemAssets { mesh, material_map }
    }
}

fn merge_items(
    mut commands: Commands,
    mut collision_started: MessageReader<CollisionStart>,
    merge_sensors: Query<&ChildOf, With<ItemSensor>>,
    item_stack_objs: Query<(Entity, &DroppedItem)>,
    item_assets: Res<DroppedItemAssets>,
    transforms: Query<&Transform>,
) -> Result<()> {
    let mut merged = EntityHashSet::default();

    for collision in collision_started.read() {
        let &CollisionStart {
            collider1,
            collider2,
            ..
        } = collision;
        // TODO: handle multiple merges
        if merged.contains(&collider1) || merged.contains(&collider2) {
            continue;
        }
        let Ok([parent1, parent2]) = merge_sensors
            .get_many([collider1, collider2])
            .map(|cs| cs.map(|c| c.parent()))
        else {
            continue;
        };

        let [stack1, stack2] = item_stack_objs.get_many([parent1, parent2])?;

        if stack1.1.item_stack.item_id != stack2.1.item_stack.item_id {
            continue;
        }

        merged.insert(collider1);
        merged.insert(collider2);

        let mid_translation = transforms
            .get_many([parent1, parent2])?
            .map(|t| t.translation)
            .into_iter()
            .sum::<Vec3>()
            / 2.0;

        let total_quantity = stack1.1.item_stack.quantity() + stack2.1.item_stack.quantity();
        if total_quantity > ItemStack::MAX_QUANTITY {
            commands.entity(stack1.0).insert(DroppedItem {
                item_stack: ItemStack::new(stack1.1.item_stack.item_id, ItemStack::MAX_QUANTITY)?,
            });
            commands.entity(stack2.0).insert(DroppedItem {
                item_stack: ItemStack::new(
                    stack2.1.item_stack.item_id,
                    total_quantity - ItemStack::MAX_QUANTITY,
                )?,
            });
            continue;
        }
        let merged_item_stack = ItemStack::new(stack1.1.item_stack.item_id, total_quantity)?;
        commands.entity(stack1.0).despawn();
        commands.entity(stack2.0).despawn();

        commands.spawn(dropped_item_bundle(
            merged_item_stack,
            &item_assets,
            Transform::from_translation(mid_translation),
        )?);
    }

    Ok(())
}

/// A character with this component and an inventory can pick up dropped items.
#[derive(Component, Default, Clone, Copy)]
pub struct PickupItems;

fn pickup_items(
    chars: Query<&Children, With<PickupItems>>,
    mut inventories: Query<&mut Inventory>,
    item_objs: Query<(&DroppedItem, &Transform)>,
    item_sensors: Query<&ChildOf, With<ItemSensor>>,
    mut collision_started: MessageReader<CollisionStart>,
    item_assets: Res<DroppedItemAssets>,
    mut commands: Commands,
) -> Result<()> {
    for collision in collision_started.read() {
        let &CollisionStart {
            collider1,
            collider2,
            ..
        } = collision;

        let (player_children, item_id) = if let Ok(player_children) = chars.get(collider1)
            && let Ok(item_sensor_parent) = item_sensors.get(collider2)
        {
            (player_children, item_sensor_parent.parent())
        } else if let Ok(player_children) = chars.get(collider2)
            && let Ok(item_sensor_parent) = item_sensors.get(collider1)
        {
            (player_children, item_sensor_parent.parent())
        } else {
            continue;
        };

        let (item_obj, item_transform) = item_objs.get(item_id)?;

        let inventory = player_children
            .iter()
            .find(|&c| inventories.contains(c))
            .ok_or("Player has no inventory")?;
        let mut inventory = inventories.get_mut(inventory)?;

        if let Err(remaining) = inventory.add_item_stack(item_obj.item_stack.clone()) {
            if remaining.quantity() == item_obj.item_stack.quantity() {
                continue;
            }

            commands.spawn(dropped_item_bundle(
                remaining,
                &item_assets,
                Transform::from_translation(item_transform.translation),
            )?);
        }

        commands.entity(item_id).despawn();
        debug!("Despawned item {:?}", item_id);
    }

    Ok(())
}
