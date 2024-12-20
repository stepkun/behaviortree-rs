// Copyright © 2024 Stephan Kunz

//! This example implements the eigth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_08_additional_args
//!
//! Differences to BehaviorTree.CPP
//! - using an initialize method currently is not possible because we can not get a mutuable iterator.
//!   A method visit_nodes_mut() is missing.
//!

use behaviortree_rs::prelude::*;

const XML: &str = r#"
<root BTCPP_format="4">
    <BehaviorTree ID="MainTree">
        <Sequence>
            <ActionA message="Running ActionA" />
            <ActionB message="Running ActionB" />
            <ActionC message="Running ActionC" />
        </Sequence>
    </BehaviorTree>
</root>"#;

/// SyncActionNode "ActionA"
#[bt_node(SyncActionNode)]
struct ActionA {
    arg1: i32,
    arg2: String,
}

#[bt_node(SyncActionNode)]
impl ActionA {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("message")?;

        let arg1 = self.arg1;
        let arg2 = self.arg2.clone();
        println!("{msg} robot says: {}, the answer is {}!", arg2, arg1);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("message"))
    }
}

/// SyncActionNode "ActionB"
#[bt_node(SyncActionNode)]
struct ActionB {
    #[bt(default)]
    arg1: i32,
    #[bt(default)]
    arg2: String,
}

#[bt_node(SyncActionNode)]
impl ActionB {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("message")?;

        println!("{msg} is currently not implementable");
        //let arg1 = self.arg1;
        //let arg2 = self.arg2.clone();
        //println!("{msg} robot says: {}, the answer is {}!", arg2, arg1);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("message"))
    }

    #[allow(dead_code)]
    fn initialize(&mut self, arg1: i32, arg2: String) {
        self.arg1 = arg1;
        self.arg2 = arg2;
    }
}

/// SyncActionNode "ActionC"
#[bt_node(SyncActionNode)]
struct ActionC {
    #[bt(default = "42")]
    arg1: i32,
    #[bt(default = "String::from(\"hello world\")")]
    arg2: String,
}

#[bt_node(SyncActionNode)]
impl ActionC {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("message")?;

        let arg1 = self.arg1;
        let arg2 = self.arg2.clone();
        println!("{msg} robot says: {}, the answer is {}!", arg2, arg1);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("message"))
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    let arg1 = 42;
    let arg2 = String::from("hello world");

    // registering with a different constructor
    register_action_node!(factory, "ActionA", ActionA, arg1, arg2);
    // registering for using initialize function
    register_action_node!(factory, "ActionB", ActionB);
    // registering using defaults
    register_action_node!(factory, "ActionC", ActionC);

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(XML.to_string(), &blackboard)?;

    // initialize ActionB with the help of a visitor
    /*
    currently does not work, as there is no visit_nodes_mut() method providing a mutuable iterator
    for node in tree.visit_nodes_mut() {
        if node.name() == "ActionB" {
            let action = node.context.downcast_mut::<ActionB>().unwrap();
            action.initialize(42, "hello world".into());
        }
    }
    */

    // run the BT
    let result = tree.tick_while_running()?;
    println!("tree result is {result}");

    Ok(())
}
