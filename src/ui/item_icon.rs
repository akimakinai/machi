use bevy::{
    asset::RenderAssetUsages,
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
    shader::ShaderRef,
};

use crate::item::ItemId;

pub fn plugin(app: &mut App) {
    app.add_plugins(UiMaterialPlugin::<BlockIconMaterial>::default())
        .add_plugins(UiMaterialPlugin::<ItemIconMaterial>::default())
        .init_resource::<ItemIconRegistry>()
        .add_observer(add_item_icon)
        .add_systems(Startup, register_item_icon_materials);
}

/// UI node that displays an item icon.
#[derive(Component)]
#[require(Node)]
#[component(immutable)]
pub struct ItemIconNode(pub Option<ItemId>);

fn add_item_icon(
    on: On<Insert, ItemIconNode>,
    mut query: Query<(
        &ItemIconNode,
        Option<&mut MaterialNode<BlockIconMaterial>>,
        Option<&mut MaterialNode<ItemIconMaterial>>,
    )>,
    registry: Res<ItemIconRegistry>,
    mut commands: Commands,
) {
    let entity = on.entity;
    let Ok((item_icon, block_material, item_material)) = query.get_mut(entity) else {
        return;
    };
    let Some(item_id) = item_icon.0 else {
        commands.entity(entity).remove::<(
            MaterialNode<ItemIconMaterial>,
            MaterialNode<BlockIconMaterial>,
        )>();
        return;
    };

    if item_id.is_block() {
        if item_material.is_some() {
            commands
                .entity(entity)
                .remove::<MaterialNode<ItemIconMaterial>>();
        }

        let Some(new_mat) = registry.get_block(item_id) else {
            error!("No block icon material for block id {}", item_id.0);
            return;
        };
        if let Some(mut cur_mat) = block_material {
            if cur_mat.0 != new_mat {
                cur_mat.0 = new_mat;
            }
        } else {
            commands.entity(entity).insert(MaterialNode(new_mat));
        }
    } else {
        if block_material.is_some() {
            commands
                .entity(entity)
                .remove::<MaterialNode<BlockIconMaterial>>();
        }

        let Some(new_mat) = registry.get_item(item_id) else {
            error!("No item icon material for item id {}", item_id.0);
            return;
        };
        if let Some(mut cur_mat) = item_material {
            if cur_mat.0 != new_mat {
                cur_mat.0 = new_mat;
            }
        } else {
            commands.entity(entity).insert(MaterialNode(new_mat));
        }
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ItemIconMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub icon: Handle<Image>,
}

impl From<Handle<Image>> for ItemIconMaterial {
    fn from(icon: Handle<Image>) -> Self {
        Self { icon }
    }
}

impl UiMaterial for ItemIconMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/item_icon.wgsl".into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct BlockIconMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub icon: Handle<Image>,
}

impl UiMaterial for BlockIconMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/block_icon.wgsl".into()
    }
}

#[derive(Resource, Default)]
pub struct ItemIconRegistry {
    block_materials: HashMap<ItemId, Handle<BlockIconMaterial>>,
    item_materials: HashMap<ItemId, Handle<ItemIconMaterial>>,
}

impl ItemIconRegistry {
    pub fn register_item(&mut self, item_id: ItemId, material: Handle<ItemIconMaterial>) {
        self.item_materials.insert(item_id, material);
    }

    pub fn get_block(&self, item_id: ItemId) -> Option<Handle<BlockIconMaterial>> {
        self.block_materials.get(&item_id).cloned()
    }

    pub fn get_item(&self, item_id: ItemId) -> Option<Handle<ItemIconMaterial>> {
        self.item_materials.get(&item_id).cloned()
    }
}

fn register_item_icon_materials(
    mut registry: ResMut<ItemIconRegistry>,
    mut block_icon_mats: ResMut<Assets<BlockIconMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let debug_tex = images.add(uv_debug_texture());
    registry.block_materials.insert(
        ItemId(1),
        block_icon_mats.add(BlockIconMaterial {
            icon: debug_tex.clone(),
        }),
    );
    registry.block_materials.insert(
        ItemId(2),
        block_icon_mats.add(BlockIconMaterial {
            icon: debug_tex.clone(),
        }),
    );
}

// Taken from https://bevy.org/examples-webgpu/3d-rendering/3d-shapes/
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
