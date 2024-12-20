# Examples for behaviortree-rs

These examples follow the tutorial of [BehaviorTree.dev](https://www.behaviortree.dev/docs/intro).

## [A first behaviortree](01_first/main.rs)

This example implements the [first tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_01_first_tree)

Run it with ```cargo run --example 01_first```

Differences to BehaviorTree.CPP:

- we cannot register functions/methods of a struct/class
- there is no separate ConditionNode type, these have to be implemented as SyncActionNode
- we do not have a nice function to read xml definitions from a file

## [Blackboard and ports](02_ports/main.rs)

This example implements the [second tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_02_basic_ports)

Run it with ```cargo run --example 02_ports```

Differences to BehaviorTree.CPP

- we do not have a nice function to read xml definitions from a file

## [Use generic types with ports](03_ports_generic/main.rs)

This example implements the [third tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_03_generic_ports)

Run it with ```cargo run --example 03_ports_generic```

Differences to BehaviorTree.CPP

- there is no Script node available, that has to be implemented by user

## [Create reactive behavior](04_reactive/main.rs)

This example implements the [fourth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_04_sequence)

Run it with ```cargo run --example 04_reactive```

Differences to BehaviorTree.CPP

- there is no tree::sleep(...) available, using sleep of tokio async runtime instead, which is not interrupted when tree state changes

## [Use of subtrees](05_subtrees/main.rs)

This example implements the [fifth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_05_subtrees)

Run it with ```cargo run --example 05_subtrees```

Differences to BehaviorTree.CPP

- we do not have a nice function to read xml definitions from a file

It is enriched with random behavior of nodes, so run it several times to see different executions

- IsDoorClosed in [main.rs](05_subtrees/main.rs)
- OpenDoor in [subtree.rs](05_subtrees/subtree.rs)
- PickLock in [subtree.rs](05_subtrees/subtree.rs)

## [Remapping of ports](06_port_remapping/main.rs)

This example implements the [sixth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_06_subtree_ports)

Run it with ```cargo run --example 06_port_remapping```

Differences to BehaviorTree.CPP

- there is no Script node available, that has to be implemented by user
- no access to sub blackboards
- no 'Display' implementation for Blackboard
- `Debug` implementation is basic

## [Use multiple xml files](07_xml_files/main.rs)

This example implements the [seventh tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_07_multiple_xml)

Run it with ```cargo run --example 07_xml_files```

Via include gives

```text
Error: Errors like this shouldn't happen. Something bad has happened. Please report this. Empty(BytesStart { buf: Borrowed("include path=\"./subtree_A.xml\" "), name_len: 7 })
```

Manual loading works

## [Pass arguments to nodes](08_arguments/main.rs)

This example implements the [eighth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_08_additional_args)

Run it with ```cargo run --example 08_arguments```

Differences to BehaviorTree.CPP

- using an initialize method currently is not possible, a method visit_nodes_mut() is missing


## [Scripting](09_scripting/main.rs)

This example implements the [nineth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_09_scripting)

Run it with ```cargo run --example 09_scripting```

Currently not implemented as there is no scripting node in the library

## [Logging and observer](10_observer/main.rs)

This example implements the [tenth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_10_observer)

Run it with ```cargo run --example 10_observer```

Currently not implementable due to same problem in tree.rs as in 07_xml_files

## [Connection to groot2](11_groot2/main.rs)

This example implements the [eleventh tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_11_groot2)

Run it with ```cargo run --example 11_groot2```

Currently not implementable due to missing functionality

## [Default values for ports](12_port_defaults/main.rs)

This example implements the [twelveth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_12_default_ports)

Run it with ```cargo run --example 12_port_defaults```

Differences to BehaviorTree.CPP

- It is not possible to add an action node directly below the root node (same error as in 07_xml_files and 10_observer)
- only 3 of the 6 ways in BehaviorTree.CPP are working (at least in an easy manner)

## [Access ports by reference](13_port_by_reference/main.rs)

This example implements the [thirteenth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_13_blackboard_reference)

Run it with ```cargo run --example 13_port_by_reference```

Not working

Differences to BehaviorTree.CPP
- example at behaviorTree.CPP is inconsistent, does not match code in github repo
- could not get the ```blackboard.get_exact::<wanted_type>()``` access to work

## [Subtree models and autoremap](14_autoremap/main.rs)

This example implements the [fourteenth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_14_subtree_model)

Run it with ```cargo run --example 14_autoremap```

Differences to BehaviorTree.CPP

- example in BehaviorTree.CPP is inconsistent
- not sure wether this example really shows how to do it

## [Mocking and replacement of nodes](15_mocking_replacement/main.rs)

This example implements the [fifteenth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_15_replace_rules)

Run it with ```cargo run --example 15_mocking_replacement```

Currently not implementable due to missing functionality

## [Usage of a global blackboard](16_global_blackboard/main.rs)

This example implements the [sixteenth tutorial from BehaviorTree.dev](https://www.behaviortree.dev/docs/tutorial-basics/tutorial_16_global_blackboard)

Run it with ```cargo run --example 16_global_blackboard```

Currently not working due to missing functionality

Differences to BehaviorTree.CPP

- there is no Script node available, that has to be implemented by user
- enhanced example for trees with depth > 2 level
