// Copyright © 2024 Stephan Kunz

//! Implementation of the subtree
//!

use behaviortree_rs::{nodes::decorator::RetryNode, prelude::*};
use rand::Rng;

/// SyncActionNode "OpenDoor"
#[bt_node(SyncActionNode)]
struct OpenDoor {}

#[bt_node(SyncActionNode)]
impl OpenDoor {
    async fn tick(&mut self) -> NodeResult {
        let mut rng = rand::thread_rng();
        let state = rng.gen::<bool>();
        if state {
            println!("opened door");

            Ok(NodeStatus::Success)
        } else {
            println!("could not open door");

            Ok(NodeStatus::Failure)
        }
    }
}

/// SyncActionNode "PickLock"
#[bt_node(SyncActionNode)]
struct PickLock {}

#[bt_node(SyncActionNode)]
impl PickLock {
    async fn tick(&mut self) -> NodeResult {
        let mut rng = rand::thread_rng();
        let state = rng.gen::<i32>();
        if state % 5 == 0 {
            println!("picked lock");

            Ok(NodeStatus::Success)
        } else {
            println!("could not pick lock");

            Ok(NodeStatus::Failure)
        }
    }
}

/// SyncActionNode "SmashDoor"
#[bt_node(SyncActionNode)]
struct SmashDoor {}

#[bt_node(SyncActionNode)]
impl SmashDoor {
    async fn tick(&mut self) -> NodeResult {
        println!("smashed door");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        PortsList::new()
    }
}

pub fn register_nodes(factory: &mut Factory) -> anyhow::Result<()> {
    register_action_node!(factory, "OpenDoor", OpenDoor);
    register_decorator_node!(factory, "RetryUntilSuccessful", RetryNode);
    register_action_node!(factory, "PickLock", PickLock);
    register_action_node!(factory, "SmashDoor", SmashDoor);

    Ok(())
}
