// Copyright © 2024 Stephan Kunz

//! This example implements the thirteenth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-advanced/tutorial_13_blackboard_reference
//!
//! Differences to BehaviorTree.CPP
//! - example at behaviorTree.CPP is inconsistent, does not match code in github repo
//! - could not get the get_exact::<wanted_type>() access to work
//!

use behaviortree_rs::{nodes::NodeError, prelude::*};

const XML: &str = r#"
<root BTCPP_format="4"
      main_tree_to_execute="SegmentCup">
    <BehaviorTree ID="SegmentCup">
       <Sequence>
           <AcquirePointCloud  cloud="{pointcloud}"/>
           <SegmentObject  obj_name="cup" cloud="{pointcloud}" obj_pose="{pose}"/>
       </Sequence>
    </BehaviorTree>
</root>"#;

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct PointCloud {
    points: Vec<Point>,
}

/// ActionNode "AcquirePointCloud"
#[bt_node(SyncActionNode)]
struct AcquirePointCloud {}

#[bt_node(SyncActionNode)]
impl AcquirePointCloud {
    async fn tick(&mut self) -> NodeResult {
        println!("setting PointCloud");
        // put a PointCloud into blackboard
        let p_cloud = vec![
            Point { x: 0, y: 0 },
            Point { x: 1, y: 1 },
            Point { x: 2, y: 2 },
            Point { x: 3, y: 3 },
        ];
        node_.config.blackboard.set("cloud", p_cloud);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(output_port!("cloud"))
    }
}

/// ActionNode "SegmentObject"
#[bt_node(SyncActionNode)]
struct SegmentObject {}

#[bt_node(SyncActionNode)]
impl SegmentObject {
    async fn tick(&mut self) -> NodeResult {
        println!("accessing PointCloud");
        let p_cloud = node_
            .config
            .blackboard
            .get_exact::<&PointCloud>("cloud")
            .ok_or_else(|| NodeError::PortError("cloud".into()))?;
        println!("PointCloud is {:#?}", p_cloud);

        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(
            input_port!("cloud"),
            input_port!("obj_name"),
            output_port!("obj_pose")
        )
    }
}

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // register all needed nodes
    register_action_node!(factory, "AcquirePointCloud", AcquirePointCloud);
    register_action_node!(factory, "SegmentObject", SegmentObject);

    // create the BT
    let mut tree = factory.create_sync_tree_from_text(XML.to_string(), &blackboard)?;

    // run the BT
    let result = tree.tick_while_running()?;
    println!("tree result is {result}");

    Ok(())
}
