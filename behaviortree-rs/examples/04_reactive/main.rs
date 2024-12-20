// Copyright © 2024 Stephan Kunz

//! This example implements the fourth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_04_sequence
//!
//! Differences to BehaviorTree.CPP
//! - there is no tree::sleep(...) available, using sleep of async runtime instead,
//!   which is not interrupted,when tree state changes
//!

extern crate tokio;

use std::{num::ParseFloatError, str::FromStr, time::Duration};

use behaviortree_rs::{basic_types::FromString, prelude::*};
use behaviortree_rs_derive::FromString;

const XML: &str = r#"
<root BTCPP_format="4">
    <BehaviorTree ID="MainTree">
    <Sequence>
        <BatteryOK/>
        <SaySomething   message="mission started..." />
        <MoveBase          goal="1;2;3"/>
        <SaySomething   message="mission completed!" />
    </Sequence>
    </BehaviorTree>
</root>
"#;

#[derive(Clone, Copy, Debug, FromString)]
struct Pose2D {
    x: f64,
    y: f64,
    theta: f64,
}

impl FromStr for Pose2D {
    type Err = ParseFloatError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // remove redundant ' and &apos; from string
        let s = value
            .replace("'", "")
            .trim()
            .replace("&apos;", "")
            .trim()
            .to_string();
        let v: Vec<&str> = s.split(';').collect();
        let x = f64::from_string(v[0])?;
        let y = f64::from_string(v[1])?;
        let theta = f64::from_string(v[2])?;
        Ok(Self { x, y, theta })
    }
}

/// ConditionNode "BatteryOK"
#[bt_node(SyncActionNode)]
struct BatteryOK {}

#[bt_node(SyncActionNode)]
impl BatteryOK {
    async fn tick(&mut self) -> NodeResult {
        println!("battery is ok");

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        PortsList::new()
    }
}

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

/// SyncActionNode "MoveBase"
#[bt_node(StatefulActionNode)]
struct MoveBase {
    #[bt(default)]
    counter: usize,
}

#[bt_node(StatefulActionNode)]
impl MoveBase {
    async fn on_start(&mut self) -> NodeResult {
        let pos = node_.config.get_input::<Pose2D>("goal")?;

        println!(
            "[ MoveBase: SEND REQUEST ]. goal: x={:2.1} y={:2.1} theta={:2.1}",
            pos.x, pos.y, pos.theta
        );

        Ok(NodeStatus::Running)
    }

    async fn on_running(&mut self) -> NodeResult {
        if self.counter < 5 {
            self.counter += 1;
            println!("--- status: RUNNING");
            Ok(NodeStatus::Running)
        } else {
            println!("[ MoveBase: FINISHED ]");
            Ok(NodeStatus::Success)
        }
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("goal"))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register all needed nodes
    register_action_node!(factory, "BatteryOK", BatteryOK);
    register_action_node!(factory, "SaySomething", SaySomething);
    register_action_node!(factory, "MoveBase", MoveBase);

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(XML.to_string(), &blackboard)?;

    // run the BT using own loop with sleep to avoid busy loop
    println!("--- ticking");
    let mut result = tree.tick_once()?;
    while result == NodeStatus::Running {
        let _ = tokio::time::sleep(Duration::from_millis(100)).await;
        println!("--- ticking");
        result = tree.tick_once()?;
    }

    println!("tree result is {result}");

    Ok(())
}
