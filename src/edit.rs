use bevy::prelude::*;

use crate::chunk::{Blocks, HoveredBlock};

pub struct EditPlugin;

impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click);
    }
}

fn on_click(on: On<Pointer<Click>>, hovered: Res<HoveredBlock>, mut blocks: Blocks) -> Result<()> {
    let Some(block_pos) = hovered.0 else {
        return Ok(());
    };

    match on.event().button {
        PointerButton::Primary => {
            blocks.set_block(block_pos.0, 0)?;
        }
        PointerButton::Secondary => {
            debug!("Hit pos: {:?}, Hit face: {:?}", block_pos.0, block_pos.1);
            blocks.set_block(block_pos.0 + block_pos.1.normal(), 1)?;
        }
        _ => {}
    }

    Ok(())
}
