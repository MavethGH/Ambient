use std::{collections::HashMap, sync::Arc};

use ambient_core::{
    player::{player, user_id},
    runtime,
};
use ambient_ecs::{
    query, ArchetypeFilter, Component, ComponentQuery, ComponentValue, ECSError, EntityId,
    FrameEvent, QueryState, TypedReadQuery, World, WorldDiff,
};
use ambient_element::{Hooks, Setter};
use ambient_std::{cb, Cb};

use crate::{
    client::ClientState, log_network_result, persistent_resources, rpc::rpc_world_diff,
    synced_resources,
};

pub fn use_remote_world_system<
    'a,
    R: ComponentQuery<'a> + Clone + 'static,
    F: Fn(&TypedReadQuery<R>, &mut World, Option<&mut QueryState>, &FrameEvent)
        + Send
        + Sync
        + 'static,
>(
    hooks: &mut Hooks,
    query: TypedReadQuery<R>,
    run: F,
) {
    if let Some((client_state, _)) = hooks.consume_context::<ClientState>() {
        let query_state = hooks.use_ref_with(|_| QueryState::new());
        hooks.use_frame(move |_| {
            let mut game_state = client_state.game_state.lock();
            let mut qs = query_state.lock();
            run(&query, &mut game_state.world, Some(&mut qs), &FrameEvent);
        });
    }
}

pub fn use_remote_component<T: ComponentValue + std::fmt::Debug>(
    hooks: &mut Hooks,
    entity: EntityId,
    component: Component<T>,
) -> Result<T, ECSError> {
    let (client_state, _) = hooks.consume_context::<ClientState>().unwrap();
    let component_version = hooks.use_ref_with(|_| {
        let game_state = client_state.game_state.lock();
        game_state
            .world
            .get_component_content_version(entity, component.index())
            .ok()
    });
    let (value, set_value) = hooks.use_state_with(|_| {
        let game_state = client_state.game_state.lock();
        game_state.world.get_ref(entity, component).cloned()
    });
    hooks.use_frame(move |_| {
        let game_state = client_state.game_state.lock();
        let mut cv = component_version.lock();
        let version = game_state
            .world
            .get_component_content_version(entity, component.index())
            .ok();
        if *cv != version {
            *cv = version;
            set_value(game_state.world.get_ref(entity, component).cloned());
        }
    });
    value
}

#[allow(clippy::type_complexity)]
pub fn use_remote_components<T: ComponentValue + std::fmt::Debug>(
    hooks: &mut Hooks,
    arch_filter: ArchetypeFilter,
    component: Component<T>,
) -> Vec<(EntityId, T, Cb<dyn Fn(Option<T>) + Sync + Send>)> {
    let (values, set_values) = hooks.use_state(HashMap::new());
    let runtime = hooks.world.resource(runtime()).clone();

    let (client_state, _) = hooks.consume_context::<ClientState>().unwrap();
    let qs_changed = hooks.use_ref_with(|_| QueryState::new());
    let qs_despawned = hooks.use_ref_with(|_| QueryState::new());
    let values_intermediate = hooks.use_ref_with(|_| HashMap::new());
    hooks.use_frame(move |_| {
        let set_values = set_values.clone();
        let game_state = client_state.game_state.lock();
        let mut qs_changed = qs_changed.lock();
        let mut qs_despawned = qs_despawned.lock();
        let mut values = values_intermediate.lock();

        let mut changed = false;
        for (id, (value,)) in query((component.changed(),))
            .filter(&arch_filter)
            .iter(&game_state.world, Some(&mut *qs_changed))
        {
            let client_state = client_state.clone();
            let runtime = runtime.clone();
            let update: Cb<dyn Fn(Option<T>) + Sync + Send> = cb(move |value| {
                let client_state = client_state.clone();
                runtime.spawn(async move {
                    log_network_result!(
                        client_state
                            .rpc(
                                rpc_world_diff,
                                match value {
                                    Some(value) => WorldDiff::new().set(id, component, value),
                                    None => WorldDiff::new()
                                        .remove_components_raw(id, vec![component.into()]),
                                },
                            )
                            .await
                    );
                });
            });
            values.insert(id, (id, value.clone(), update));
            changed = true;
        }
        for (id, _) in query((component,))
            .despawned()
            .filter(&arch_filter)
            .iter(&game_state.world, Some(&mut *qs_despawned))
        {
            values.remove(&id);
            changed = true;
        }

        if changed {
            set_values(values.clone());
        }
    });

    values.into_values().collect()
}

#[allow(clippy::type_complexity)]
pub fn use_remote_first_component<T: ComponentValue + std::fmt::Debug>(
    hooks: &mut Hooks,
    arch_filter: ArchetypeFilter,
    entity_filter: impl 'static + Fn(&World, EntityId) -> bool + Send + Sync,
    component: Component<T>,
) -> Option<(Option<T>, Arc<dyn Fn(Option<T>) + Sync + Send>)> {
    let (value, set_value) = hooks.use_state(None);
    let (entity_id, set_entity_id) = hooks.use_state(EntityId::null());
    let f = Arc::new(entity_filter);
    let f2 = f.clone();

    use_remote_world_system(hooks, query((component.changed(),)).filter(&arch_filter), {
        let set_value = set_value.clone();
        let set_entity_id = set_entity_id.clone();
        move |q, world, qs, _| {
            if let Some((id, (value,))) = q.iter(world, qs).find(|v| f(world, v.0)) {
                set_value(Some(value.clone()));
                set_entity_id(id);
            }
        }
    });

    use_remote_world_system(
        hooks,
        query((component,)).despawned().filter(&arch_filter),
        move |q, world, qs, _| {
            if q.iter(world, qs).any(|v| f2(world, v.0)) {
                set_value(None);
                set_entity_id(EntityId::null());
            }
        },
    );

    let (client_state, _) = hooks.consume_context::<ClientState>()?;
    let runtime = hooks.world.resource(runtime()).clone();
    Some((
        value,
        Arc::new(move |value| {
            let client_state = client_state.clone();
            runtime.spawn(async move {
                log_network_result!(
                    client_state
                        .rpc(
                            rpc_world_diff,
                            match value {
                                Some(value) => WorldDiff::new().set(entity_id, component, value),
                                None => WorldDiff::new()
                                    .remove_components_raw(entity_id, vec![component.into()]),
                            },
                        )
                        .await
                );
            });
        }),
    ))
}

#[allow(clippy::type_complexity)]
/// A **persistent** component shared with all clients
pub fn use_remote_persisted_resource<T: ComponentValue + std::fmt::Debug>(
    hooks: &mut Hooks,
    component: Component<T>,
) -> Option<(Option<T>, Arc<dyn Fn(Option<T>) + Sync + Send>)> {
    use_remote_first_component(
        hooks,
        ArchetypeFilter::new().incl(persistent_resources()),
        |_, _| true,
        component,
    )
}

#[allow(clippy::type_complexity)]
/// A **non** persistent component shared with all clients
pub fn use_remote_synced_resource<T: ComponentValue + std::fmt::Debug>(
    hooks: &mut Hooks,
    component: Component<T>,
) -> Option<(Option<T>, Arc<dyn Fn(Option<T>) + Sync + Send>)> {
    use_remote_first_component(
        hooks,
        ArchetypeFilter::new().incl(synced_resources()),
        |_, _| true,
        component,
    )
}

#[allow(clippy::type_complexity)]
/// Uses a resource from the [`GameClient`] world
pub fn use_remote_resource<T: ComponentValue + std::fmt::Debug>(
    hooks: &mut Hooks,
    component: Component<T>,
) -> Option<(Option<T>, Arc<dyn Fn(Option<T>) + Sync + Send>)> {
    use_remote_first_component(
        hooks,
        ArchetypeFilter::new(),
        |w, v| v == w.resource_entity(),
        component,
    )
}

pub fn use_player_id(hooks: &mut Hooks) -> Option<EntityId> {
    let (client_state, _) = hooks.consume_context::<ClientState>().unwrap();
    let (ent, set_ent) = hooks.use_state(None);
    use_remote_world_system(
        hooks,
        query(user_id().changed()).incl(player()),
        move |q, world, qs, _| {
            for (id, pid) in q.iter(world, qs) {
                if pid == &client_state.user_id {
                    set_ent(Some(id));
                }
            }
        },
    );
    ent
}
pub fn use_remote_player_component<T: ComponentValue + Default + std::fmt::Debug + Clone>(
    hooks: &mut Hooks,
    component: Component<T>,
) -> (T, Setter<T>) {
    let player_id = use_player_id(hooks);
    let (value, set_value) = hooks.use_state(T::default());
    use_remote_world_system(
        hooks,
        query((component.changed(),)),
        move |q, world, qs, _| {
            for (id, (value,)) in q.iter(world, qs) {
                if Some(id) == player_id {
                    set_value(value.clone());
                }
            }
        },
    );

    let (client_state, _) = hooks.consume_context::<ClientState>().unwrap();
    let runtime = hooks.world.resource(runtime()).clone();
    let set_value = cb(move |new_value| {
        let client_state = client_state.clone();
        runtime.spawn(async move {
            let diff = WorldDiff::new().set(player_id.unwrap(), component, new_value);
            log_network_result!(client_state.rpc(rpc_world_diff, diff).await);
        });
    });

    (value, set_value)
}
