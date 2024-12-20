// Copyright © 2024 Stephan Kunz

//! Implementation of MoveRobot tree
//!

use std::{num::ParseFloatError, str::FromStr};

use behaviortree_rs::{basic_types::FromString, prelude::*};
use behaviortree_rs_derive::FromString;

#[derive(Clone, Copy, Debug, FromString)]
pub struct Position2D {
    x: f64,
    y: f64,
    theta: f64,
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
        let theta = f64::from_string(v[2])?;
        Ok(Self { x, y, theta })
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
        let pos = node_.config.get_input::<Position2D>("goal")?;

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

pub fn register_nodes(factory: &mut Factory) -> anyhow::Result<()> {
    register_action_node!(factory, "MoveBase", MoveBase);

    Ok(())
}
