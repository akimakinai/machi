use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};

use crate::inventory::{ItemId, ItemStack};

pub struct ItemStackObjPlugin;

impl Plugin for ItemStackObjPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemStackObjAssets>();
    }
}

#[derive(Component)]
pub struct ItemStackObj {
    item_stack: ItemStack,
}

pub fn create_item_stack_obj(
    item_stack: ItemStack,
    item_stack_obj_assets: &ItemStackObjAssets,
    overrides: impl Bundle,
) -> Result<impl Bundle> {
    let item_id = item_stack.item_id;
    Ok((
        Name::new(format!("ItemStackObj ({:?})", item_stack)),
        ItemStackObj { item_stack },
        Mesh3d(item_stack_obj_assets.mesh.clone()),
        MeshMaterial3d(
            item_stack_obj_assets
                .material_map
                .get(&item_id)
                .cloned()
                .unwrap_or(default()),
        ),
        Cuboid::from_length(0.2).collider(),
        RigidBody::Dynamic,
        overrides,
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
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            ..default()
        });
        material_map.insert(1, material.clone());
        material_map.insert(2, material);

        ItemStackObjAssets { mesh, material_map }
    }
}
