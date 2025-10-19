//! Overwrite material alpha for debugging

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
    scene::SceneInstanceReady,
};

/// Add this component to the root of a scene to set alpha for `StandardMaterial::base_color` of all meshes in the scene.
#[derive(Component, Clone)]
#[component(on_add = overwrite_alpha_on_add)]
pub struct OverwriteAlpha(pub f32);

fn overwrite_alpha_on_add(mut world: DeferredWorld, context: HookContext) {
    world
        .commands()
        .entity(context.entity)
        .observe(overwrite_alpha_on_scene_ready);
}

fn overwrite_alpha_on_scene_ready(
    on: On<SceneInstanceReady>,
    overwrite_alpha: Query<&OverwriteAlpha>,
    children: Query<&Children>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let scene_root = on.event().entity;

    let Ok(alpha) = overwrite_alpha.get(scene_root).map(|oa| oa.0) else {
        return;
    };

    children.iter_descendants(scene_root).for_each(|id| {
        if let Ok(mesh_material) = mesh_materials.get(id)
            && let Some(mat) = materials.get_mut(&mesh_material.0)
        {
            mat.base_color = mat.base_color.with_alpha(alpha);
            if mat.alpha_mode == AlphaMode::Opaque && alpha < 1.0 {
                mat.alpha_mode = AlphaMode::AlphaToCoverage;
            }
        }
    });
}
