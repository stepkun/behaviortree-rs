// Copyright © 2024 Stephan Kunz

//! This example implements the tenth tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_10_observer
//!
//! Currently not implementable due to same problem as in example 07_xml_files

use behaviortree_rs::prelude::*;

const XML: &str = r#"
<root BTCPP_format="4"
    main_tree_to_execute="MainTree">
    <BehaviorTree ID="MainTree">
        <Sequence>
            <Fallback>
                <AlwaysFailure name="failing_action"/>
                <SubTree ID="SubTreeA" name="mysub"/>
            </Fallback>
            <AlwaysSuccess name="last_action"/>
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="SubTreeA">
        <Sequence>
            <AlwaysSuccess name="action_subA"/>
            <SubTree ID="SubTreeB" name="sub_nested"/>
            <SubTree ID="SubTreeB" />
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="SubTreeB">
        <AlwaysSuccess name="action_subB"/>
    </BehaviorTree>
</root>
"#;

fn main() -> anyhow::Result<()> {
    // create BT environment
    let mut factory = Factory::new();
    let blackboard = Blackboard::create();

    // create the BT
    /* create tree fails at line 789 in trees.rs with
    Error: Violated node type constraint: Expected end tag for BehaviorTree
    */
    let mut tree = factory.create_sync_tree_from_text(XML.to_string(), &blackboard)?;

    // run the BT
    let result = tree.tick_while_running()?;
    println!("tree result is {result}");

    Ok(())
}
