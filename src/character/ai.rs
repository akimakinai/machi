use std::time::Duration;

use bevy::{
    ecs::{
        entity::EntityHashSet,
        system::{BoxedSystem, SystemId, SystemParam, SystemState, lifetimeless::Read},
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
            (update_behavior_trees, reset_leaf_results)
                .chain()
                .in_set(AiActionSystems::PreUpdateAction),
        )
        .add_observer(on_add_active_node)
        .add_observer(on_remove_active_node);
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AiActionSystems {
    PreUpdateAction,
    UpdateAction,
}

#[derive(SystemParam)]
pub struct AiTarget<'w, 's> {
    parent: Query<'w, 's, Read<ChildOf>>,
    root: Query<'w, 's, Read<BehaviorTreeRoot>>,
}

impl<'w, 's> AiTarget<'w, 's> {
    /// Returns the target entity for the behavior tree that `entity` is part of.
    pub fn get_target(&self, mut entity: Entity) -> Result<Entity> {
        if let Ok(root) = self.root.get(entity) {
            return Ok(root.target);
        }
        while let Ok(parent) = self.parent.get(entity).map(|p| p.parent()) {
            entity = parent;
            if let Ok(root) = self.root.get(entity) {
                return Ok(root.target);
            }
        }
        Err("No BehaviorTreeRoot found in parents".into())
    }
}

/// Active leaf nodes should set this to indicate their result to their parent.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LeafNodeResult {
    #[default]
    Idle,
    Continue,
    Complete,
}

impl LeafNodeResult {
    pub fn reset(&mut self) {
        *self = LeafNodeResult::Idle;
    }

    pub fn set_continue(&mut self) {
        *self = LeafNodeResult::Continue;
    }

    pub fn set_complete(&mut self) {
        *self = LeafNodeResult::Complete;
    }
}

/// Marker component for currently active nodes.
/// If a leaf node is active, all its parents are also active.
#[derive(Component)]
pub struct ActiveNode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeResult {
    Continue,
    Complete,
    QueueNode(Entity),
}

#[derive(Component)]
pub struct BehaviorTreeRoot {
    pub target: Entity,
}

impl BehaviorTreeRoot {
    pub fn new(target: Entity) -> Self {
        BehaviorTreeRoot { target }
    }
}

#[derive(Component)]
pub struct ControlNodeSystem(Option<ControlNodeSystemInner>);

// we have to be able to take this out, as `&mut World` is needed to register/run the system
#[derive(Component)]
pub enum ControlNodeSystemInner {
    Uncached(Option<BoxedSystem<In<Entity>, Result<NodeResult>>>),
    Cached(SystemId<In<Entity>, Result<NodeResult>>),
}

impl ControlNodeSystem {
    pub fn new<F, M>(system: F) -> Self
    where
        F: IntoSystem<In<Entity>, Result<NodeResult>, M> + 'static,
    {
        ControlNodeSystem(Some(ControlNodeSystemInner::Uncached(Some(Box::new(
            F::into_system(system),
        )))))
    }
}

impl ControlNodeSystemInner {
    pub fn run(&mut self, world: &mut World, entity: Entity) -> Result<NodeResult> {
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

/// Evaluates and updates all behavior trees.
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
            With<ActiveNode>,
        >,
        Query<(), With<ActiveNode>>,
    )>,
) -> Result<()> {
    let (mut active_nodes, is_active) = queries.get(world);

    let mut node_parents = HashMap::new();
    let mut leaf_nodes = EntityHashSet::new();

    for (id, child_of, children, is_root) in &mut active_nodes {
        if !is_root && let Some(&ChildOf(parent)) = child_of {
            node_parents.insert(id, parent);
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
                NodeResult::QueueNode(new_node) => {
                    if world.get::<ControlNodeSystem>(new_node).is_some() {
                        leaf_nodes.push(new_node);
                        node_parents.insert(new_node, node);
                        debug!("Queued {new_node}");
                    }
                }
                NodeResult::Complete => {
                    if let Some(&parent) = node_parents.get(&node) {
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
            if let Some(&parent) = node_parents.get(&node) {
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

fn reset_leaf_results(mut query: Query<&mut LeafNodeResult, With<ActiveNode>>) {
    for mut result in &mut query {
        result.reset();
    }
}

fn on_add_active_node(
    on: On<Add, ActiveNode>,
    has_result_or_control: Query<(), Or<(With<LeafNodeResult>, With<ControlNodeSystem>)>>,
    mut commands: Commands,
) {
    if !has_result_or_control.contains(on.entity) {
        commands.entity(on.entity).insert(LeafNodeResult::Idle);
    }
}

fn on_remove_active_node(
    on: On<Remove, ActiveNode>,
    has_result: Query<(), With<LeafNodeResult>>,
    mut commands: Commands,
) {
    if has_result.contains(on.entity) {
        commands.entity(on.entity).remove::<LeafNodeResult>();
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
) -> Result<NodeResult> {
    let (&config, state, children) = node.get(world, entity)?;

    if let Some(current) = state.current {
        let current_entity = *children.get(current).ok_or("Invalid child index")?;

        if let Some(&res) = world.get::<LeafNodeResult>(current_entity) {
            if res == LeafNodeResult::Complete {
                world.entity_mut(current_entity).remove::<ActiveNode>();
                debug!("{current} completed");
            } else {
                return Ok(NodeResult::Continue);
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
        world.entity_mut(cur_child).insert(ActiveNode);
        Ok(NodeResult::QueueNode(cur_child))
    } else if config.repeat && current != 0 && !children.is_empty() {
        // Queue child 0 if configured to `repeat`, and `current` ran off indices
        debug!("Repeat");
        state.current = Some(0);
        let child = children[0];
        world.entity_mut(child).insert(ActiveNode);
        Ok(NodeResult::QueueNode(child))
    } else {
        state.current = None;
        world.entity_mut(entity).remove::<ActiveNode>();
        Ok(NodeResult::Complete)
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
) -> Result<NodeResult> {
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
        remove_active_node(world, entity);
        return Ok(NodeResult::Complete);
    }

    // Check if child completed naturally
    if let Some(&result) = world.get::<LeafNodeResult>(child)
        && result == LeafNodeResult::Complete
    {
        let (_, mut state, _) = node.get_mut(world, entity)?;
        state.timer = None;
        state.child_activated = false;
        remove_active_node(world, entity);
        return Ok(NodeResult::Complete);
    }

    if !child_activated {
        let (_, mut state, _) = node.get_mut(world, entity)?;
        state.child_activated = true;
        world.entity_mut(child).insert(ActiveNode);
        return Ok(NodeResult::QueueNode(child));
    }

    Ok(NodeResult::Continue)
}

fn remove_active_node(world: &mut World, entity: Entity) {
    if world.get::<ActiveNode>(entity).is_some() {
        world.entity_mut(entity).remove::<ActiveNode>();
    }
    if let Some(children) = world.get::<Children>(entity) {
        let children = children.to_vec();
        for child in children {
            remove_active_node(world, child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_behavior_tree(world: &mut World) {
        world
            .run_system_cached::<Result<()>, _, _>(update_behavior_trees)
            .unwrap()
            .unwrap();
    }

    #[test]
    fn ai_target_gets_target_from_root_chain() {
        let mut world = World::new();

        let target = world.spawn_empty().id();
        let root = world.spawn(BehaviorTreeRoot::new(target)).id();
        let child = world.spawn(ChildOf(root)).id();
        let grandchild = world.spawn(ChildOf(child)).id();

        let mut state: SystemState<AiTarget> = SystemState::new(&mut world);
        let ai_target = state.get(&world);

        assert_eq!(ai_target.get_target(root).unwrap(), target);
        assert_eq!(ai_target.get_target(child).unwrap(), target);
        assert_eq!(ai_target.get_target(grandchild).unwrap(), target);
    }

    #[test]
    fn sequence_node_queues_children_in_order() {
        let mut world = World::new();

        let node = world
            .spawn((
                SequenceNode { repeat: false },
                SequenceState::default(),
                ControlNodeSystem::new(update_sequence),
                ActiveNode,
            ))
            .id();

        let first_child = world.spawn((LeafNodeResult::Idle, ChildOf(node))).id();
        let second_child = world.spawn((LeafNodeResult::Idle, ChildOf(node))).id();

        run_behavior_tree(&mut world);
        assert!(world.entity(first_child).contains::<ActiveNode>());
        assert!(!world.entity(second_child).contains::<ActiveNode>());

        world
            .get_mut::<LeafNodeResult>(first_child)
            .unwrap()
            .set_complete();

        run_behavior_tree(&mut world);
        assert!(!world.entity(first_child).contains::<ActiveNode>());
        assert!(world.entity(second_child).contains::<ActiveNode>());

        world
            .get_mut::<LeafNodeResult>(second_child)
            .unwrap()
            .set_complete();

        run_behavior_tree(&mut world);
        assert!(!world.entity(node).contains::<ActiveNode>());
        assert!(!world.entity(first_child).contains::<ActiveNode>());
        assert!(!world.entity(second_child).contains::<ActiveNode>());
    }

    #[test]
    fn time_limit_node_completes_when_child_finishes() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());

        let node = world
            .spawn((
                TimeLimitNode::from_seconds(1.0),
                TimeLimitState::default(),
                ControlNodeSystem::new(update_time_limit),
                ActiveNode,
            ))
            .id();
        let child = world.spawn((LeafNodeResult::Idle, ChildOf(node))).id();

        run_behavior_tree(&mut world);
        assert!(world.entity(child).contains::<ActiveNode>());

        world
            .get_mut::<LeafNodeResult>(child)
            .unwrap()
            .set_complete();

        run_behavior_tree(&mut world);
        assert!(!world.entity(node).contains::<ActiveNode>());
        assert!(!world.entity(child).contains::<ActiveNode>());
    }

    #[test]
    fn time_limit_node_completes_when_duration_expires() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());

        let node = world
            .spawn((
                TimeLimitNode::from_seconds(1.0),
                TimeLimitState::default(),
                ControlNodeSystem::new(update_time_limit),
                ActiveNode,
            ))
            .id();
        let child = world.spawn((LeafNodeResult::Idle, ChildOf(node))).id();

        run_behavior_tree(&mut world);
        assert!(world.entity(child).contains::<ActiveNode>());

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.5));
        run_behavior_tree(&mut world);
        assert!(world.entity(node).contains::<ActiveNode>());
        assert!(world.entity(child).contains::<ActiveNode>());

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.6));
        run_behavior_tree(&mut world);
        assert!(!world.entity(node).contains::<ActiveNode>());
        assert!(!world.entity(child).contains::<ActiveNode>());
    }
}
