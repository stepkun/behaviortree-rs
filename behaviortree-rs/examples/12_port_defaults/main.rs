// Copyright © 2024 Stephan Kunz

//! This example implements the twelvth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-advanced/tutorial_12_default_ports
//!
//! Differences to BehaviorTree.CPP
//! - It is not possible to add an action node directly below the root node
//! - only 3 of the 6 ways in BehaviorTree.CPP are working in an easy manner
//!

use std::{
    fmt::{Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

use behaviortree_rs::{basic_types::FromString, nodes::NodeError, prelude::*};
use behaviortree_rs_derive::{BTToString, FromString};

const XML: &str = r#"
<root BTCPP_format="4">
    <BehaviorTree ID="MainTree">
        <Sequence>
            <NodeWithDefaultPoints input="-1,-2"/>
        </Sequence>
    </BehaviorTree>
</root>"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BTToString, FromString)]
struct Point2D {
    x: i32,
    y: i32,
}

impl Display for Point2D {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

impl FromStr for Point2D {
    type Err = ParseIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // remove redundant ' and &apos; from string
        let s = value
            .replace("'", "")
            .trim()
            .replace("&apos;", "")
            .trim()
            .to_string();
        let v: Vec<&str> = s.split(',').collect();
        let x = i32::from_string(v[0])?;
        let y = i32::from_string(v[1])?;
        Ok(Self { x, y })
    }
}

/// SyncActionNode "CalculateGoal"
#[bt_node(SyncActionNode)]
struct NodeWithDefaultPoints {}

#[bt_node(SyncActionNode)]
impl NodeWithDefaultPoints {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("input")?;
        let point = Point2D::from_string(&msg)
            .map_err(|_| NodeError::PortValueParseError("input".into(), msg))?;
        println!("input:  [{},{}]", point.x, point.y);

        let msg: String = node_.config.get_input("pointA")?;
        let point = Point2D::from_string(&msg)
            .map_err(|_| NodeError::PortValueParseError("pointA".into(), msg))?;
        println!("pointA:  [{},{}]", point.x, point.y);

        //let msg: String = node_.config.get_input("pointB")?;
        //let point = Point2D::from_string(&msg).map_err(|_| NodeError::PortValueParseError("pointB".into(), msg))?;
        //println!("pointB:  [{},{}]", point.x, point.y);

        let msg: String = node_.config.get_input("pointC")?;
        let point = Point2D::from_string(&msg)
            .map_err(|_| NodeError::PortValueParseError("pointC".into(), msg))?;
        println!("pointC:  [{},{}]", point.x, point.y);

        //let msg: String = node_.config.get_input("pointD")?;
        //let point = Point2D::from_string(&msg).map_err(|_| NodeError::PortValueParseError("pointD".into(), msg))?;
        //println!("pointD:  [{},{}]", point.x, point.y);

        //let msg: String = node_.config.get_input("pointE")?;
        //let point = Point2D::from_string(&msg).map_err(|_| NodeError::PortValueParseError("pointE".into(), msg))?;
        //println!("pointE:  [{},{}]", point.x, point.y);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        let json = r#"(json:{"x':9,"y":10})"#;
        define_ports!(
            input_port!("input"),                          // no default value
            input_port!("pointA", Point2D { x: 1, y: 2 }), // default value is [1,2]
            input_port!("pointB", "{point}"), // default value inside blackboard {point}
            input_port!("pointC", "5,6"),     // default value is [5,6],
            input_port!("pointD", "{=}"),     // default value inside blackboard {pointD}
            input_port!("pointE", json)       // default value inside blackboard {pointD}
                                              //input_port!("pointE, r#"(json:{"x':9,"y":10})"#) // default value is [9,10]
        )
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register all needed nodes
    register_action_node!(factory, "NodeWithDefaultPoints", NodeWithDefaultPoints);

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(XML.to_string(), &blackboard)?;

    // initialize blackboard values
    tree.root_blackboard().set("point", Point2D { x: 3, y: 4 });
    tree.root_blackboard().set("pointD", Point2D { x: 7, y: 8 });

    // run the BT
    let result = tree.tick_while_running()?;
    println!("tree result is {result}");

    Ok(())
}
