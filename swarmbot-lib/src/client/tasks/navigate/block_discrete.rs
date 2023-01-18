//! Navigation that is discrete over a block

use std::time::Instant;

use interfaces::types::{BlockLocation, ChunkLocation};

use crate::{
    client::{
        follow::{BlockFollower, Follower, Result},
        pathfind::{
            context::{BlockNode, BotMultithreadedContext},
            implementations::{
                no_vehicle::{BlockGoalCheck, BlockHeuristic, BlockNearGoalCheck, TravelProblem},
                GenericProgressor, Problem, SearchProblem,
            },
            incremental::Node,
            traits::{GoalCheck, Heuristic, Progressor},
        },
        state::{global::GlobalState, local::LocalState},
        tasks::TaskTrait,
        timing::Increment,
    },
    protocol::InterfaceOut,
};

pub struct ChunkTravelTask;

impl ProblemDefinition for ChunkTravelTask {
    type Node = BlockNode;
    type Heuristic = BlockHeuristic;
    type GoalCheck = BlockGoalCheck;
    type Progressor<'a> = GenericProgressor<'a>;
}

pub struct BlockTravelTask;
impl ProblemDefinition for BlockTravelTask {
    type Node = BlockNode;
    type Heuristic = BlockHeuristic;
    type GoalCheck = BlockGoalCheck;
    type Progressor<'a> = GenericProgressor<'a>;
}

pub struct BlockTravelNearTask;
impl ProblemDefinition for BlockTravelNearTask {
    type Node = BlockNode;
    type Heuristic = BlockHeuristic;
    type GoalCheck = BlockNearGoalCheck;
    type Progressor<'a> = GenericProgressor<'a>;
}

impl ChunkTravelTask {
    #[allow(unused)]
    pub fn new(goal: ChunkLocation, local: &LocalState) -> Self {
        let start = local.physics.location().into();
        let problem = TravelProblem::navigate_center_chunk(start, goal);
        problem.into()
    }
}

impl BlockTravelTask {
    pub fn new(goal: BlockLocation, local: &LocalState) -> Self {
        let start = local.physics.location().into();
        let problem = TravelProblem::navigate_block(start, goal);
        problem.into()
    }
}

pub trait ProblemDefinition {
    type Node: Node;
    type Heuristic: Heuristic<Self::Node>;
    type GoalCheck: GoalCheck<Self::Node>;
    type Progressor<'a>: Progressor<Self::Node> + From<BotMultithreadedContext<'a>>;

    /// TODO: this is jank. Should probably be separated
    fn generate_progressor<'a>(
        global: &'a GlobalState,
        local: &'a mut LocalState,
    ) -> Self::Progressor<'a> {
        let context = BotMultithreadedContext { local, global };
        Self::Progressor::from(context)
    }
}

/// wraps a [`SearchProblem`] () and its [`BlockFollower`]
pub struct PerBlockNavigationSystem<P: ProblemDefinition<Node = BlockNode>> {
    calculate: bool,
    problem: Box<SearchProblem<P>>,
    follower: Option<BlockFollower>,
}

impl<P: ProblemDefinition<Node = BlockNode>> From<SearchProblem<P>>
    for PerBlockNavigationSystem<P>
{
    fn from(problem: SearchProblem<P>) -> Self {
        Self {
            calculate: true,
            problem: box problem,
            follower: None,
        }
    }
}

impl<P: ProblemDefinition> TaskTrait for PerBlockNavigationSystem<P> {
    fn tick(
        &mut self,
        _out: &mut impl InterfaceOut,
        local: &mut LocalState,
        global: &mut GlobalState,
    ) -> bool {
        let Some(follower) = self.follower.as_mut() else { return false };

        if follower.should_recalc() {
            println!("recalc");
            self.problem
                .recalc(BlockNode::simple(local.physics.location().into()));
            self.calculate = true;
        }

        match follower.follow_iteration(local, global) {
            Result::Failed => {
                println!("failed");
                self.follower = None;
                self.problem
                    .recalc(BlockNode::simple(local.physics.location().into()));
                self.calculate = true;
                false
            }
            Result::InProgress => false,
            Result::Finished => {
                println!("finished!");
                true
            }
        }
    }

    fn expensive(&mut self, end_at: Instant, local: &mut LocalState, global: &GlobalState) {
        if !self.calculate {
            return;
        }

        let res = self.problem.iterate_until(end_at, local, global);
        match res {
            Increment::Finished(res) => {
                self.calculate = false;
                match self.follower.as_mut() {
                    None => self.follower = BlockFollower::new(res),
                    Some(before) => before.merge(res),
                };
            }

            // Nothing as we are still in progress
            Increment::InProgress => {}
        }
    }
}
