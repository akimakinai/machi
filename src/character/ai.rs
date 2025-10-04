use std::time::Duration;

use bevy::{
    ecs::{
        entity::EntityHashSet,
        system::{BoxedSystem, SystemId, SystemState},
    },
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
        )
        .add_observer(remove_current_result);
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

#[derive(Component, Default, Clone, Copy)]
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

#[derive(Component)]
pub struct ControlNodeSystem(Option<ControlNodeSystemInner>);

#[derive(Component)]
pub enum ControlNodeSystemInner {
    Uncached(Option<BoxedSystem<In<Entity>, Result<AiActionResult>>>),
    Cached(SystemId<In<Entity>, Result<AiActionResult>>),
}

impl ControlNodeSystem {
    pub fn new<F, M>(system: F) -> Self
    where
        F: IntoSystem<In<Entity>, Result<AiActionResult>, M> + 'static,
    {
        ControlNodeSystem(Some(ControlNodeSystemInner::Uncached(Some(Box::new(
            F::into_system(system),
        )))))
    }
}

impl ControlNodeSystemInner {
    pub fn run(&mut self, world: &mut World, entity: Entity) -> Result<AiActionResult> {
        if let ControlNodeSystemInner::Uncached(system) = self {
            *self = ControlNodeSystemInner::Cached(
                world.register_boxed_system(system.take().expect("this can never be None")),
            );
        }
        match self {
            ControlNodeSystemInner::Cached(system_id) => {
                world.run_system_with(*system_id, entity)?
            }
            ControlNodeSystemInner::Uncached(_) => {
                unreachable!("This variant has been eliminated above")
            }
        }
    }
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
        // Take the inner out to regain access to world
        if let Some(mut system) = world
            .get_mut::<ControlNodeSystem>(node)
            .and_then(|mut s| s.0.take())
        {
            let action_result = system.run(world, node)?;
            world
                .get_mut::<ControlNodeSystem>(node)
                .expect("Control node system removed itself")
                .0 = Some(system);
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

fn remove_current_result(
    on: On<Remove, ActiveAiAction>,
    has_result: Query<(), With<CurrentAiActionResult>>,
    mut commands: Commands,
) {
    if has_result.contains(on.entity) {
        commands.entity(on.entity).remove::<CurrentAiActionResult>();
    }
}

/// Control node that runs its children in sequence.
#[derive(Component, Clone, Copy)]
#[require(SequenceState, ControlNodeSystem::new(update_sequence))]
pub struct SequenceNode {
    pub repeat: bool,
}

#[derive(Component, Default)]
struct SequenceState {
    current: Option<usize>,
}

fn update_sequence(
    In(entity): In<Entity>,
    world: &mut World,
    node: &mut QueryState<(&SequenceNode, &mut SequenceState, &Children)>,
) -> Result<AiActionResult> {
    let (&config, state, children) = node.get(world, entity)?;

    if let Some(current) = state.current {
        let current_entity = *children.get(current).ok_or("Invalid child index")?;

        if let Some(&res) = world.get::<CurrentAiActionResult>(current_entity) {
            if res.0 == Some(AiActionResult::Complete) {
                world.entity_mut(current_entity).remove::<ActiveAiAction>();
                debug!("{current} completed");
            } else {
                return Ok(AiActionResult::Continue);
            }
        }
    }

    let (_, mut state, children) = node.get_mut(world, entity)?;
    let current = if let Some(current) = &mut state.current {
        *current += 1;
        *current
    } else {
        state.current = Some(0);
        0
    };

    debug!("Current = {current}");

    if let Some(&cur_child) = children.get(current) {
        world.entity_mut(cur_child).insert(ActiveAiAction);
        Ok(AiActionResult::QueueNode(cur_child))
    } else if config.repeat && current != 0 && !children.is_empty() {
        debug!("Repeat");
        state.current = Some(0);
        let child = children[0];
        world.entity_mut(child).insert(ActiveAiAction);
        Ok(AiActionResult::QueueNode(child))
    } else {
        state.current = None;
        world.entity_mut(entity).remove::<ActiveAiAction>();
        Ok(AiActionResult::Complete)
    }
}

fn children_to_slice(c: Option<&Children>) -> &[Entity] {
    c.map(|c| &**c).unwrap_or(&[])
}

/// Control node that runs a child for a specified duration, then completes.
#[derive(Component, Clone, Copy)]
#[require(TimeLimitState, ControlNodeSystem::new(update_time_limit))]
pub struct TimeLimitNode {
    pub duration: Duration,
}

impl TimeLimitNode {
    pub fn from_seconds(seconds: f32) -> Self {
        TimeLimitNode {
            duration: Duration::from_secs_f32(seconds),
        }
    }
}

#[derive(Component, Default)]
struct TimeLimitState {
    timer: Option<Timer>,
    child_activated: bool,
}

fn update_time_limit(
    In(entity): In<Entity>,
    world: &mut World,
    node: &mut QueryState<(&TimeLimitNode, &mut TimeLimitState, &Children)>,
) -> Result<AiActionResult> {
    let delta = world.resource::<Time>().delta();
    let (&config, mut state, children) = node.get_mut(world, entity)?;

    let &child = children
        .first()
        .ok_or("TimeLimitNode requires exactly one child")?;
    if children.len() > 1 {
        warn!("TimeLimitNode has more than one child, only the first will be used");
    }

    // Initialize timer if not started (first run or after reset)
    if state.timer.is_none() {
        state.timer = Some(Timer::new(config.duration, TimerMode::Once));
        state.child_activated = false;
    }

    let timer_finished = state.timer.as_mut().unwrap().tick(delta).just_finished();
    let child_activated = state.child_activated;

    if timer_finished {
        state.timer = None;
        state.child_activated = false;
        world.entity_mut(child).remove::<ActiveAiAction>();
        world.entity_mut(entity).remove::<ActiveAiAction>();
        return Ok(AiActionResult::Complete);
    }

    // Check if child completed naturally
    if let Some(&result) = world.get::<CurrentAiActionResult>(child)
        && result.0 == Some(AiActionResult::Complete)
    {
        let (_, mut state, _) = node.get_mut(world, entity)?;
        state.timer = None;
        state.child_activated = false;
        world.entity_mut(child).remove::<ActiveAiAction>();
        world.entity_mut(entity).remove::<ActiveAiAction>();
        return Ok(AiActionResult::Complete);
    }

    if !child_activated {
        let (_, mut state, _) = node.get_mut(world, entity)?;
        state.child_activated = true;
        world.entity_mut(child).insert(ActiveAiAction);
        return Ok(AiActionResult::QueueNode(child));
    }

    Ok(AiActionResult::Continue)
}
