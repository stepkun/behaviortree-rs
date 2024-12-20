// Copyright © 2024 Stephan Kunz

//! This example implements the seventh tutorial from https://www.behaviortree.dev
//! see https://www.behaviortree.dev/docs/tutorial-basics/tutorial_07_multiple_xml
//!
//! Via include gives an error.
//! Manual loading works.

use std::{fs::File, io::Read, path::PathBuf};

use behaviortree_rs::prelude::*;

const XML_MANUALLY: &str = r#"
<root BTCPP_format="4"
      main_tree_to_execute="MainTree">
    <BehaviorTree ID="MainTree">
        <Sequence>
            <SaySomething message="starting MainTree" />
            <SubTree ID="SubTreeA" />
            <SubTree ID="SubTreeB" />
        </Sequence>
    </BehaviorTree>
</root>
"#;

const XML_INCLUDE: &str = r#"
<root BTCPP_format="4"
      main_tree_to_execute="MainTree">
    <include path="./subtree_A.xml" />
    <include path="./subtree_B.xml" />
    <BehaviorTree ID="MainTree">
        <Sequence>
            <SaySomething message="starting MainTree" />
            <SubTree ID="SubTreeA" />
            <SubTree ID="SubTreeB" />
        </Sequence>
    </BehaviorTree>
</root>"#;

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
        define_ports!(input_port!("message"))
    }
}

fn main() -> anyhow::Result<()> {
    // via include
    {
        println!("subtrees via include");
        /*
        fails with
        Error: Errors like this shouldn't happen. Something bad has happened. Please report this. Empty(BytesStart { buf: Borrowed("include path=\"./subtree_A.xml\" "), name_len: 7 })
        */
        // create BT environment
        let mut factory = Factory::new();
        let blackboard = Blackboard::create();

        // register all needed nodes
        register_action_node!(factory, "SaySomething", SaySomething);

        // create tree
        let tree = factory.create_sync_tree_from_text(XML_INCLUDE.to_string(), &blackboard);
        match tree {
            Ok(mut tree) => {
                // run the BT
                let result = tree.tick_while_running()?;
                println!("tree result is {result}");
            }
            Err(error) => println!("{error}"),
        }
    }

    // manually
    {
        println!("subtrees manually");

        // create BT environment
        let mut factory = Factory::new();
        let blackboard = Blackboard::create();

        // register all needed nodes
        register_action_node!(factory, "SaySomething", SaySomething);

        // create tree
        // create the search path to xml filess independant of current directory in project
        let mut directory = std::env::current_dir()?.to_str().unwrap().to_string();
        let pos = directory.find("behaviortree-rs").expect("wrong path");
        directory.replace_range(pos.., "behaviortree-rs");
        let search_path = PathBuf::from(directory)
            .join(file!())
            .parent()
            .expect("no path to file")
            .to_string_lossy()
            .to_string();

        // iterate over directories xml files
        let files = std::fs::read_dir(search_path)?.flatten();
        for file in files {
            if file
                .file_name()
                .into_string()
                .expect("could not determine file type")
                .ends_with("xml")
            {
                // read xml from file
                let mut file = File::open(file.path())?;
                let mut xml = String::new();
                file.read_to_string(&mut xml)?;
                // register
                factory.register_bt_from_text(xml)?;
            }
        }

        //// register main tree
        //factory.register_bt_from_text(XML_MANUALLY.into())?;
        //// instantiate the BT
        //let tree = factory.instantiate_sync_tree(&blackboard, "MainTree");

        // create the BT
        let tree = factory.create_sync_tree_from_text(XML_MANUALLY.to_string(), &blackboard);
        match tree {
            Ok(mut tree) => {
                // run the BT
                let result = tree.tick_while_running()?;
                println!("tree result is {result}");
            }
            Err(error) => println!("{error}"),
        }
    }

    Ok(())
}
