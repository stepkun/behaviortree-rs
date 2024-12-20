// Copyright © 2024 Stephan Kunz

//! This example implements the second tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_02_basic_ports
//!

use std::{fs::File, io::Read, path::PathBuf};

use behaviortree_rs::prelude::*;

/// SyncActionNode "SaySomething"
#[bt_node(SyncActionNode)]
struct SaySomething {}

#[bt_node(SyncActionNode)]
impl SaySomething {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("message")?;

        println!("Robot says: {msg}");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("message", "hello"))
    }
}

/// SyncActionNode "ThinkWhatToSay"
#[bt_node(SyncActionNode)]
struct ThinkWhatToSay {}

#[bt_node(SyncActionNode)]
impl ThinkWhatToSay {
    async fn tick(&mut self) -> NodeResult {
        node_.config.set_output("text", "The answer is 42.").await?;

        println!("Robot has thought");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(output_port!("text"))
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register all needed nodes
    register_action_node!(factory, "SaySomething", SaySomething);
    register_action_node!(factory, "ThinkWhatToSay", ThinkWhatToSay);

    // construct path to examples xml independant of current directory in project
    let mut directory = std::env::current_dir()?.to_str().unwrap().to_string();
    let pos = directory.find("behaviortree-rs").expect("wrong path");
    directory.replace_range(pos.., "behaviortree-rs");
    let path = PathBuf::from(directory)
        .join(file!())
        .parent()
        .expect("no path to file")
        .join("ports.xml");

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
