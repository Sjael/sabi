use std::{
    collections::{btree_map::Entry, BTreeMap},
    fmt,
};

use bevy::{ecs::entity::Entities, prelude::*, utils::HashMap};
use bevy_renet::renet::{RenetClient, RenetServer};

use crate::{
    prelude::*,
    stage::{NetworkSimulationInfo, Rewind},
};
use serde::{Deserialize, Serialize};

use super::{demands::ReplicateSizeEstimates, interest::InterestsToSend, ClientId, NetworkTick};

pub const FRAME_BUFFER: u64 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMessage {
    pub tick: NetworkTick,
    pub entity_update: EntityUpdate,
}

impl UpdateMessage {
    pub fn apply(&mut self, other: Self) {
        if other.tick != self.tick {
            panic!("attempt to apply update message on different tick");
        }

        self.entity_update.apply(other.entity_update);
    }
}

#[derive(Default, Debug, Clone)]
pub struct ClientEntityUpdates {
    clients: BTreeMap<ClientId, EntityUpdate>,
}

impl ClientEntityUpdates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ClientId, &EntityUpdate)> {
        self.clients.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ClientId, &mut EntityUpdate)> {
        self.clients.iter_mut()
    }

    pub fn get(&self, client_id: &ClientId) -> Option<&EntityUpdate> {
        self.clients.get(client_id)
    }

    pub fn get_mut(&mut self, client_id: &ClientId) -> Option<&mut EntityUpdate> {
        self.clients.get_mut(client_id)
    }

    pub fn upsert(&mut self, client_id: ClientId) -> &mut EntityUpdate {
        self.clients.entry(client_id).or_default()
    }
}

#[derive(Deref, DerefMut, Default, Clone, Serialize, Deserialize)]
pub struct EntityUpdate {
    pub updates: BTreeMap<ServerEntity, ComponentsUpdate>,
}

impl fmt::Debug for EntityUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut counts: BTreeMap<ReplicateId, u16> = Default::default();

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
        Self::default()
    }

    pub fn clear(&mut self) {
        self.updates.clear();
    }

    pub fn apply(&mut self, other: Self) {
        for (entity, components) in other.updates {
            match self.entry(entity) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().apply(components);
                }
                Entry::Vacant(entry) => {
                    entry.insert(components);
                }
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ServerEntity, &ComponentsUpdate)> {
        self.updates.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ServerEntity, &mut ComponentsUpdate)> {
        self.updates.iter_mut()
    }
}

#[derive(Default, Deref, DerefMut, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentsUpdate(pub BTreeMap<ReplicateId, Vec<u8>>);

impl ComponentsUpdate {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn apply(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

impl EntityUpdate {
    pub fn protocol_id() -> u64 {
        1
    }
}

#[derive(Debug, Clone)]
pub struct UpdateMessages {
    messages: BTreeMap<NetworkTick, UpdateMessage>,
}

impl UpdateMessages {
    pub fn new() -> Self {
        Self {
            messages: Default::default(),
        }
    }

    pub fn get(&self, tick: &NetworkTick) -> Option<&UpdateMessage> {
        self.messages.get(tick)
    }

    pub fn latest(&self) -> Option<&NetworkTick> {
        self.messages.keys().max()
    }

    pub fn push(&mut self, message: UpdateMessage) {
        match self.messages.entry(message.tick) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().apply(message);
            }
            Entry::Vacant(entry) => {
                entry.insert(message);
            }
        }
    }

    /// Retain any in the queue that are within a buffer range.
    pub fn retain(&mut self) {
        let newest = self.latest().cloned().unwrap_or_default();

        self.messages.retain(|tick, _| {
            (newest.tick() as i64) - (tick.tick() as i64)
                < crate::protocol::resim::SNAPSHOT_RETAIN_BUFFER
        });
    }
}

pub fn client_recv_interest(
    mut commands: Commands,
    mut network_sim_info: ResMut<NetworkSimulationInfo>,
    mut tick: ResMut<NetworkTick>,
    mut server_updates: ResMut<UpdateMessages>,
    mut server_entities: ResMut<ServerEntities>,
    mut client: ResMut<RenetClient>,
) {
    let mut rewind: Option<NetworkTick> = None;

    while let Some(message) = client.receive_message(channel::COMPONENT) {
        let dict = crate::message_sample::DICTIONARIES
            .get("update")
            .expect("no update dictionary");
        let mut decompressor =
            zstd::bulk::Decompressor::with_dictionary(dict).expect("couldn't make decompressor");
        /*
        let mut decompressor = zstd::bulk::Decompressor::new().expect("couldn't make decompressor");
        */

        let decompressed = decompressor
            .decompress(&message.as_slice(), 10 * 1024)
            .expect("could not decompress message");

        let message: UpdateMessage = bincode::deserialize(&decompressed).unwrap();

        let diff = tick.tick() as i64 - message.tick.tick() as i64;
        if diff < 0 {
            error!("falling behind server, hard stepping tick");
            *tick = NetworkTick::new(message.tick.tick() + FRAME_BUFFER);
        }

        if diff > FRAME_BUFFER as i64 {
            network_sim_info.accel(0.01);
        } else if diff < FRAME_BUFFER as i64 {
            network_sim_info.decel(0.01);
        } else if diff == FRAME_BUFFER as i64 {
            network_sim_info.accel(0.0);
        }

        match rewind {
            Some(ref mut rewind) if message.tick.tick() < rewind.tick() => {
                *rewind = message.tick;
            }
            None => {
                rewind = Some(message.tick);
            }
            _ => {}
        }

        for (server_entity, _) in message.entity_update.iter() {
            server_entities.spawn_or_get(&mut commands, *server_entity);
        }

        server_updates.push(message);
    }

    if let Some(rewind) = rewind {
        commands.insert_resource(Rewind(rewind));
    }
}

pub fn client_apply_server_update(
    tick: Res<NetworkTick>,
    server_updates: Res<UpdateMessages>,
    mut update_events: EventWriter<(ServerEntity, ComponentsUpdate)>,
) {
    if let Some(update) = server_updates.get(&*tick) {
        update_events.send_batch(update.entity_update.updates.clone().into_iter());
    }
}

pub fn client_update<C>(
    mut commands: Commands,
    entities: &Entities,
    server_entities: Res<ServerEntities>,
    mut update_events: EventReader<(ServerEntity, ComponentsUpdate)>,
    mut query: Query<&mut C>,
) where
    C: 'static + Send + Sync + Component + Replicate + Clone,
{
    for (server_entity, components_update) in update_events.iter() {
        if let Some(update_data) = components_update.get(&C::replicate_id()) {
            let def: <C as Replicate>::Def = bincode::deserialize(&update_data).unwrap();
            if let Some(entity) = server_entities.get(entities, *server_entity) {
                if let Ok(mut component) = query.get_mut(entity) {
                    let current_def = component.clone().into_def();
                    if current_def != def {
                        component.apply_def(def);
                    }
                } else {
                    let component = C::from_def(def);
                    commands.entity(entity).insert(component);
                }
            } else {
                error!("server entity was not spawned before sending component event");
            }
        }
    }
}

pub fn server_clear_queue(mut updates: ResMut<EntityUpdate>) {
    updates.clear();
}

pub fn server_queue_interest<C>(
    mut estimate: ResMut<ReplicateSizeEstimates>,
    mut updates: ResMut<ClientEntityUpdates>,
    to_send: Res<InterestsToSend>,
    query: Query<&C>,
) where
    C: 'static + Send + Sync + Component + Replicate + Clone,
{
    for (client_id, interests) in to_send.iter() {
        let entity_update = updates.upsert(*client_id);
        for (entity, replicate_id) in interests.iter() {
            if *replicate_id == C::replicate_id() {
                if let Ok(component) = query.get(*entity) {
                    let server_entity = ServerEntity::from_entity(*entity);
                    let component_def = component.clone().into_def();
                    let component_data = bincode::serialize(&component_def).unwrap();

                    estimate.add(C::replicate_id(), component_data.len());

                    let update = entity_update
                        .entry(server_entity)
                        .or_insert(ComponentsUpdate::new());
                    update.insert(C::replicate_id(), component_data);
                }
            }
        }
    }
}

pub fn server_send_interest(
    tick: Res<NetworkTick>,
    updates: Res<ClientEntityUpdates>,
    mut server: ResMut<RenetServer>,
) {
    let dict = crate::message_sample::DICTIONARIES
        .get("update")
        .expect("no update dictionary");
    let mut compressor =
        zstd::bulk::Compressor::with_dictionary(0, dict).expect("couldn't make compressor");

    for (client_id, update) in updates.iter() {
        let message = UpdateMessage {
            tick: *tick,
            entity_update: update.clone(),
        };
        let serialized = bincode::serialize(&message).unwrap();
        /*
        crate::message_sample::try_add_sample("update", &serialized);
        */
        /*
        let mut compressor = zstd::bulk::Compressor::new(0).expect("couldn't make compressor");
            */
        let compressed = compressor
            .compress(&serialized.as_slice())
            .expect("couldn't compress message");

        server.send_message(*client_id, channel::COMPONENT, compressed)
    }
}
