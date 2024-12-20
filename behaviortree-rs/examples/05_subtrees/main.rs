// Copyright © 2024 Stephan Kunz

//! This example implements the fifth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_05_subtrees
//! It is enriched with random behavior of nodes
//! - IsDoorClosed in main.rs
//! - OpenDoor in subtree.rs
//! - PickLock in subtree.rs
//!

mod subtree;

use rand::Rng;
use std::{fs::File, io::Read, path::PathBuf};

use behaviortree_rs::prelude::*;

/// ConditionNode "IsDoorClosed"
#[bt_node(SyncActionNode)]
struct IsDoorClosed {}

#[bt_node(SyncActionNode)]
impl IsDoorClosed {
    async fn tick(&mut self) -> NodeResult {
        let mut rng = rand::thread_rng();
        let state = rng.gen::<bool>();
        if state {
            println!("door is closed");

            Ok(NodeStatus::Success)
        } else {
            println!("door is open");

            Ok(NodeStatus::Failure)
        }
    }
}

/// SyncActionNode "PassThroughDoor"
#[bt_node(SyncActionNode)]
struct PassThroughDoor {}

#[bt_node(SyncActionNode)]
impl PassThroughDoor {
    async fn tick(&mut self) -> NodeResult {
        println!("door passed");

        Ok(NodeStatus::Success)
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register main tree nodes
    register_action_node!(factory, "IsDoorClosed", IsDoorClosed);
    register_action_node!(factory, "PassThroughDoor", PassThroughDoor);
    // register subtrees nodes
    subtree::register_nodes(&mut factory)?;

    // construct path to examples xml independant of current directory in project
    let mut directory = std::env::current_dir()?.to_str().unwrap().to_string();
    let pos = directory.find("behaviortree-rs").expect("wrong path");
    directory.replace_range(pos.., "behaviortree-rs");
    let path = PathBuf::from(directory)
        .join(file!())
        .parent()
        .expect("no path to file")
        .join("subtrees.xml");

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
