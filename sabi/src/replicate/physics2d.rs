use bevy::prelude::*;
use bevy_rapier2d::{prelude::*, rapier::prelude::SharedShape};

use serde::{Deserialize, Serialize};

use crate::{plugin::ReplicatePlugin, protocol::demands::RequireDependency, Replicate};

pub struct ReplicatePhysics2dPlugin;
impl Plugin for ReplicatePhysics2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ReplicatePlugin::<RigidBody>::default());
        app.add_plugin(ReplicatePlugin::<Velocity>::default());
        app.add_plugin(ReplicatePlugin::<LockedAxes>::default());
        app.add_plugin(ReplicatePlugin::<ExternalForce>::default());
        app.add_plugin(ReplicatePlugin::<ExternalImpulse>::default());
        app.add_plugin(ReplicatePlugin::<Ccd>::default());
        app.add_plugin(ReplicatePlugin::<Sleeping>::default());
        app.add_plugin(ReplicatePlugin::<Dominance>::default());
        app.add_plugin(ReplicatePlugin::<Damping>::default());
        app.add_plugin(ReplicatePlugin::<Restitution>::default());
        app.add_plugin(ReplicatePlugin::<Friction>::default());
        app.add_plugin(ReplicatePlugin::<GravityScale>::default());
        app.add_plugin(ReplicatePlugin::<Sensor>::default());
        app.add_plugin(ReplicatePlugin::<CollisionGroups>::default());
        app.add_plugin(ReplicatePlugin::<SolverGroups>::default());
        app.add_plugin(ReplicatePlugin::<Collider>::default());
        app.add_plugin(ReplicatePlugin::<ColliderScale>::default());

        app.add_plugin(ReplicatePlugin::<AdditionalMassProperties>::default());
        app.add_plugin(ReplicatePlugin::<ColliderMassProperties>::default());

        app.add_plugin(RequireDependency::<Collider, RigidBody>::default());
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "RigidBody")]
#[replicate(remote = "RigidBody")]
#[replicate(crate = "crate")]
pub enum RigidBodyDef {
    Dynamic,
    Fixed,
    KinematicVelocityBased,
    KinematicPositionBased,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "Velocity")]
#[replicate(remote = "Velocity")]
#[replicate(crate = "crate")]
pub struct VelocityDef {
    pub linvel: Vec2,
    pub angvel: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "MassProperties")]
pub struct MassPropertiesDef {
    pub local_center_of_mass: Vec2,
    pub mass: f32,
    pub principal_inertia: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "AdditionalMassProperties")]
#[replicate(remote = "AdditionalMassProperties")]
#[replicate(crate = "crate")]
pub enum AdditionalMassPropertiesDef {
    Mass(f32),
    MassProperties(#[serde(with = "MassPropertiesDef")] MassProperties),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "ColliderMassProperties")]
#[replicate(remote = "ColliderMassProperties")]
#[replicate(crate = "crate")]
pub enum ColliderMassPropertiesDef {
    Density(f32),
    Mass(f32),
    MassProperties(#[serde(with = "MassPropertiesDef")] MassProperties),
}

impl Replicate for LockedAxes {
    type Def = u8;
    fn into_def(self) -> Self::Def {
        self.bits()
    }
    fn from_def(def: Self::Def) -> Self {
        LockedAxes::from_bits(def).expect("locked axes from bits")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "ExternalForce")]
#[replicate(remote = "ExternalForce")]
#[replicate(crate = "crate")]
pub struct ExternalForceDef {
    pub force: Vec2,
    pub torque: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "ExternalImpulse")]
#[replicate(remote = "ExternalImpulse")]
#[replicate(crate = "crate")]
pub struct ExternalImpulseDef {
    pub impulse: Vec2,
    pub torque_impulse: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "Sleeping")]
#[replicate(remote = "Sleeping")]
#[replicate(crate = "crate")]
pub struct SleepingDef {
    pub linear_threshold: f32,
    pub angular_threshold: f32,
    pub sleeping: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "Damping")]
#[replicate(remote = "Damping")]
#[replicate(crate = "crate")]
pub struct DampingDef {
    pub linear_damping: f32,
    pub angular_damping: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "CoefficientCombineRule")]
pub enum CoefficientCombineRuleDef {
    Average,
    Min,
    Multiply,
    Max,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "Friction")]
#[replicate(remote = "Friction")]
#[replicate(crate = "crate")]
pub struct FrictionDef {
    pub coefficient: f32,

    #[serde(with = "CoefficientCombineRuleDef")]
    pub combine_rule: CoefficientCombineRule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "Restitution")]
#[replicate(remote = "Restitution")]
#[replicate(crate = "crate")]
pub struct RestitutionDef {
    pub coefficient: f32,

    #[serde(with = "CoefficientCombineRuleDef")]
    pub combine_rule: CoefficientCombineRule,
}

impl Replicate for Ccd {
    type Def = bool;
    fn into_def(self) -> Self::Def {
        self.enabled
    }
    fn from_def(def: Self::Def) -> Self {
        Ccd { enabled: def }
    }
}

impl Replicate for Sensor {
    type Def = ();
    fn into_def(self) -> Self::Def {
        ()
    }
    fn from_def(_def: Self::Def) -> Self {
        Sensor
    }
}

impl Replicate for GravityScale {
    type Def = f32;
    fn into_def(self) -> Self::Def {
        self.0
    }
    fn from_def(def: Self::Def) -> Self {
        GravityScale(def)
    }
}

impl Replicate for Dominance {
    type Def = i8;
    fn into_def(self) -> Self::Def {
        self.groups
    }
    fn from_def(def: Self::Def) -> Self {
        Dominance { groups: def }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "CollisionGroups")]
#[replicate(remote = "CollisionGroups")]
#[replicate(crate = "crate")]
pub struct CollisionGroupsDef {
    pub memberships: u32,
    pub filters: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "SolverGroups")]
#[replicate(remote = "SolverGroups")]
#[replicate(crate = "crate")]
pub struct SolverGroupsDef {
    pub memberships: u32,
    pub filters: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SharedShapeEq(SharedShape);

impl PartialEq for SharedShapeEq {
    fn eq(&self, other: &Self) -> bool {
        self.0.shape_type() == other.0.shape_type()
    }
}

impl Replicate for Collider {
    type Def = SharedShapeEq;
    fn into_def(self) -> Self::Def {
        SharedShapeEq(self.raw)
    }
    fn from_def(shared_shape: Self::Def) -> Self {
        Collider::from(shared_shape.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "ColliderScale")]
#[replicate(remote = "ColliderScale")]
#[replicate(crate = "crate")]
pub enum ColliderScaleDef {
    Relative(Vec2),
    Absolute(Vec2),
}
