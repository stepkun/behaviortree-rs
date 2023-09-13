use bt_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};

/// The ReactiveSequence is similar to a ParallelNode.
/// All the children are ticked from first to last:
/// 
/// - If a child returns RUNNING, halt the remaining siblings in the sequence and return RUNNING.
/// - If a child returns SUCCESS, tick the next sibling.
/// - If a child returns FAILURE, stop and return FAILURE.
/// 
/// If all the children return SUCCESS, this node returns SUCCESS.
/// 
/// IMPORTANT: to work properly, this node should not have more than a single
///            asynchronous child.
#[bt_node(ControlNode)]
pub struct ReactiveSequenceNode {
    #[bt(default = "-1")]
    running_child: i32,
}

impl TreeNode for ReactiveSequenceNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let mut all_skipped = true;

        self.status = NodeStatus::Running;

        for (counter, child) in self.children.iter().enumerate() {
            let child_status = child.borrow_mut().execute_tick()?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match child_status {
                NodeStatus::Running => {
                    for i in 0..counter {
                        self.halt_child(i)?;
                    }
                    if self.running_child == -1 {
                        self.running_child = counter as i32;
                    } else if self.running_child != counter as i32 {
                        // Multiple children running at the same time
                        return Err(NodeError::NodeStructureError("[ReactiveSequence]: Only a single child can return Running.".to_string()))
                    }
                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {
                    self.reset_children();
                    return Ok(NodeStatus::Failure);
                }
                // Do nothing on Success
                NodeStatus::Success => {}
                NodeStatus::Skipped => {
                    // Halt current child
                    child.borrow_mut().halt();
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(child.borrow_mut().config().path.clone(), "Idle".to_string()));
                }
            }
        }

        self.reset_children();

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Success),
        }
    }
}

impl NodeHalt for ReactiveSequenceNode {
    fn halt(&mut self) {
        self.reset_children()
    }
}