use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

pub struct BlockIconPlugin;

impl Plugin for BlockIconPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<BlockIconMaterial>::default());
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct BlockIconMaterial {
    // #[texture(0)]
    // #[sampler(1)]
    // pub icon: Handle<Image>,
}

impl UiMaterial for BlockIconMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/block_icon.wgsl".into()
    }
}
