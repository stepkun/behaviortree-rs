// Copyright © 2024 Stephan Kunz

//! This example implements the sixteenth tutorial from https://www.behaviortree.dev
//! https://www.behaviortree.dev/docs/tutorial-advanced/tutorial_16_global_blackboard
//!
//! Differences to BehaviorTree.CPP
//! - currently not working due to missing functionality
//! 

use behaviortree_rs::{nodes::NodeError, prelude::*};

const XML: &str = r#"
<root BTCPP_format="4"
      main_tree_to_execute="MainTree">
    <BehaviorTree ID="MainTree">
        <Sequence>
            <PrintNumber name="main_print" val="{@value}" />
            <SubTree ID="MySub"/>
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="MySub">
        <Sequence>
            <PrintNumber name="sub_print" val="{@value}" />
            <SubTree ID="MySubSub"/>
            <Script code="@value_sqr := @value * @value" />
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="MySubSub">
        <Sequence>
            <PrintNumber name="sub_sub_print" val="{@value}" />
            <Script code="@value_pow3 := @value * @value * @value" />
            <SubTree ID="MySubSubSub"/>
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="MySubSubSub">
        <Sequence>
            <PrintNumber name="sub_sub_sub_print" val="{@value}" />
            <Script code="@value_pow4 := @value * @value * @value * @value" />
        </Sequence>
    </BehaviorTree>
</root>"#;

/// ActionNode "PrintNumber"
#[bt_node(SyncActionNode)]
struct PrintNumber {}

#[bt_node(SyncActionNode)]
impl PrintNumber {
    async fn tick(&mut self) -> NodeResult {
        let name: String = node_.config.get_input("name")?;
        println!("PrintNumber {}", name);
        let value: i32 = node_.config.get_input("val")?;

        println!("[{}] val: {}", name, value);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("name"), input_port!("val"))
    }
}

/// SyncActionNode "Script"
#[bt_node(SyncActionNode)]
struct Script {}

#[bt_node(SyncActionNode)]
impl Script {
    async fn tick(&mut self) -> NodeResult {
        print!("Script: ");
        let script: String = node_.config.get_input("code")?;
        let elements: Vec<&str> = script.split(":=").collect();
        //println!("{} - {}", elements[0], elements[1]);

        // try to cheat, as there is no script language implemented
        let value: i32 = node_
            .config
            .blackboard
            .get("@value")
            .ok_or_else(|| NodeError::PortError("@value".into()))?;
        if  elements[0].contains("value_sqr") {
            println!("sqr");
            node_.config.blackboard.set("@value_sqr", value * value);
            Ok(NodeStatus::Success)
        } else if  elements[0].contains("value_pow3") {
            println!("pow3");
            node_.config.blackboard.set("@value_pow3", value * value * value);
            Ok(NodeStatus::Success)
        } else if  elements[0].contains("value_pow4") {
            println!("pow4");
            node_.config.blackboard.set("@value_pow4", value * value * value * value);
            Ok(NodeStatus::Success)
        } else {
            Ok(NodeStatus::Failure)
        }
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("code"))
    }
}

fn main() -> anyhow::Result<()> {
    // create an external blackboard which will survive the tree
    let mut global_blackboard = Blackboard::create();

    // create BT environment
    let mut factory = Factory::new();
    // BT-Trees blackboard has global blackboard as parent
    let blackboard = Blackboard::with_parent(&global_blackboard);

    // register all needed nodes
    register_action_node!(factory, "PrintNumber", PrintNumber);
    register_action_node!(factory, "Script", Script);

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(XML.into(), &blackboard)?;

    // direct interaction with the global blackboard
    for value in 1..=3 {
        global_blackboard.set("value", value);
        tree.tick_once()?;
        let value_sqr = global_blackboard
            .get::<i32>("value_sqr")
            .ok_or_else(|| NodeError::PortError("value_sqr".into()))?;
        let value_pow3 = global_blackboard
            .get::<i32>("value_pow3")
            .ok_or_else(|| NodeError::PortError("value_pow3".into()))?;
        let value_pow4 = global_blackboard
            .get::<i32>("value_pow4")
            .ok_or_else(|| NodeError::PortError("value_pow3".into()))?;
        println!("[While loop] value: {value} value_sqr: {value_sqr} value_pow3: {value_pow3} value_pow4: {value_pow4}");
    }

    Ok(())
}
