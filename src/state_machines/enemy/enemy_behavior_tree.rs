use serde::{Deserialize, Serialize};

use crate::config::Config;

use rand::seq::SliceRandom;

#[derive(Clone, Deserialize, Serialize)]
pub struct BehaviorTree {
    pub root: NodeId,
    pub nodes: Vec<BtNode>,
}

impl BehaviorTree {
    pub fn new(kind: &str) -> Self {
        match kind {
            "melee" => BehaviorTree::load_from_file("config/enemy_bt_melee.json"),
            "ranged" => BehaviorTree::load_from_file("config/enemy_bt_ranged.json"),
            _ => {
                panic!("{} is not a valid behavior tree type", kind);
            }
        }
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
            BtNode::RandomSelector(node) => self.evaluate_random_selector(node, ctx),
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
            ActionKind::Block => {
                ctx.desired_action = Some(ActionKind::Block);
                BtStatus::Running
            }
            ActionKind::Dodge => {
                ctx.desired_action = Some(ActionKind::Dodge);
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
            ConditionKind::InMeleeRange => {
                if ctx.is_in_melee_range {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            ConditionKind::InProjectileRange => {
                if ctx.is_in_projectile_range {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            ConditionKind::PlayerIsAttacking => {
                if ctx.player_is_attacking {
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

        BtStatus::Failure
    }

    fn evaluate_random_selector(&self, node: &SelectorNode, ctx: &mut BtContext) -> BtStatus {
        let mut children = node.children.clone();

        let mut rng = rand::rng();
        children.shuffle(&mut rng);

        for child in &children {
            match self.visit_node(&child, ctx) {
                BtStatus::Success => return BtStatus::Success,
                BtStatus::Failure => continue,
                BtStatus::Running => return BtStatus::Running,
            }
        }

        BtStatus::Failure
    }

    #[allow(dead_code)]
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

impl Config for BehaviorTree {}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct NodeId(pub usize);

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "node_type")]
pub enum BtNode {
    Action(ActionNode),
    Selector(SelectorNode),
    Sequence(SequenceNode),
    Condition(ConditionNode),
    RandomSelector(SelectorNode),
    // Inverter(InverterNode),
    // RepeatUntilFail(RepeatUntilFailNode),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SequenceNode {
    pub children: Vec<NodeId>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SelectorNode {
    pub children: Vec<NodeId>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ActionNode {
    pub action: ActionKind,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ConditionNode {
    pub condition: ConditionKind,
}

#[derive(Debug, Deserialize, Clone, Serialize, Copy, PartialEq)]
pub enum ActionKind {
    Idle,
    ChasePlayer,
    AttackPlayer,
    Block,
    Dodge,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub enum ConditionKind {
    CanSeePlayer,
    InAggroRange,
    InMeleeRange,
    InProjectileRange,
    PlayerIsAttacking,
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
    pub is_in_projectile_range: bool,
    pub player_is_attacking: bool,

    pub desired_action: Option<ActionKind>,
}
