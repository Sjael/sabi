use bevy::{math::Affine3A, prelude::*};

use crate::prelude::Replicate;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "Transform")]
#[replicate(remote = "Transform")]
#[replicate(crate = "crate")]
pub struct TransformDef {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Replicate)]
#[serde(remote = "GlobalTransform")]
#[replicate(remote = "GlobalTransform")]
#[replicate(crate = "crate")]
pub struct GlobalTransformDef(#[serde(getter = "GlobalTransform::affine")] Affine3A);

impl From<GlobalTransformDef> for GlobalTransform {
    fn from(def: GlobalTransformDef) -> GlobalTransform {
        GlobalTransform::from(def.0)
    }
}

impl Replicate for Name {
    type Def = String;
    fn into_def(self) -> Self::Def {
        self.as_str().to_owned()
    }
    fn apply_def(&mut self, def: Self::Def) {
        self.set(def);
    }
    fn from_def(def: Self::Def) -> Self {
        Name::new(def)
    }
}
