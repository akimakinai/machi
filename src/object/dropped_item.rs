use avian3d::prelude::*;
use bevy::{
    ecs::{entity::EntityHashSet, relationship::RelatedSpawner},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    helper::CommandExt as _,
    inventory::Inventory,
    item::{BlockItem, ItemId, ItemIndex, ItemStack, item_icon::ItemIcon},
    pause::PausableSystems,
    physics::GameLayer,
    startup::StartupSystems,
};

pub struct DroppedItemPlugin;

impl Plugin for DroppedItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DroppedItemAssets>()
            .add_systems(
                Startup,
                setup_assets.in_set(StartupSystems::PostRegisterItems),
            )
            .add_observer(add_item_texture)
            .add_systems(Update, (merge_items, pickup_items).in_set(PausableSystems))
            .add_systems(Update, animate_dropped_items.in_set(PausableSystems));
    }
}

#[derive(Component)]
#[require(Visibility, Transform)]
pub struct DroppedItem;

#[derive(Component)]
struct ItemSensor;

pub fn dropped_item_bundle(item_stack: ItemStack) -> Result<impl Bundle> {
    let item_id = item_stack.item_id;

    if item_stack.quantity() == 0 {
        return Err("Cannot create DroppedItem with quantity 0".into());
    }

    let num_cubes = (item_stack.quantity() as f32).log2().ceil() as u32 + 1;

    // Visual representation of the dropped item
    let visual_spawner = move |parent: &mut RelatedSpawner<ChildOf>| {
        let Ok(item_kind) = parent.world().get_entity(item_id.entity()) else {
            error!("Item kind {:?} not found", item_id.entity());
            return;
        };
        let is_block = item_kind.contains::<BlockItem>();

        if is_block {
            let assets = parent.world().resource::<DroppedItemAssets>();
            let cloned_mesh = assets.block_mesh.clone();
            let cloned_material = assets
                .material_map
                .get(&item_id)
                .cloned()
                .unwrap_or_default();

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
        } else {
            let assets = parent.world().resource::<DroppedItemAssets>();
            parent.spawn((
                Mesh3d(assets.item_mesh.clone()),
                MeshMaterial3d(
                    assets
                        .material_map
                        .get(&item_id)
                        .cloned()
                        .unwrap_or_default(),
                ),
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
        DroppedItem,
        item_stack,
        Sphere::new(0.2).collider(),
        CollisionLayers::new(
            [GameLayer::Object],
            [GameLayer::Terrain, GameLayer::Character],
        ),
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED,
        Children::spawn((Spawn(sensor), SpawnWith(visual_spawner))),
    ))
}

#[derive(Resource, Default)]
pub struct DroppedItemAssets {
    // TODO: mesh should also be a HashMap
    block_mesh: Handle<Mesh>,
    item_mesh: Handle<Mesh>,
    material_map: HashMap<ItemId, Handle<StandardMaterial>>,
}

fn setup_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    item_index: Res<ItemIndex>,
    mut dropped_item_assets: ResMut<DroppedItemAssets>,
) {
    dropped_item_assets.block_mesh = meshes.add(Mesh::from(Cuboid::from_length(0.2)));
    dropped_item_assets.item_mesh =
        meshes.add(Mesh::from(Plane3d::new(-Vec3::Z, Vec2::splat(0.2))));

    dropped_item_assets.material_map.insert(
        item_index.get("grass").unwrap(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0),
            ..default()
        }),
    );
    dropped_item_assets.material_map.insert(
        item_index.get("dirt").unwrap(),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            ..default()
        }),
    );
}

fn add_item_texture(
    on: On<Add, ItemIcon>,
    mut item_assets: ResMut<DroppedItemAssets>,
    q_item_icon: Query<(ItemId, &ItemIcon)>,
    mut sm: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let item_id = on.event().entity;
    debug!("Adding item texture for item {:?}", item_id);
    let (item_id, image) = q_item_icon.get(item_id)?;
    let material = StandardMaterial {
        base_color_texture: Some(image.0.clone()),
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
    };
    item_assets.material_map.insert(item_id, sm.add(material));

    Ok(())
}

fn merge_items(
    mut commands: Commands,
    mut collision_started: MessageReader<CollisionStart>,
    merge_sensors: Query<&ChildOf, With<ItemSensor>>,
    dropped_items: Query<(Entity, &DroppedItem, &ItemStack)>,
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

        let [stack1, stack2] = dropped_items.get_many([parent1, parent2])?;

        if stack1.2.item_id != stack2.2.item_id {
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

        let total_quantity = stack1.2.quantity() + stack2.2.quantity();
        if total_quantity > ItemStack::MAX_QUANTITY {
            commands.entity(stack1.0).insert((
                DroppedItem,
                ItemStack::new(stack1.2.item_id, ItemStack::MAX_QUANTITY)?,
            ));
            commands.entity(stack2.0).insert((
                DroppedItem,
                ItemStack::new(stack2.2.item_id, total_quantity - ItemStack::MAX_QUANTITY)?,
            ));
            continue;
        }
        let merged_item_stack = ItemStack::new(stack1.2.item_id, total_quantity)?;
        commands.entity(stack1.0).despawn();
        commands.entity(stack2.0).despawn();

        commands.spawn((
            dropped_item_bundle(merged_item_stack)?,
            Transform::from_translation(mid_translation),
        ));
    }

    Ok(())
}

/// A character with this component and an inventory can pick up dropped items.
#[derive(Component, Default, Clone, Copy)]
pub struct PickupItems;

fn pickup_items(
    chars: Query<&Children, With<PickupItems>>,
    inventories: Query<(), With<Inventory>>,
    item_objs: Query<(&ItemStack, &Transform), With<DroppedItem>>,
    item_sensors: Query<&ChildOf, With<ItemSensor>>,
    mut collision_started: MessageReader<CollisionStart>,
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

        let (&item_stack, item_transform) = item_objs.get(item_id)?;

        let inventory = player_children
            .iter()
            .find(|&c| inventories.contains(c))
            .ok_or("Player has no inventory")?;

        let item_translation = item_transform.translation;

        commands.queue(Inventory::add_item_stack(inventory, item_stack).pipe(
            move |In(res), world| {
                world.entity_mut(item_id).despawn();

                match res {
                    Ok(remaining) => {
                        if remaining > 0 {
                            world.spawn((
                                dropped_item_bundle(
                                    ItemStack::new(item_stack.item_id, remaining).unwrap(),
                                )
                                .unwrap(),
                                Transform::from_translation(item_translation),
                            ));
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            },
        ));
    }

    Ok(())
}

fn animate_dropped_items(
    mut items: Query<&Children, With<DroppedItem>>,
    mut mesh_tr: Query<&mut Transform, With<Mesh3d>>,
    time: Res<Time>,
) {
    for children in &mut items {
        for child in children.iter() {
            if let Ok(mut transform) = mesh_tr.get_mut(child) {
                transform.rotation *= Quat::from_axis_angle(Vec3::Y, time.delta_secs() * 0.5);
            }
        }
    }
}
