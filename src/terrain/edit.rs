use avian3d::prelude::LinearVelocity;
use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    inventory::ItemStack,
    object::item_stack::{ItemStackObjAssets, create_item_stack_obj},
    pause::Pause,
    terrain::chunk::BlockId,
};

use super::chunk::{HoveredBlock, WriteBlocks};

pub struct EditPlugin;

impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
    }
}

fn startup(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) -> Result<()> {
    commands.entity(primary_window.single()?).observe(on_click);
    Ok(())
}

fn on_click(
    on: On<Pointer<Click>>,
    hovered: Res<HoveredBlock>,
    mut blocks: WriteBlocks,
    mut commands: Commands,
    item_assets: Res<ItemStackObjAssets>,
    pause: Res<State<Pause>>,
) -> Result<()> {
    if pause.0 {
        return Ok(());
    }
    let Some(block_pos) = hovered.0 else {
        return Ok(());
    };

    match on.event().button {
        PointerButton::Primary => {
            let block_id = blocks.get_block(block_pos.0)?.0;
            if block_id == BlockId(0) {
                return Ok(());
            }

            blocks.set_block(block_pos.0, BlockId(0))?;
            let random_vel = LinearVelocity(Vec3::new(
                (rand::random::<f32>() - 0.5) * 2.0,
                rand::random::<f32>() * 2.0,
                (rand::random::<f32>() - 0.5) * 2.0,
            ));
            commands.spawn(create_item_stack_obj(
                ItemStack {
                    item_id: block_id.as_item_id(),
                    quantity: 1,
                },
                &item_assets,
                (
                    Transform::from_translation(block_pos.0.as_vec3() + Vec3::splat(0.5)),
                    random_vel,
                ),
            )?);
        }
        PointerButton::Secondary => {
            debug!("Hit pos: {:?}, Hit face: {:?}", block_pos.0, block_pos.1);
            blocks.set_block(block_pos.0 + block_pos.1.normal(), BlockId(1))?;
        }
        _ => {}
    }

    Ok(())
}
