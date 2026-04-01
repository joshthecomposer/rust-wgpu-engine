use crate::entity_manager::EntityManager;

pub struct BehaviorTree {
    pub root: NodeId,
    pub nodes: Vec<BtNode>,
}

impl BehaviorTree {
    pub fn new() -> Self {
        let mut tree = BehaviorTree::default();

        let in_range = tree.add_node(BtNode::Condition(ConditionNode {
            condition: ConditionKind::InAttackRange,
        }));

        let attack = tree.add_node(BtNode::Action(ActionNode {
            action: ActionKind::AttackPlayer,
        }));

        let can_see = tree.add_node(BtNode::Condition(ConditionNode {
            condition: ConditionKind::CanSeePlayer,
        }));

        let attack_seq = tree.add_node(BtNode::Sequence(SequenceNode {
            children: vec![in_range, can_see, attack],
        }));

        let in_aggro = tree.add_node(BtNode::Condition(ConditionNode {
            condition: ConditionKind::InAggroRange,
        }));
        let chase = tree.add_node(BtNode::Action(ActionNode {
            action: ActionKind::ChasePlayer,
        }));
        let chase_seq = tree.add_node(BtNode::Sequence(SequenceNode {
            children: vec![can_see, in_aggro, chase],
        }));

        let idle = tree.add_node(BtNode::Action(ActionNode {
            action: ActionKind::Idle,
        }));

        let root = tree.add_node(BtNode::Selector(SelectorNode {
            children: vec![attack_seq, chase_seq, idle],
        }));

        tree.root = root;

        tree
    }

    pub fn update(&self, ctx: &mut BtContext) -> BtStatus {
        self.visit_node(&self.root, ctx)
    }

    fn visit_node(&self, id: &NodeId, ctx: &mut BtContext) -> BtStatus {
        match &self.nodes[id.0] {
            BtNode::Action(node) => self.evaluate_action(node, ctx),
            BtNode::Condition(node) => self.evaluate_condition(node, ctx),
            BtNode::Sequence(node) => self.evaluate_sequence(node, ctx),
            BtNode::Selector(node) => self.evaluate_selector(node, ctx),
        }
    }

    fn evaluate_action(&self, node: &ActionNode, ctx: &mut BtContext) -> BtStatus {
        match node.action {
            ActionKind::Idle => {
                ctx.desired_action = Some(ActionKind::Idle);
                BtStatus::Success
            }
            ActionKind::ChasePlayer => {
                ctx.desired_action = Some(ActionKind::ChasePlayer);
                BtStatus::Running
            }
            ActionKind::AttackPlayer => {
                ctx.desired_action = Some(ActionKind::AttackPlayer);
                BtStatus::Running
            }
        }
    }

    fn evaluate_condition(&self, node: &ConditionNode, ctx: &mut BtContext) -> BtStatus {
        match node.condition {
            ConditionKind::CanSeePlayer => {
                if ctx.can_see_player {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            ConditionKind::InAggroRange => {
                if ctx.is_in_aggro_range {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            ConditionKind::InAttackRange => {
                if ctx.is_in_melee_range {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
        }
    }

    fn evaluate_sequence(&self, node: &SequenceNode, ctx: &mut BtContext) -> BtStatus {
        for child in &node.children {
            match self.visit_node(&child, ctx) {
                BtStatus::Success => continue,
                BtStatus::Failure => return BtStatus::Failure,
                BtStatus::Running => return BtStatus::Running,
            }
        }

        BtStatus::Success
    }

    fn evaluate_selector(&self, node: &SelectorNode, ctx: &mut BtContext) -> BtStatus {
        for child in &node.children {
            match self.visit_node(&child, ctx) {
                BtStatus::Success => return BtStatus::Success,
                BtStatus::Failure => continue,
                BtStatus::Running => return BtStatus::Running,
            }
        }

        BtStatus::Success
    }

    pub fn add_node(&mut self, node: BtNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }
}

impl Default for BehaviorTree {
    fn default() -> Self {
        Self {
            root: NodeId(0),
            nodes: Vec::new(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct NodeId(pub usize);

pub enum BtNode {
    Action(ActionNode),
    Selector(SelectorNode),
    Sequence(SequenceNode),
    Condition(ConditionNode),
    // Inverter(InverterNode),
    // RepeatUntilFail(RepeatUntilFailNode),
}

pub struct SequenceNode {
    pub children: Vec<NodeId>,
}

pub struct SelectorNode {
    pub children: Vec<NodeId>,
}

pub struct ActionNode {
    pub action: ActionKind,
}

pub struct ConditionNode {
    pub condition: ConditionKind,
}

#[derive(Debug)]
pub enum ActionKind {
    Idle,
    ChasePlayer,
    AttackPlayer,
}

pub enum ConditionKind {
    CanSeePlayer,
    InAggroRange,
    InAttackRange,
}

pub enum BtStatus {
    Success,
    Failure,
    Running,
}

#[derive(Default, Debug)]
pub struct BtContext {
    pub can_see_player: bool,
    pub was_recently_damaged: bool,
    pub is_in_melee_range: bool,
    pub is_in_aggro_range: bool,

    pub desired_action: Option<ActionKind>,
}
