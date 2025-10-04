use bevy::{
    ecs::{entity::EntityHashSet, system::SystemState},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{helper::WorldExt as _, pause::PausableSystems};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            (
                AiActionSystems::PreUpdateAction,
                AiActionSystems::UpdateAction,
            )
                .chain()
                .in_set(PausableSystems),
        )
        .add_systems(
            FixedUpdate,
            (update_behavior_trees, reset_results)
                .chain()
                .in_set(AiActionSystems::PreUpdateAction),
        );
    }
}

// TODO: Control nodes in behavior tree (like Sequence) assigns `ActiveAiAction`
// in `PreUpdateAction` phase, action nodes run in `UpdateAction`

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AiActionSystems {
    PreUpdateAction,
    UpdateAction,
}

// TODO: make this relation
#[derive(Component)]
pub struct AiOf(pub Entity);

#[derive(Component, Default)]
pub struct CurrentAiActionResult(pub Option<AiActionResult>);

#[derive(Component)]
pub struct ActiveAiAction;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiActionResult {
    Continue,
    Complete,
    QueueNode(Entity),
}

#[derive(Component)]
pub struct BehaviorTreeRoot;

#[derive(Component, Clone, Copy)]
pub struct ControlNodeSystem(fn(&mut World, Entity) -> Result<AiActionResult>);

impl ControlNodeSystem {
    pub fn run(&self, world: &mut World, entity: Entity) -> Result<AiActionResult> {
        (self.0)(world, entity)
    }
}

#[derive(Component, Clone, Copy)]
#[require(SequenceState, ControlNodeSystem(update_sequence))]
pub struct SequenceNode {
    pub repeat: bool,
}

#[derive(Component, Default)]
struct SequenceState {
    current: Option<usize>,
}

fn update_sequence(world: &mut World, entity: Entity) -> Result<AiActionResult> {
    let (mut entities, mut commands) = world.entities_and_commands();
    let mut node = entities.get_mut(entity)?;

    let children = node
        .get::<Children>()
        .ok_or("Missing Children")?
        .iter()
        .collect::<Vec<_>>();

    let &config = node
        .get::<SequenceNode>()
        .expect("Update function mismatch with node");

    let state = node
        .get_mut::<SequenceState>()
        .ok_or_else(|| format!("Missing {}", ShortName::of::<SequenceState>()))?;

    if let Some(current) = state.current {
        let current_entity = *children.get(current).ok_or("Invalid child index")?;

        if let Some(res) = entities.get(current_entity)?.get::<CurrentAiActionResult>() {
            if res.0 == Some(AiActionResult::Complete) {
                commands.entity(current_entity).remove::<ActiveAiAction>();
                debug!("{current} completed");
            } else {
                return Ok(AiActionResult::Continue);
            }
        }
    }

    let mut node = entities.get_mut(entity)?;
    let mut state = node
        .get_mut::<SequenceState>()
        .ok_or_else(|| format!("Missing {}", ShortName::of::<SequenceState>()))?;
    let current = if let Some(current) = &mut state.current {
        *current += 1;
        *current
    } else {
        state.current = Some(0);
        0
    };

    debug!("Current = {current}");

    if let Some(&cur_child) = children.get(current) {
        commands.entity(cur_child).insert(ActiveAiAction);
        Ok(AiActionResult::QueueNode(cur_child))
    } else if config.repeat && current != 0 && !children.is_empty() {
        debug!("Repeat");
        state.current = Some(0);
        let child = children[0];
        commands.entity(child).insert(ActiveAiAction);
        Ok(AiActionResult::QueueNode(child))
    } else {
        state.current = None;
        commands.entity(entity).remove::<ActiveAiAction>();
        Ok(AiActionResult::Complete)
    }
}

fn children_to_slice(c: Option<&Children>) -> &[Entity] {
    c.map(|c| &**c).unwrap_or(&[])
}

fn update_behavior_trees(
    world: &mut World,
    queries: &mut SystemState<(
        Query<
            (
                Entity,
                Option<&ChildOf>,
                Option<&Children>,
                Has<BehaviorTreeRoot>,
            ),
            With<ActiveAiAction>,
        >,
        Query<(), With<ActiveAiAction>>,
    )>,
) -> Result<()> {
    let (mut active_nodes, is_active) = queries.get(world);

    let mut node_parent = HashMap::new();
    let mut leaf_nodes = EntityHashSet::new();

    for (id, child_of, children, is_root) in &mut active_nodes {
        if !is_root && let Some(&ChildOf(parent)) = child_of {
            node_parent.insert(id, parent);
        }

        // Nodes without active children are considered leaf nodes
        if !children_to_slice(children)
            .iter()
            .any(|&id| is_active.contains(id))
        {
            leaf_nodes.insert(id);
        }
    }

    // for &node in &leaf_nodes {
    //     debug!("Leaf active node: {}", world.debug_entity(node));
    // }

    let mut leaf_nodes = leaf_nodes.into_iter().collect::<Vec<_>>();

    while let Some(node) = leaf_nodes.pop() {
        if let Some(&system) = world.get::<ControlNodeSystem>(node) {
            let action_result = system.run(world, node)?;
            match action_result {
                AiActionResult::QueueNode(new_node) => {
                    if world.get::<ControlNodeSystem>(new_node).is_some() {
                        leaf_nodes.push(new_node);
                        node_parent.insert(new_node, node);
                        debug!("Queued {new_node}");
                    }
                }
                AiActionResult::Complete => {
                    if let Some(&parent) = node_parent.get(&node) {
                        debug!("Node {node} completed, activating parent {parent}");
                        leaf_nodes.push(parent);
                    } else {
                        debug!("Behavior tree {node} completed");
                    }
                }
                _ => {}
            }
        } else {
            // leaf node is active
            if let Some(&parent) = node_parent.get(&node) {
                debug!("Node {node} is active, activating parent {parent}");
                leaf_nodes.push(parent);
            } else {
                error!(
                    "Non-control node is on root of behavior tree: {}",
                    world.debug_entity(node)?
                );
            }
        }
    }

    Ok(())
}

fn reset_results(
    mut query: Query<&mut CurrentAiActionResult, With<ActiveAiAction>>,
    missing: Query<
        Entity,
        (
            With<ActiveAiAction>,
            Without<CurrentAiActionResult>,
            Without<ControlNodeSystem>,
        ),
    >,
    mut commands: Commands,
) {
    for mut result in &mut query {
        result.0 = None;
    }
    for entity in &missing {
        commands.entity(entity).insert(CurrentAiActionResult(None));
    }
}
