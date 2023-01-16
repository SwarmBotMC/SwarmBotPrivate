use std::time::Instant;

use crate::client::{
    pathfind::{
        context::{GlobalContext, MoveNode, MoveRecord},
        incremental::{AStar, Node, PathResult},
        moves::Movements,
        traits::{GoalCheck, Heuristic, Progression, Progressor},
    },
    state::{global::GlobalState, local::LocalState},
    timing::Increment,
};

pub mod no_vehicle;

/// A problem that can be solved via BFS
pub trait Problem: Send + Sync {
    /// A node
    type Node: Node;

    /// Try to find a solution until `time`
    fn iterate_until(
        &mut self,
        time: Instant,
        local: &mut LocalState,
        global: &GlobalState,
    ) -> Increment<PathResult<<Self::Node as Node>::Record>>;

    /// recalculate given our new start is `context`
    fn recalc(&mut self, context: Self::Node);
}

pub struct PlayerProblem<H: Heuristic<MoveNode>, G: GoalCheck<MoveNode>> {
    a_star: AStar<MoveNode>,
    heuristic: H,
    goal_checker: G,
}

impl<H: Heuristic<MoveNode> + Send + Sync, G: GoalCheck<MoveNode> + Send + Sync>
    PlayerProblem<H, G>
{
    pub fn new(start: MoveNode, heuristic: H, goal_checker: G) -> Self {
        let a_star = AStar::new(start);
        Self {
            a_star,
            heuristic,
            goal_checker,
        }
    }

    #[allow(unused)]
    pub fn set_max_millis(&mut self, value: u128) {
        self.a_star.set_max_millis(value);
    }
}

#[derive(Copy, Clone)]
struct GenericProgressor<'a> {
    ctx: GlobalContext<'a>,
}

impl Progressor<MoveNode> for GenericProgressor<'_> {
    fn progressions(&self, location: &MoveNode) -> Progression<MoveNode> {
        Movements::new(location, self.ctx).obtain_all()
    }
}

impl<H: Heuristic<MoveNode> + Send + Sync, G: GoalCheck<MoveNode> + Send + Sync> Problem
    for PlayerProblem<H, G>
{
    type Node = MoveNode;

    fn iterate_until(
        &mut self,
        end_at: Instant,
        _: &mut LocalState,
        global: &GlobalState,
    ) -> Increment<PathResult<MoveRecord>> {
        let ctx = GlobalContext {
            path_config: &global.travel_config,
            world: &global.blocks,
        };
        let progressor = GenericProgressor { ctx };
        self.a_star
            .iterate_until(end_at, &self.heuristic, &progressor, &self.goal_checker)
    }

    fn recalc(&mut self, context: Self::Node) {
        self.a_star = AStar::new(context);
    }
}
