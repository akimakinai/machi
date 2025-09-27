use avian3d::prelude::*;
use bevy::{ecs::relationship::RelatedSpawner, platform::collections::HashMap, prelude::*};

use crate::{
    character::Player,
    inventory::{Inventory, ItemId, ItemStack},
    physics::GameLayer,
};

pub struct ItemStackObjPlugin;

impl Plugin for ItemStackObjPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemStackObjAssets>()
            .add_systems(Update, merge_items)
            .add_systems(Update, pickup_items);
    }
}

#[derive(Component)]
pub struct ItemStackObj {
    item_stack: ItemStack,
}

#[derive(Component)]
struct ItemSensor;

pub fn create_item_stack_obj(
    item_stack: ItemStack,
    item_stack_obj_assets: &ItemStackObjAssets,
    overrides: impl Bundle,
) -> Result<impl Bundle> {
    let item_id = item_stack.item_id;

    if item_stack.quantity == 0 {
        return Err("Cannot create ItemStackObj with quantity 0".into());
    }

    let num_cubes = (item_stack.quantity as f32).log2().ceil() as u32 + 1;

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
        Name::new("ItemStackObj Sensor"),
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
        Name::new(format!("ItemStackObj ({:?})", item_stack)),
        ItemStackObj { item_stack },
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

#[derive(Resource)]
pub struct ItemStackObjAssets {
    // TODO: mesh should also be a HashMap
    mesh: Handle<Mesh>,
    material_map: HashMap<ItemId, Handle<StandardMaterial>>,
}

impl FromWorld for ItemStackObjAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh = meshes.add(Mesh::from(Cuboid::from_length(0.2)));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mut material_map = HashMap::new();
        material_map.insert(
            1,
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                ..default()
            }),
        );
        material_map.insert(
            2,
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            }),
        );

        ItemStackObjAssets { mesh, material_map }
    }
}

fn merge_items(
    mut commands: Commands,
    mut collision_started: MessageReader<CollisionStarted>,
    merge_sensors: Query<&ChildOf, With<ItemSensor>>,
    mut item_stack_objs: Query<(Entity, &mut ItemStackObj)>,
    item_assets: Res<ItemStackObjAssets>,
    transforms: Query<&Transform>,
) -> Result<()> {
    let mut merged = HashMap::new();

    for collision in collision_started.read() {
        let &CollisionStarted(entity1, entity2) = collision;
        // TODO: handle multiple merges
        if merged.contains_key(&entity1) || merged.contains_key(&entity2) {
            continue;
        }
        let Ok([parent1, parent2]) = merge_sensors
            .get_many([entity1, entity2])
            .map(|cs| cs.map(|c| c.parent()))
        else {
            continue;
        };

        let [stack1, stack2] = item_stack_objs.get_many([parent1, parent2])?;

        if stack1.1.item_stack.item_id != stack2.1.item_stack.item_id {
            continue;
        }

        let mid_translation = transforms
            .get_many([parent1, parent2])?
            .map(|t| t.translation)
            .into_iter()
            .sum::<Vec3>()
            / 2.0;

        let total_quantity = stack1.1.item_stack.quantity + stack2.1.item_stack.quantity;
        let merged_item_stack = ItemStack {
            item_id: stack1.1.item_stack.item_id,
            quantity: total_quantity,
        };
        merged.insert(stack1.0, (merged_item_stack, mid_translation));
        commands.entity(stack1.0).despawn();
        commands.entity(stack2.0).despawn();
    }

    for (_, (merged_item_stack, mid_translation)) in merged {
        debug!(message = "Merged", item_stack = ?merged_item_stack);
        commands.spawn(create_item_stack_obj(
            merged_item_stack,
            &item_assets,
            Transform::from_translation(mid_translation),
        )?);
    }

    Ok(())
}

fn pickup_items(
    names: Query<NameOrEntity>,
    mut players: Query<&Children, With<Player>>,
    mut inventories: Query<&mut Inventory>,
    item_objs: Query<(&ItemStackObj, &Transform)>,
    item_sensors: Query<&ChildOf, With<ItemSensor>>,
    mut collision_started: MessageReader<CollisionStarted>,
    item_assets: Res<ItemStackObjAssets>,
    mut commands: Commands,
) -> Result<()> {
    for collision in collision_started.read() {
        let &CollisionStarted(entity1, entity2) = collision;

        let (player_children, item_id) = if let Ok(player_children) = players.get(entity1)
            && let Ok(item_sensor_parent) = item_sensors.get(entity2)
        {
            (player_children, item_sensor_parent.parent())
        } else if let Ok(player_children) = players.get(entity2)
            && let Ok(item_sensor_parent) = item_sensors.get(entity1)
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
            if remaining.quantity == item_obj.item_stack.quantity {
                continue;
            }

            commands.spawn(create_item_stack_obj(
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
