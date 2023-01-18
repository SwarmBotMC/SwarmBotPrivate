use std::time::Instant;

use crate::client::{
    pathfind::{
        context::{BlockNode, BotMultithreadedContext, GlobalContext},
        incremental::{AStar, Node, PathResult},
        moves::Movements,
        traits::{Heuristic, Progression, Progressor},
    },
    state::{global::GlobalState, local::LocalState},
    tasks::navigate::ProblemDefinition,
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

/// A problem that is defined as searchable by [`AStar`] with nodes being a
/// discrete over block [`BlockNode`].
pub struct SearchProblem<P: ProblemDefinition> {
    problem: P,
    a_star: AStar<P::Node>,
    heuristic: P::Heuristic,
    goal_checker: P::GoalCheck,
}

impl<P: ProblemDefinition> SearchProblem<P> {
    pub fn new(
        problem: P,
        start: P::Node,
        heuristic: P::Heuristic,
        goal_checker: P::GoalCheck,
    ) -> Self {
        let a_star = AStar::new(start);
        Self {
            problem,
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
pub struct GenericProgressor<'a> {
    ctx: GlobalContext<'a>,
}

impl<'a> From<BotMultithreadedContext<'a>> for GenericProgressor<'a> {
    fn from(value: BotMultithreadedContext<'a>) -> Self {
        Self {
            ctx: GlobalContext {
                path_config: &value.global.travel_config,
                world: &value.global.blocks,
            },
        }
    }
}

impl Progressor<BlockNode> for GenericProgressor<'_> {
    fn progressions(&self, location: &BlockNode) -> Progression<BlockNode> {
        Movements::new(location, self.ctx).obtain_all()
    }
}

impl<P: ProblemDefinition> Problem for SearchProblem<P> {
    type Node = P::Node;

    fn iterate_until(
        &mut self,
        end_at: Instant,
        local: &mut LocalState,
        global: &GlobalState,
    ) -> Increment<PathResult<<P::Node as Node>::Record>> {
        let progressor = self.progressor.generate(global, local);
        self.a_star
            .iterate_until(end_at, &self.heuristic, &progressor, &self.goal_checker)
    }

    fn recalc(&mut self, context: Self::Node) {
        self.a_star = AStar::new(context);
    }
}
