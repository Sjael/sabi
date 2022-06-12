use std::fmt;

use bevy::{prelude::*, utils::HashMap};
use bevy_renet::renet::RenetClient;

use crate::prelude::*;
use serde::{Deserialize, Serialize};

use super::priority::{ComponentsToSend, PriorityAccumulator, ReplicateSizeEstimates};

#[derive(Deref, DerefMut, Clone, Serialize, Deserialize)]
pub struct ClientEntityUpdate(pub HashMap<u64, EntityUpdate>);

#[derive(Deref, DerefMut, Clone, Serialize, Deserialize)]
pub struct EntityUpdate {
    pub updates: HashMap<ServerEntity, ComponentsUpdate>,
}

impl fmt::Debug for EntityUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut counts: HashMap<ReplicateId, u16> = HashMap::new();

        for (_, component_update) in self.iter() {
            for (replicate_id, _) in component_update.iter() {
                *counts.entry(*replicate_id).or_insert(0) += 1;
            }
        }

        f.debug_struct("EntityUpdate")
            .field("entities", &self.updates.len())
            .field("components", &counts)
            .finish()
    }
}

impl EntityUpdate {
    pub fn new() -> Self {
        Self {
            updates: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.updates.clear();
    }
}

#[derive(Default, Deref, DerefMut, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentsUpdate(pub HashMap<ReplicateId, Vec<u8>>);

impl ComponentsUpdate {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl EntityUpdate {
    pub fn protocol_id() -> u64 {
        1
    }
}

pub fn client_recv_interest_reliable(
    mut commands: Commands,
    mut server_entities: ResMut<ServerEntities>,
    mut update_events: EventWriter<(ServerEntity, ComponentsUpdate)>,
    mut client: ResMut<RenetClient>,
) {
    while let Some(message) = client.receive_message(channel::COMPONENT) {
        let decompressed = zstd::bulk::decompress(&message.as_slice(), 10 * 1024).unwrap();
        let data: EntityUpdate = bincode::deserialize(&decompressed).unwrap();

        for (server_entity, _) in data.iter() {
            server_entities.spawn_or_get(&mut commands, *server_entity);
        }

        update_events.send_batch(data.updates.into_iter());
    }
}

pub fn client_update_reliable<C>(
    mut commands: Commands,
    mut server_entities: ResMut<ServerEntities>,
    mut update_events: EventReader<(ServerEntity, ComponentsUpdate)>,
    mut query: Query<&mut C>,
) where
    C: 'static + Send + Sync + Component + Replicate,
{
    for (server_entity, components_update) in update_events.iter() {
        if let Some(update_data) = components_update.get(&C::replicate_id()) {
            let def: <C as Replicate>::Def = bincode::deserialize(&update_data).unwrap();
            let entity = server_entities.spawn_or_get(&mut commands, *server_entity);

            if let Ok(mut component) = query.get_mut(entity) {
                component.apply_def(def);
            } else {
                let component = C::from_def(def);
                commands.entity(entity).insert(component);
            }
        }
    }
}

pub fn server_clear_queue(mut updates: ResMut<EntityUpdate>) {
    updates.clear();
}

pub fn server_queue_interest<C>(
    mut priority: ResMut<PriorityAccumulator>,
    mut estimate: ResMut<ReplicateSizeEstimates>,
    mut updates: ResMut<EntityUpdate>,
    to_send: Res<ComponentsToSend>,
    query: Query<&C>,
) where
    C: 'static + Send + Sync + Component + Replicate + Clone,
{
    for (entity, replicate_id) in to_send.iter() {
        if *replicate_id == C::replicate_id() {
            if let Ok(component) = query.get(*entity) {
                let server_entity = ServerEntity::from_entity(*entity);
                let component_def = component.clone().into_def();
                let component_data = bincode::serialize(&component_def).unwrap();

                estimate.add(C::replicate_id(), component_data.len());

                let update = updates
                    .entry(server_entity)
                    .or_insert(ComponentsUpdate::new());
                update.insert(C::replicate_id(), component_data);

                priority.clear(*entity, C::replicate_id());
            }
        }
    }
}
