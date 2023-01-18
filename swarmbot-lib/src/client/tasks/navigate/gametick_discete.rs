//! Searches that are discrete over a gametick

use interfaces::types::{Direction, Location};
use num::complex::ComplexFloat;
use smallvec::{smallvec, SmallVec};
use test::stats::Stats;

use crate::client::{
    pathfind::{
        context::BlockNode,
        implementations::SearchProblem,
        traits::{GoalCheck, Heuristic, Progression, Progressor},
    },
    physics::{speed::MovementSpeed, Physics, WalkDirection},
    tasks::navigate::ProblemDefinition,
};

struct PhysicsHeuristic {
    goal: Location,
}

impl Heuristic<Physics> for PhysicsHeuristic {
    fn heuristic(&self, input: &Physics) -> f64 {
        let Location {
            x: dx,
            y: dy,
            z: dz,
        } = self.goal - input.location();

        // admissible heuristic1
        let heuristic1 = {
            /// to make heuristic admissible we will be conservative
            /// We are rounding the value `7.127` from
            /// <https://minecraft.fandom.com/wiki/Sprinting>
            const MAX_HORIZONTAL_BLOCKS_PER_TICK: f64 = 8.0 / 20.0; // = 0.4

            /// the minimum number of ticks we need for each block
            const MIN_TICKS_PER_BLOCK: f64 = 1.0 / MAX_HORIZONTAL_BLOCKS_PER_TICK;

            let distance_horizontal = (dx * dx + dz * dz).sqrt();

            distance_horizontal * MIN_TICKS_PER_BLOCK
        };

        // generally admissible heuristic2
        let heuristic2 = {
            /// max vertical velocity is `0.42`, but since we can't jump every
            /// tick we are going to be more liberal. Ladder speed,
            /// `2.35 / 20 ≈ 0.12` anyway.
            const MAX_BLOCKS_PER_TICK_CLIMB: f64 = 0.20;

            const MIN_TICKS_PER_BLOCK_CLIMB: f64 = 1.0 / MAX_BLOCKS_PER_TICK_CLIMB;

            /// max blocks per tick free fall (roughly)
            /// <https://gaming.stackexchange.com/a/178730/235703>
            ///
            /// Max is actually 3.92 blocks per tick but is less than 3.0
            /// after 3.45 seconds
            const MAX_BLOCKS_PER_TICK_FREE_FALL: f64 = 3.0;

            /// the minimum number of ticks we need for each block in free fall
            const MIN_TICKS_PER_BLOCK_FREE_FALL: f64 = 1.0 / MAX_BLOCKS_PER_TICK_FREE_FALL;

            if dy > 0.0 {
                dy * MIN_TICKS_PER_BLOCK_CLIMB
            } else {
                (-dy) * MIN_TICKS_PER_BLOCK_FREE_FALL
            }
        };

        // TODO: add heuristic which factors in water

        // we know it is admissible if both are
        [heuristic1, heuristic2].max()
    }
}

pub struct PhysicsGoalCheck {
    goal: Location,
}

impl GoalCheck<Physics> for PhysicsGoalCheck {
    fn is_goal(&self, input: &Physics) -> bool {
        const THRESHOLD: f64 = 0.1;
        let d2 = input.location().dist2(self.goal);

        d2 < THRESHOLD * THRESHOLD
    }
}

/// angles discretized
fn discretized_angles(angle: Direction) -> impl Iterator<Item = Direction> {
    let horizontal = angle.as_horizontal();

    // degrees
    let degrees = [0.0, -6.6, 20.0, -60.0, 180.0];

    degrees.into_iter().map(|deg| horizontal.turn_degrees(deg))
}

struct PhysicsProgressor;
impl Progressor<Physics> for PhysicsProgressor {
    type Iter = impl IntoIterator<Item = Physics>;

    fn progressions(&self, input: &Physics) -> Progression<BlockNode> {
        // 5
        let angles = discretized_angles(input.direction());

        // 2
        let walk_directions = [WalkDirection::Backward, WalkDirection::Forward];

        // 2
        let jump: SmallVec<[_; 2]> = if input.on_ground() {
            smallvec![true, false]
        } else {
            smallvec![false]
        };

        // 2
        let speeds = [MovementSpeed::SPRINT, MovementSpeed::WALK];

        // 40 nodes

        for res in itertools::izip!(angles, walk_directions, jump, speeds) {
            let res: (Direction, WalkDirection, bool, MovementSpeed) = res;
            let (angle, dir, jump, speed) = res;

            Physics {
                location: Default::default(),
                look: Default::default(),
                prev: Default::default(),
                horizontal: Default::default(),
                pending: Default::default(),
                in_water: false,
            }
        }

        todo!()
    }
}

pub struct BlockTravelTask;
impl ProblemDefinition for BlockTravelTask {
    type Node = Physics;
    type Heuristic = PhysicsHeuristic;
    type GoalCheck = PhysicsGoalCheck;
    type Progressor<'a> = GenericProgressor<'a>;
}

struct GametickNavigateProblem<H: Heuristic<Physics>, G: GoalCheck<Physics>> {
    calculate: bool,
    problem: Box<SearchProblem<Physics, H, G>>,
}
