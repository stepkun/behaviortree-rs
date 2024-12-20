// Copyright © 2024 Stephan Kunz

//! This example implements the third tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_03_generic_ports
//!
//! Differences to BehaviorTree.CPP
//! - there is no Script node available, that has to be implemented by user
//!

use std::{num::ParseFloatError, str::FromStr};

use behaviortree_rs::{basic_types::FromString, nodes::NodeError, prelude::*};
use behaviortree_rs_derive::FromString;

const XML: &str = r#"
<root BTCPP_format="4" >
    <BehaviorTree ID="MainTree">
       <Sequence>
           <CalculateGoal goal="{GoalPosition}" />
           <PrintTarget   target="{GoalPosition}" />
           <Script        code=" OtherGoal:=&apos;-1;3&apos; " />
           <PrintTarget   target="{OtherGoal}" />
       </Sequence>
    </BehaviorTree>
</root>
"#;

#[derive(Clone, Copy, Debug, FromString)]
struct Position2D {
    x: f64,
    y: f64,
}

impl FromStr for Position2D {
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
        Ok(Self { x, y })
    }
}

/// SyncActionNode "CalculateGoal"
#[bt_node(SyncActionNode)]
struct CalculateGoal {}

#[bt_node(SyncActionNode)]
impl CalculateGoal {
    async fn tick(&mut self) -> NodeResult {
        // initialize GoalPosition
        let pos = Position2D { x: 1.1, y: 2.3 };
        node_.config.set_output("goal", pos).await?;

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(output_port!("goal"))
    }
}

/// SyncActionNode "PrintTarget"
#[bt_node(SyncActionNode)]
struct PrintTarget {}

#[bt_node(SyncActionNode)]
impl PrintTarget {
    async fn tick(&mut self) -> NodeResult {
        let pos: Position2D = node_.config.get_input("target")?;

        println!("Target positions: [ {}, {} ]", pos.x, pos.y);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("target"))
    }
}

/// SyncActionNode "Script"
#[bt_node(SyncActionNode)]
struct Script {}

#[bt_node(SyncActionNode)]
impl Script {
    async fn tick(&mut self) -> NodeResult {
        let script: String = node_.config.get_input("code")?;
        let elements: Vec<&str> = script.split(":=").collect();
        let pos = Position2D::from_string(elements[1].trim()).map_err(|_| {
            NodeError::PortValueParseError("code".to_string(), "Position2D".to_string())
        })?;
        node_
            .config
            .blackboard()
            .to_owned()
            .set(elements[0].trim(), pos);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("code"))
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register all needed nodes
    register_action_node!(factory, "CalculateGoal", CalculateGoal);
    register_action_node!(factory, "PrintTarget", PrintTarget);
    register_action_node!(factory, "Script", Script);

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(XML.to_string(), &blackboard)?;

    // run the BT
    let result = tree.tick_while_running()?;
    println!("tree result is {result}");

    Ok(())
}
