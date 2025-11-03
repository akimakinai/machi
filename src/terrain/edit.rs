use avian3d::prelude::LinearVelocity;
use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    item::item_core::ItemStack,
    object::dropped_item::dropped_item_bundle,
    pause::Pause,
    terrain::chunk::{BlockId, BlockIdMap},
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
    pause: Res<State<Pause>>,
    block_id_map: Res<BlockIdMap>,
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

            if let Some(item_id) = block_id_map.get(block_id) {
                let random_vel = LinearVelocity(Vec3::new(
                    (rand::random::<f32>() - 0.5) * 2.0,
                    rand::random::<f32>() * 2.0,
                    (rand::random::<f32>() - 0.5) * 2.0,
                ));
                commands.spawn((
                    dropped_item_bundle(ItemStack::new(item_id, 1)?)?,
                    (
                        Transform::from_translation(block_pos.0.as_vec3() + Vec3::splat(0.5)),
                        random_vel,
                    ),
                ));
            }
        }
        PointerButton::Secondary => {
            debug!("Hit pos: {:?}, Hit face: {:?}", block_pos.0, block_pos.1);
            blocks.set_block(block_pos.0 + block_pos.1.normal(), BlockId(1))?;
        }
        _ => {}
    }

    Ok(())
}
