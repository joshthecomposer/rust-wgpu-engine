pub struct BehaviorTree {
    pub root: NodeId,
    pub nodes: Vec<BtNode>,
}

pub struct NodeId(pub usize);

pub enum BtNode {
    Action(ActionNode),
    Selector(SelectorNode),
    Sequence(SequenceNode),
    // Inverter(InverterNode),
    // RepeatUntilFail(RepeatUntilFailNode),
    // Condition(ConditionNode),
}

pub struct SequenceNode {
    pub children: Vec<NodeId>,
}

pub struct SelectorNode {
    pub children: Vec<NodeId>,
}

pub struct ActionNode {
    pub children: Vec<NodeId>,
}
