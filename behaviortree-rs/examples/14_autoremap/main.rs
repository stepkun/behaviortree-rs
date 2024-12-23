// Copyright © 2024 Stephan Kunz

//! This example implements the fourteenth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-advanced/tutorial_14_subtree_model
//!
//! Differences to BehaviorTree.CPP
//! - there is no Script node available, that has to be implemented by user
//! - example in BehaviorTree.CPP is inconsistent
//! - not sure wether this example really shows how to do it
//!

mod move_robot;

use std::{fs::File, io::Read, path::PathBuf};

use behaviortree_rs::{basic_types::FromString, nodes::NodeError, prelude::*};

/// SyncActionNode "Script"
#[bt_node(SyncActionNode)]
struct Script {}

#[bt_node(SyncActionNode)]
impl Script {
    async fn tick(&mut self) -> NodeResult {
        let script: String = node_.config.get_input("code")?;
        let elements: Vec<&str> = script.split(":=").collect();
        if elements[1].contains("{") {
            let pos = move_robot::Position2D::from_string(elements[1].trim()).map_err(|_| {
                NodeError::PortValueParseError("code".to_string(), "Position2D".to_string())
            })?;
            node_
                .config
                .blackboard()
                .to_owned()
                .set(elements[0].trim(), pos);
        } else {
            let mut content = elements[1].to_string();
            // remove redundant ' from string
            content = content.replace("'", "").trim().to_string();
            // remove redundant &apos; from string
            content = content.replace("&apos;", "").trim().to_string();
            node_
                .config
                .blackboard()
                .to_owned()
                .set(elements[0].trim(), content);
        }

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("code"))
    }
}

/// SyncActionNode "SaySomething"
#[bt_node(SyncActionNode)]
struct SaySomething {}

#[bt_node(SyncActionNode)]
impl SaySomething {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("msg")?;

        println!("Robot says: {msg}");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("msg", "hello"))
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register main tree nodes
    register_action_node!(factory, "Script", Script);
    register_action_node!(factory, "SaySomething", SaySomething);
    // register subtrees nodes
    move_robot::register_nodes(&mut factory)?;

    // construct path to examples xml independant of current directory in project
    let mut directory = std::env::current_dir()?.to_str().unwrap().to_string();
    let pos = directory.find("behaviortree-rs").expect("wrong path");
    directory.replace_range(pos.., "behaviortree-rs");
    let path = PathBuf::from(directory)
        .join(file!())
        .parent()
        .expect("no path to file")
        .join("autoremap.xml");

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
