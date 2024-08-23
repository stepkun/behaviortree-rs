use std::{collections::HashMap, io::Cursor, string::FromUtf8Error, sync::Arc};

use futures::future::BoxFuture;
use log::{debug, info};
use quick_xml::{
    events::{attributes::Attributes, Event},
    name::QName,
    Reader,
};
use thiserror::Error;

use crate::{
    basic_types::{
        AttrsToMap, FromString, NodeCategory, NodeStatus, ParseBoolError, PortChecks,
        PortDirection, PortsRemapping,
    },
    blackboard::{Blackboard, BlackboardString},
    macros::build_node_ptr,
    nodes::{self, NodeConfig, NodeResult, TreeNode},
};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Port name [{0}] did not match Node [{1}] port list: {2:?}")]
    /// `(port_name, node_name, port_list)`
    InvalidPort(String, String, Vec<String>),
    #[error("Error occurred parsing XML attribute: {0}")]
    AttrError(#[from] quick_xml::events::attributes::AttrError),
    #[error("Error occurred parsing XML: {0}")]
    XMLError(#[from] quick_xml::Error),
    #[error("Expected to find <root> start tag at start of XML. Found incorrect tag.")]
    MissingRoot,
    #[error("Expected to find <root> tag at start of XML. Found <{0}> instead.")]
    ExpectedRoot(String),
    #[error("Reached EOF of the XML unexpectedly.")]
    UnexpectedEof,
    #[error("Error parsing UTF8: {0}")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("Attempted to parse node with unregistered name: {0}")]
    UnknownNode(String),
    #[error("Errors like this shouldn't happen. {0}")]
    InternalError(String),
    #[error("{0}")]
    MissingAttribute(String),
    #[error("Can't find tree [{0}]")]
    UnknownTree(String),
    #[error("Node type [] didn't had invalid presence/absence of children.")]
    NodeTypeMismatch(String),
    #[error("No main tree was provided, either in the XML or as a function parameter.")]
    NoMainTree,
    #[error("{0}")]
    ParseStringError(#[from] ParseBoolError),
    #[error("Violated node type constraint: {0}")]
    ViolateNodeConstraint(String),
}

type NodeCreateFnDyn = dyn Fn(NodeConfig, Vec<TreeNode>) -> TreeNode + Send + Sync;

enum TickOption {
    WhileRunning,
    ExactlyOnce,
    OnceUnlessWokenUp,
}

pub struct NodeIter<'a> {
    nodes: Vec<&'a TreeNode>,
    idxs: Vec<i32>,
}

#[derive(Debug)]
enum CreateNodeResult {
    Node(TreeNode),
    Continue,
    End,
}

#[derive(Debug)]
pub struct AsyncTree {
    root: TreeNode,
}

impl AsyncTree {
    pub fn new(root: TreeNode) -> AsyncTree {
        Self { root }
    }

    async fn tick_root(&mut self, opt: TickOption) -> NodeResult {
        let mut status = NodeStatus::Idle;

        while status == NodeStatus::Idle
            || (matches!(opt, TickOption::WhileRunning) && matches!(status, NodeStatus::Running))
        {
            status = self.root.execute_tick().await?;

            // Not implemented: Check for wake-up conditions and tick again if so

            if status.is_completed() {
                self.root.reset_status();
            }
        }

        Ok(status)
    }

    pub async fn tick_exactly_once(&mut self) -> NodeResult {
        self.tick_root(TickOption::ExactlyOnce).await
    }

    pub async fn tick_once(&mut self) -> NodeResult {
        self.tick_root(TickOption::OnceUnlessWokenUp).await
    }

    pub async fn tick_while_running(&mut self) -> NodeResult {
        self.tick_root(TickOption::WhileRunning).await
    }

    pub async fn root_blackboard(&self) -> Blackboard {
        self.root.config().blackboard.clone()
    }

    pub async fn halt_tree(&mut self) {
        self.root.halt().await;
    }

    pub fn visit_nodes(&self) -> impl Iterator<Item = &TreeNode> {
        NodeIter::new(&self.root)
    }
}

impl<'a> NodeIter<'a> {
    pub fn new(root: &'a TreeNode) -> Self {
        Self {
            nodes: vec![root],
            idxs: vec![-1],
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a TreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        // Loop until we find a node to return
        loop {
            // Out of nodes; we have traversed the entire tree
            if self.nodes.is_empty() {
                return None;
            }

            let end_idx = self.nodes.len() - 1;

            let node = self.nodes[end_idx];
            let child_idx = &mut self.idxs[end_idx];

            // When this index is -1, that means we haven't returned the node yet
            if *child_idx < 0 {
                self.idxs[end_idx] = 0;
                return Some(node);
            } else if node.children().is_none()
                || *child_idx >= node.children().unwrap().len() as i32
            {
                // When the node has no children, pop it off and try the next element
                // OR
                // If we've already returned all children, pop it off
                // Unwrap is safe because we just checked if it's None
                self.nodes.pop();
                self.idxs.pop();
            } else {
                // If nothing else applies, we can push the node's child and return it
                // Unwrap is safe because we just checked if it's None
                let child = &node.children().unwrap()[*child_idx as usize];
                *child_idx += 1;

                self.nodes.push(child);
                self.idxs.push(-1);
            }
        }
    }
}

#[derive(Debug)]
pub struct SyncTree {
    root: AsyncTree,
}

impl SyncTree {
    pub fn new(root: TreeNode) -> SyncTree {
        Self {
            root: AsyncTree::new(root),
        }
    }

    pub fn tick_exactly_once(&mut self) -> NodeResult {
        futures::executor::block_on(self.root.tick_exactly_once())
    }

    pub fn tick_once(&mut self) -> NodeResult {
        futures::executor::block_on(self.root.tick_once())
    }

    pub fn tick_while_running(&mut self) -> NodeResult {
        futures::executor::block_on(self.root.tick_while_running())
    }

    pub fn root_blackboard(&self) -> Blackboard {
        futures::executor::block_on(self.root.root_blackboard())
    }

    pub async fn halt_tree(&mut self) {
        futures::executor::block_on(self.root.halt_tree());
    }

    pub fn visit_nodes(&self) -> impl Iterator<Item = &TreeNode> {
        NodeIter::new(&self.root.root)
    }
}

pub struct Factory {
    node_map: HashMap<String, (NodeCategory, Arc<NodeCreateFnDyn>)>,
    blackboard: Blackboard,
    tree_roots: HashMap<String, Reader<Cursor<Vec<u8>>>>,
    main_tree_id: Option<String>,
    // TODO: temporary solution, potentially replace later
    tree_uid: std::sync::Mutex<u32>,
}

impl Factory {
    pub fn new() -> Factory {
        let blackboard = Blackboard::create();

        Self {
            node_map: builtin_nodes(),
            blackboard,
            tree_roots: HashMap::new(),
            main_tree_id: None,
            tree_uid: std::sync::Mutex::new(0),
        }
    }

    pub fn blackboard(&mut self) -> &Blackboard {
        &self.blackboard
    }

    pub fn set_blackboard(&mut self, blackboard: Blackboard) {
        self.blackboard = blackboard;
    }

    pub fn register_node<F>(&mut self, name: impl AsRef<str>, node_fn: F, node_type: NodeCategory)
    where
        F: Fn(NodeConfig, Vec<TreeNode>) -> TreeNode + Send + Sync + 'static,
    {
        self.node_map
            .insert(name.as_ref().into(), (node_type, Arc::new(node_fn)));
    }

    fn create_node(
        &self,
        node_fn: &Arc<NodeCreateFnDyn>,
        config: NodeConfig,
        children: Vec<TreeNode>,
    ) -> TreeNode {
        node_fn(config, children)
    }

    fn get_uid(&self) -> u32 {
        let uid = *self.tree_uid.lock().unwrap();
        *self.tree_uid.lock().unwrap() += 1;

        uid
    }

    async fn recursively_build_subtree(
        &self,
        tree_id: &String,
        tree_name: &String,
        path_prefix: &String,
        blackboard: Blackboard,
    ) -> Result<TreeNode, ParseError> {
        let mut reader = match self.tree_roots.get(tree_id) {
            Some(root) => root.clone(),
            None => {
                return Err(ParseError::UnknownTree(tree_id.clone()));
            }
        };

        // Loop until either a child or end tag is found
        loop {
            match self
                .build_child(&mut reader, &blackboard, tree_name, path_prefix)
                .await?
            {
                CreateNodeResult::Node(child) => break Ok(child),
                CreateNodeResult::Continue => (),
                CreateNodeResult::End => {
                    break Err(ParseError::NodeTypeMismatch("SubTree".to_string()))
                }
            }
        }
    }

    pub fn create_sync_tree_from_text(
        &mut self,
        text: String,
        blackboard: &Blackboard,
    ) -> Result<SyncTree, ParseError> {
        self.register_bt_from_text(text)?;

        if self.tree_roots.len() > 1 && self.main_tree_id.is_none() {
            Err(ParseError::NoMainTree)
        } else if self.tree_roots.len() == 1 {
            // Unwrap is safe because we check that tree_roots.len() == 1
            let main_tree_id = self.tree_roots.iter().next().unwrap().0.clone();

            self.instantiate_sync_tree(blackboard, &main_tree_id)
        } else {
            // Unwrap is safe here because there are more than 1 root and
            // self.main_tree_id is Some
            let main_tree_id = self.main_tree_id.clone().unwrap();
            self.instantiate_sync_tree(blackboard, &main_tree_id)
        }
    }

    pub async fn create_async_tree_from_text(
        &mut self,
        text: String,
        blackboard: &Blackboard,
    ) -> Result<AsyncTree, ParseError> {
        self.register_bt_from_text(text)?;

        if self.tree_roots.len() > 1 && self.main_tree_id.is_none() {
            Err(ParseError::NoMainTree)
        } else if self.tree_roots.len() == 1 {
            // Unwrap is safe because we check that tree_roots.len() == 1
            let main_tree_id = self.tree_roots.iter().next().unwrap().0.clone();

            self.instantiate_async_tree(blackboard, &main_tree_id).await
        } else {
            // Unwrap is safe here because there are more than 1 root and
            // self.main_tree_id is Some
            let main_tree_id = self.main_tree_id.clone().unwrap();
            self.instantiate_async_tree(blackboard, &main_tree_id).await
        }
    }

    pub fn instantiate_sync_tree(
        &mut self,
        blackboard: &Blackboard,
        main_tree_id: &str,
    ) -> Result<SyncTree, ParseError> {
        // Clone ptr to Blackboard
        let blackboard = blackboard.clone();

        let main_tree_id = String::from(main_tree_id);

        let root_node = futures::executor::block_on(self.recursively_build_subtree(
            &main_tree_id,
            &String::new(),
            &String::new(),
            blackboard,
        ))?;

        Ok(SyncTree::new(root_node))
    }

    pub async fn instantiate_async_tree(
        &mut self,
        blackboard: &Blackboard,
        main_tree_id: &str,
    ) -> Result<AsyncTree, ParseError> {
        // Clone ptr to Blackboard
        let blackboard = blackboard.clone();

        let main_tree_id = String::from(main_tree_id);

        let root_node = self
            .recursively_build_subtree(&main_tree_id, &String::new(), &String::new(), blackboard)
            .await?;

        Ok(AsyncTree::new(root_node))
    }

    async fn build_leaf_node<'a>(
        &self,
        node_name: &String,
        attributes: Attributes<'a>,
        config: NodeConfig,
    ) -> Result<TreeNode, ParseError> {
        // Get clone of node from node_map based on tag name
        let (node_type, node_fn) = self
            .node_map
            .get(node_name)
            .ok_or_else(|| ParseError::UnknownNode(node_name.clone()))?;
        if !matches!(node_type, NodeCategory::Action) {
            return Err(ParseError::NodeTypeMismatch(String::from("Action")));
        }

        let mut node = self.create_node(node_fn, config, Vec::new());

        self.add_ports_to_node(&mut node, node_name, attributes)
            .await?;

        Ok(node)
    }

    async fn build_children(
        &self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
        blackboard: &Blackboard,
        tree_name: &String,
        path_prefix: &String,
    ) -> Result<Vec<TreeNode>, ParseError> {
        let mut nodes = Vec::new();

        loop {
            match self
                .build_child(reader, blackboard, tree_name, path_prefix)
                .await?
            {
                CreateNodeResult::Node(node) => {
                    nodes.push(node);
                }
                CreateNodeResult::Continue => (),
                CreateNodeResult::End => break,
            }
        }

        Ok(nodes)
    }

    async fn add_ports_to_node<'a>(
        &self,
        node_ptr: &mut TreeNode,
        node_name: &str,
        attributes: Attributes<'a>,
    ) -> Result<(), ParseError> {
        let config = node_ptr.config_mut();
        let manifest = config.manifest()?;

        let mut remap = PortsRemapping::new();

        for (port_name, port_value) in attributes.to_map()? {
            remap.insert(port_name, port_value);
        }

        // Check if all ports from XML match ports in manifest
        for port_name in remap.keys() {
            if !manifest.ports.contains_key(port_name) {
                return Err(ParseError::InvalidPort(
                    port_name.clone(),
                    node_name.to_owned(),
                    manifest.ports.to_owned().into_keys().collect(),
                ));
            }
        }

        // Add ports to NodeConfig
        for (remap_name, remap_val) in remap {
            if let Some(port) = manifest.ports.get(&remap_name) {
                config.add_port(port.direction().clone(), remap_name, remap_val);
            }
        }

        // Try to use defaults for unspecified port values
        for (port_name, port_info) in manifest.ports.iter() {
            let direction = port_info.direction();

            if !matches!(direction, PortDirection::Output)
                && !config.has_port(direction, port_name)
                && port_info.default_value().is_some()
            {
                config.add_port(
                    PortDirection::Input,
                    port_name.clone(),
                    port_info.default_value_str().unwrap(),
                );
            }
        }

        Ok(())
    }

    fn build_child<'a>(
        &'a self,
        reader: &'a mut Reader<Cursor<Vec<u8>>>,
        blackboard: &'a Blackboard,
        tree_name: &'a String,
        path_prefix: &'a String,
    ) -> BoxFuture<Result<CreateNodeResult, ParseError>> {
        Box::pin(async move {
            let mut buf = Vec::new();

            let node = match reader.read_event_into(&mut buf)? {
                // exits the loop when reaching end of file
                Event::Eof => {
                    debug!("EOF");
                    return Err(ParseError::UnexpectedEof);
                }
                // Node with Children
                Event::Start(e) => {
                    let node_name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes();

                    debug!("build_child Start: {node_name}");

                    let mut config = NodeConfig::new(blackboard.clone());
                    config.path = path_prefix.to_owned() + &node_name;

                    let (node_type, node_fn) = self
                        .node_map
                        .get(&node_name)
                        .ok_or_else(|| ParseError::UnknownNode(node_name.clone()))?;

                    let node = match node_type {
                        NodeCategory::Control => {
                            let children = self
                                .build_children(
                                    reader,
                                    blackboard,
                                    tree_name,
                                    &(config.path.to_owned() + "/"),
                                )
                                .await?;

                            let mut node = self.create_node(node_fn, config, children);

                            self.add_ports_to_node(&mut node, &node_name, attributes)
                                .await?;

                            node
                        }
                        NodeCategory::Decorator => {
                            // Make checkpoint to rewind to if necessary
                            let checkpoint = reader.buffer_position();
                            // Loop until either an end tag or the child is found
                            let child = loop {
                                match self
                                    .build_child(
                                        reader,
                                        blackboard,
                                        tree_name,
                                        &(config.path.to_owned() + "/"),
                                    )
                                    .await?
                                {
                                    CreateNodeResult::Node(node) => break node,
                                    CreateNodeResult::Continue => (),
                                    CreateNodeResult::End => {
                                        return Err(ParseError::NodeTypeMismatch(
                                            "Decorator".to_string(),
                                        ))
                                    }
                                }
                            };

                            let mut buf = Vec::new();

                            // Try to match the end tag to close the Decorator
                            loop {
                                match reader.read_event_into(&mut buf)? {
                                    // Ignore comments
                                    Event::Comment(_) => continue,
                                    Event::End(tag) => {
                                        // If a matching end tag is found, all good
                                        if tag.name() == e.name() {
                                            break;
                                        } else {
                                            // Otherwise, an error. Theoretically this should be unreachable since the XML parser should catch this error, but keeping it here just in case
                                            return Err(ParseError::ViolateNodeConstraint(
                                                format!(
                                                    "Expected end tag for Decorator {node_name}"
                                                ),
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(ParseError::ViolateNodeConstraint(format!(
                                            "Decorator node [{node_name}] may only have one child"
                                        )));
                                    }
                                }
                            }

                            let mut node = self.create_node(node_fn, config, vec![child]);

                            self.add_ports_to_node(&mut node, &node_name, attributes)
                                .await?;

                            node
                        }
                        // TODO: expand more
                        x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
                    };

                    CreateNodeResult::Node(node)
                }
                // Leaf Node
                Event::Empty(e) => {
                    let node_name = String::from_utf8(e.name().0.into())?;
                    debug!("[Leaf node]: {node_name}");
                    let attributes = e.attributes();

                    let mut config = NodeConfig::new(blackboard.clone());
                    config.path = path_prefix.to_owned() + &node_name;

                    let node = match node_name.as_str() {
                        "SubTree" => {
                            let attributes = attributes.to_map()?;
                            let mut child_blackboard = Blackboard::with_parent(blackboard);

                            // Process attributes (Ports, special fields, etc)
                            for (attr, value) in attributes.iter() {
                                // Set autoremapping to true or false
                                if attr == "_autoremap" {
                                    child_blackboard.enable_auto_remapping(
                                        <bool as FromString>::from_string(value)?,
                                    );
                                    continue;
                                } else if !attr.is_allowed_port_name() {
                                    continue;
                                }

                                if let Some(port_name) = value.strip_bb_pointer() {
                                    // Add remapping if `value` is a Blackboard pointer
                                    child_blackboard.add_subtree_remapping(attr.clone(), port_name);
                                } else {
                                    // Set string value into Blackboard
                                    child_blackboard.set(attr, value.clone());
                                }
                            }

                            let id = match attributes.get("ID") {
                                Some(id) => id,
                                None => return Err(ParseError::MissingAttribute("ID".to_string())),
                            };

                            let mut subtree_name = tree_name.clone();
                            if !subtree_name.is_empty() {
                                subtree_name += "/";
                            }

                            if let Some(name_attr) = attributes.get("name") {
                                subtree_name += name_attr;
                            } else {
                                subtree_name += &format!("{id}::{}", self.get_uid());
                            }

                            let new_prefix = format!("{subtree_name}/");

                            self.recursively_build_subtree(
                                id,
                                &subtree_name,
                                &new_prefix,
                                child_blackboard,
                            )
                            .await?
                        }
                        _ => self.build_leaf_node(&node_name, attributes, config).await?,
                    };

                    CreateNodeResult::Node(node)
                }
                Event::End(_e) => CreateNodeResult::End,
                Event::Comment(content) => {
                    debug!("Comment - \"{content:?}\"");
                    CreateNodeResult::Continue
                }
                e => {
                    debug!("Other - SHOULDN'T BE HERE");
                    debug!("{e:?}");

                    return Err(ParseError::InternalError(
                        "Didn't match one of the expected XML tag types.".to_string(),
                    ));
                }
            };

            Ok(node)
        })
    }

    pub fn register_bt_from_text(&mut self, xml: String) -> Result<(), ParseError> {
        let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes().to_vec()));
        reader.trim_text(true);

        let mut buf = Vec::new();

        // TODO: Check includes

        // TODO: Parse for correctness

        loop {
            // Try to match root tag
            match reader.read_event_into(&mut buf)? {
                // Ignore XML declaration tag <?xml ...
                Event::Decl(_) => buf.clear(),
                Event::Start(e) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes().to_map()?;

                    if name.as_str() != "root" {
                        buf.clear();
                        continue;
                    }

                    if let Some(tree_id) = attributes.get("main_tree_to_execute") {
                        info!("Found main tree ID: {tree_id}");
                        self.main_tree_id = Some(tree_id.clone());
                    }

                    buf.clear();
                    break;
                }
                _ => return Err(ParseError::MissingRoot),
            }
        }

        // Register each BehaviorTree in the XML
        loop {
            let event = { reader.read_event_into(&mut buf)? };

            match event {
                Event::Start(e) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes().to_map()?;

                    // Strange method of cloning QName such that the internal buffer is also cloned
                    // Otherwise, borrow checker errors with &mut buf still being borrowed
                    let end = e.to_end();
                    let end_name = end.name().as_ref().to_vec().clone();
                    let end_name = QName(end_name.as_slice());

                    // TODO: Maybe do something with TreeNodesModel?
                    // For now, just ignore it
                    if name.as_str() == "TreeNodesModel" {
                        reader.read_to_end_into(end_name, &mut buf)?;
                    } else {
                        // Add error for missing BT
                        if name.as_str() != "BehaviorTree" {
                            return Err(ParseError::ExpectedRoot(name));
                        }

                        // Save position of Reader for each BT
                        if let Some(id) = attributes.get("ID") {
                            self.tree_roots.insert(id.clone(), reader.clone());
                        } else {
                            return Err(ParseError::MissingAttribute("Found BehaviorTree definition without ID. Cannot continue parsing.".to_string()));
                        }

                        let mut buf = Vec::new();

                        // Try to match the first node and skip past it
                        loop {
                            match reader.read_event_into(&mut buf)? {
                                // Ignore comments
                                Event::Comment(_) => continue,
                                Event::End(tag) => {
                                    // If a matching end tag is found, all good
                                    if tag.name() == e.name() {
                                        break;
                                    } else {
                                        // Otherwise, an error. Theoretically this should be unreachable since the XML parser should catch this error, but keeping it here just in case
                                        return Err(ParseError::ViolateNodeConstraint(
                                            String::from("Expected end tag for BehaviorTree"),
                                        ));
                                    }
                                }
                                Event::Start(e) => {
                                    let mut buf = Vec::new();
                                    reader.read_to_end_into(e.name(), &mut buf)?;
                                    break;
                                    // return Err(ParseError::ViolateNodeConstraint(String::from("BehaviorTree node may only have one child")));
                                }
                                _ => continue,
                            }
                        }

                        // Try to match the end tag to close the BehaviorTree
                        loop {
                            match reader.read_event_into(&mut buf)? {
                                // Ignore comments
                                Event::Comment(_) => continue,
                                Event::End(tag) => {
                                    // If a matching end tag is found, all good
                                    if tag.name() == e.name() {
                                        break;
                                    } else {
                                        // Otherwise, an error. Theoretically this should be unreachable since the XML parser should catch this error, but keeping it here just in case
                                        return Err(ParseError::ViolateNodeConstraint(
                                            String::from("Expected end tag for BehaviorTree"),
                                        ));
                                    }
                                }
                                _ => {
                                    return Err(ParseError::ViolateNodeConstraint(String::from(
                                        "BehaviorTree node may only have one child",
                                    )));
                                }
                            }
                        }
                    }
                }
                Event::End(e) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    if name != "root" {
                        return Err(ParseError::InternalError("A non-root end tag was found. This should not happen. Please report this.".to_string()));
                    } else {
                        break;
                    }
                }
                Event::Comment(_) => (),
                x => {
                    return Err(ParseError::InternalError(format!(
                        "Something bad has happened. Please report this. {x:?}"
                    )))
                }
            };
        }

        buf.clear();

        Ok(())
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

fn builtin_nodes() -> HashMap<String, (NodeCategory, Arc<NodeCreateFnDyn>)> {
    let mut node_map = HashMap::new();

    // Control nodes
    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Sequence", nodes::control::SequenceNode);
            node.data.children = children;
            node
        },
    ) as Arc<NodeCreateFnDyn>;
    node_map.insert(String::from("Sequence"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "ReactiveSequence",
                nodes::control::ReactiveSequenceNode
            );
            node.data.children = children;
            node
        },
    );
    node_map.insert(
        String::from("ReactiveSequence"),
        (NodeCategory::Control, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "SequenceStar",
                nodes::control::SequenceWithMemoryNode
            );
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("SequenceStar"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Parallel", nodes::control::ParallelNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("Parallel"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "ParallelAll", nodes::control::ParallelAllNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("ParallelAll"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Fallback", nodes::control::FallbackNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("Fallback"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "ReactiveFallback",
                nodes::control::ReactiveFallbackNode
            );
            node.data.children = children;
            node
        },
    );
    node_map.insert(
        String::from("ReactiveFallback"),
        (NodeCategory::Control, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "IfThenElse", nodes::control::IfThenElseNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("IfThenElse"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "WhileDoElse", nodes::control::WhileDoElseNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("WhileDoElse"), (NodeCategory::Control, node));

    // Decorator nodes
    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node =
                build_node_ptr!(config, "ForceFailure", nodes::decorator::ForceFailureNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(
        String::from("ForceFailure"),
        (NodeCategory::Decorator, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node =
                build_node_ptr!(config, "ForceSuccess", nodes::decorator::ForceSuccessNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(
        String::from("ForceSuccess"),
        (NodeCategory::Decorator, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Inverter", nodes::decorator::InverterNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("Inverter"), (NodeCategory::Decorator, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "KeepRunningUntilFailure",
                nodes::decorator::KeepRunningUntilFailureNode
            );
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(
        String::from("KeepRunningUntilFailure"),
        (NodeCategory::Decorator, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Repeat", nodes::decorator::RepeatNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("Repeat"), (NodeCategory::Decorator, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Retry", nodes::decorator::RetryNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("Retry"), (NodeCategory::Decorator, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "RunOnce", nodes::decorator::RunOnceNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("RunOnce"), (NodeCategory::Decorator, node));

    node_map
}
