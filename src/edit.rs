use bevy::prelude::*;

use crate::chunk::{Blocks, HoveredBlock};

pub struct EditPlugin;

impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click);
    }
}

fn on_click(_on: On<Pointer<Click>>, hovered: Res<HoveredBlock>, mut blocks: Blocks) -> Result<()> {
    if let Some(block_pos) = hovered.0 {
        blocks.set_block(block_pos, 0)?;
    }
    Ok(())
}
