//! global context information that is applicable to all bots
use std::hash::{Hash, Hasher};

use interfaces::types::BlockLocation;

use crate::{
    client::{
        pathfind::incremental::Node,
        state::{global::GlobalState, local::LocalState},
    },
    storage::blocks::WorldBlocks,
};

/// The costs of doing everything (used for pathfinding)
#[derive(Clone)]
pub struct Costs {
    /// cost to walk one block
    pub block_walk: f64,
    /// cost to parkour one block
    pub block_parkour: f64,

    /// cost to mine an unrelated block to achieve a goal
    pub mine_unrelated: f64,

    /// cost to mine a required block
    pub mine_required: f64,

    /// cost to place an unrelated block
    pub place_unrelated: f64,

    /// cost to place a required block
    pub place_required: f64,

    /// cost to ascend a block (for instance in a ladder)
    pub ascend: f64,

    /// the cost multiplier for not being able to breathe (we don't want to
    /// drown!)
    pub no_breathe_mult: f64,

    /// the cost of falling (without taking damage)
    pub fall: f64,
}

/// The configuration for finding paths
pub struct PathConfig {
    /// The [`Costs`] config
    pub costs: Costs,
    /// If the bot can do parkour
    pub parkour: bool,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            costs: Costs {
                block_walk: 1.0,
                block_parkour: 1.5,
                mine_unrelated: 20.0,
                ascend: 1.0,
                no_breathe_mult: 3.0,
                fall: 1.0,
                place_unrelated: 20.0,
                mine_required: 1.0,
                place_required: 1.0,
            },
            parkour: true,
        }
    }
}

/// The global context for path traversal
#[derive(Copy, Clone)]
pub struct BotMultithreadedContext<'a> {
    /// the local state (mutable)
    pub local: &'a mut LocalState,

    /// the global state (immutable) as it is shared cross-thread
    pub global: &'a GlobalState,
}

/// The global context for path traversal
#[derive(Copy, Clone)]
pub struct GlobalContext<'a> {
    /// the costs and path configuration values
    pub path_config: &'a PathConfig,

    /// the state of the world blocks
    pub world: &'a WorldBlocks,
}

/// A node which represents a movement
#[derive(Debug)]
pub struct BlockNode {
    /// The current location of the user
    pub location: BlockLocation,
}

impl BlockNode {
    /// A simple movement to `location`
    pub const fn simple(location: BlockLocation) -> Self {
        Self { location }
    }

    /// TODO: what
    pub fn from(previous: &Self) -> Self {
        previous.clone()
    }
}

impl Clone for BlockNode {
    fn clone(&self) -> Self {
        Self {
            location: self.location,
        }
    }
}

impl Node for BlockNode {
    type Record = BlockRecord;

    fn get_record(&self) -> Self::Record {
        let &Self { location, .. } = self;

        let state = MoveState { location };

        Self::Record { state }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct MoveState {
    pub location: BlockLocation,
}

#[derive(Clone, Debug)]
pub struct BlockRecord {
    pub state: MoveState,
}

impl PartialEq for BlockRecord {
    fn eq(&self, other: &Self) -> bool {
        self.state.eq(&other.state)
    }
}

impl Eq for BlockRecord {}

impl Hash for BlockRecord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.hash(state);
    }
}
