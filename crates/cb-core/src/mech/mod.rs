//! The mech (openspec change `mech-rig`): the rig contract, its data, its movement and its walk.

pub mod config;
pub mod damage;
pub mod gait;
pub mod locomotion;
pub mod melee;
pub mod rig;
pub mod walk;

pub use config::{BoxSpec, GaitConfig, JetConfig, LocomotionConfig, MechConfig, NodeSpec, RamConfig, SkateConfig};
pub use damage::{Damage, Hit, PartKind, PartSpec, default_parts};
pub use gait::{LegPose, LegRest, MechPose, RigRest, foot_strikes, pose, solve_leg, stance_lead_m};
pub use locomotion::{Budget, BudgetUse, LocomotionMode, MechControls, MechState, OnWater, forward};
pub use melee::{CloseCombat, MeleeConfig, Strikes, arm_rotations};
