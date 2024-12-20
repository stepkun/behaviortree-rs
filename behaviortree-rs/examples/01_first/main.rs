// Copyright © 2024 Stephan Kunz

//! This example implements the first tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_01_first_tree
//!
//! Differences to BehaviorTree.CPP:
//! - we cannot register functions/methods of a struct/class
//! - there is no separate ConditionNode type, these have to be implemented as SyncActionNode
//!

use std::{fs::File, io::Read, path::PathBuf};

use behaviortree_rs::prelude::*;

/// ConditionNode "CheckBattery"
#[bt_node(SyncActionNode)]
struct CheckBattery {}

#[bt_node(SyncActionNode)]
impl CheckBattery {
    async fn tick(&mut self) -> NodeResult {
        println!("battery state is ok");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("name"))
    }
}

/// SyncActionNode "OpenGripper"
#[bt_node(SyncActionNode)]
struct OpenGripper {}

#[bt_node(SyncActionNode)]
impl OpenGripper {
    async fn tick(&mut self) -> NodeResult {
        println!("opened gripper");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("name"))
    }
}

/// SyncActionNode "ApproachObject"
#[bt_node(SyncActionNode)]
struct ApproachObject {}

#[bt_node(SyncActionNode)]
impl ApproachObject {
    async fn tick(&mut self) -> NodeResult {
        println!("approaching object");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("name"))
    }
}

/// SyncActionNode "CloseGripper"
#[bt_node(SyncActionNode)]
struct CloseGripper {}

#[bt_node(SyncActionNode)]
impl CloseGripper {
    async fn tick(&mut self) -> NodeResult {
        println!("closed gripper");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("name"))
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register all needed nodes
    register_action_node!(factory, "CheckBattery", CheckBattery);
    register_action_node!(factory, "OpenGripper", OpenGripper);
    register_action_node!(factory, "ApproachObject", ApproachObject);
    register_action_node!(factory, "CloseGripper", CloseGripper);

    // construct path to examples xml independant of current directory in project
    let mut directory = std::env::current_dir()?.to_str().unwrap().to_string();
    let pos = directory.find("behaviortree-rs").expect("wrong path");
    directory.replace_range(pos.., "behaviortree-rs");
    let path = PathBuf::from(directory)
        .join(file!())
        .parent()
        .expect("no path to file")
        .join("first.xml");

    // read xml from file
    let mut file = File::open(path)?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)?;

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(xml, &blackboard)?;

    // run the BT
    let result = tree.tick_while_running()?;
    println!("tree result is {result}");

    Ok(())
}
