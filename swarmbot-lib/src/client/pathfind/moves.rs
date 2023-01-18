use enum_map::EnumMap;
use interfaces::types::{
    BlockLocation, Change, SimpleType,
    SimpleType::{Avoid, Solid, WalkThrough, Water},
};

use crate::{
    client::pathfind::{
        context::{Costs, GlobalContext, MoveNode},
        moves::centered_arr::CenteredArray,
        traits::{Neighbor, Progression},
    },
    storage::blocks::WorldBlocks,
};

pub const MAX_FALL: i32 = 3;

mod centered_arr;

#[derive(Copy, Clone, Eq, PartialEq)]
enum State {
    Open,
    Closed,
}

impl Default for State {
    fn default() -> Self {
        Self::Open
    }
}

pub struct Movements<'a> {
    on: &'a MoveNode,
    ctx: GlobalContext<'a>,
    current_multiplier: f64,
}

struct Edge;

impl<'a> Movements<'a> {
    pub const fn new(on: &'a MoveNode, ctx: GlobalContext<'a>) -> Self {
        Self {
            on,
            ctx,
            current_multiplier: 1.0,
        }
    }

    /// increase the cost multiplier by the set amount
    fn multiply_all_costs_by(&mut self, amount: f64) {
        self.current_multiplier *= amount;
    }

    /// get a location relative to the start of the movement
    fn loc(&self, dx: i32, dy: i16, dz: i32) -> BlockLocation {
        let BlockLocation { x, y, z } = self.on();
        BlockLocation::new(x + dx, y + dy, z + dz)
    }

    fn move_node(&self) -> MoveNode {
        MoveNode::from(self.on)
    }

    fn wrap(&self, dx: i32, dy: i16, dz: i32) -> MoveNode {
        let loc = self.loc(dx, dy, dz);
        let mut node = MoveNode::from(self.on);
        node.location = loc;
        node
    }

    ///
    const fn on(&self) -> BlockLocation {
        self.on.location
    }

    /// get the cost when factoring in the current multiplier
    #[inline]
    fn cost_of(&self, f: impl FnOnce(&Costs) -> f64) -> f64 {
        f(&self.ctx.path_config.costs) * self.current_multiplier
    }

    #[inline]
    const fn costs(&self) -> &Costs {
        &self.ctx.path_config.costs
    }

    /// get a block relative to the start of the [`Movements`]
    fn get_block(&self, dx: i32, dy: i16, dz: i32) -> Result<SimpleType, Edge> {
        let BlockLocation { x, y, z } = self.on();
        let loc = BlockLocation::new(x + dx, y + dy, z + dz);
        self.ctx.world.get_block_simple(loc).ok_or(Edge)
    }

    /// see which y we would reach if we dropped at a certain location. If we
    /// could not access blocks properly, return [`Edge`].
    fn drop_y(start: BlockLocation, world: &WorldBlocks) -> Result<Option<i16>, Edge> {
        let BlockLocation { x, y: init_y, z } = start;

        // only falling we could do would be into the void
        if init_y < 2 {
            return Ok(None);
        }

        let mut travelled = 1;
        for y in (0..=(init_y - 2)).rev() {
            let loc = BlockLocation::new(x, y, z);
            let block_type = world.get_block_simple(loc).ok_or(Edge)?;
            match block_type {
                Solid => return Ok((travelled <= MAX_FALL).then_some(y)),
                Water => return Ok(Some(y)),
                Avoid => return Ok(None),
                WalkThrough => {}
            }

            travelled += 1;
        }

        Ok(None)
    }

    fn check_head(&mut self) -> Result<SimpleType, Edge> {
        // our current head block
        let head = self.get_block(0, 1, 0)?;

        if head == Water {
            self.multiply_all_costs_by(self.costs().no_breathe_mult);
        }

        Ok(head)
    }

    pub fn obtain_all(self) -> Progression<MoveNode> {
        match self.obtain_all_internal() {
            Ok(elem) => Progression::Movements(elem),
            Err(Edge) => Progression::Edge,
        }
    }

    fn obtain_all_internal(mut self) -> Result<Vec<Neighbor<MoveNode>>, Edge> {
        let head = self.check_head()?;

        // if adj_legs && adj_head is true for any idx
        let mut can_move_adj_no_place = EnumMap::default();

        // cache adjacent leg block types
        let mut adj_legs = EnumMap::default();
        let mut adj_head = EnumMap::default();

        for dir in CardinalDirection::iter() {
            let Change { dx, dz, .. } = dir.unit_change();

            let legs = self.get_block(dx, 0, dz)?;
            adj_legs[dir] = legs;

            let head = self.get_block(dx, 1, dz)?;
            adj_head[dir] = head;

            can_move_adj_no_place[dir] =
                matches!(legs, WalkThrough | Water) && matches!(head, WalkThrough | Water);
        }

        // what we are going to turn for progressions
        let mut res = vec![];

        let mut traverse_possible_no_place = EnumMap::default();

        // moving adjacent without changing elevation
        for dir in CardinalDirection::iter() {
            let Change { dx, dz, .. } = dir.unit_change();

            // we are only looking at the locations that we can walk through
            if can_move_adj_no_place[dir] {
                let floor = self.get_block(dx, 1, dz)?;
                let walkable = floor == Solid || adj_legs[dir] == Water || adj_head[dir] == Water;
                traverse_possible_no_place[dir] = walkable;
                if walkable {
                    res.push(Neighbor {
                        value: self.wrap(dx, 0, dz),
                        cost: self.cost_of(|c| c.block_walk),
                    });
                }
            }
        }

        // descending adjacent
        for dir in CardinalDirection::iter() {
            let Change { dx, dz, .. } = dir.unit_change();

            let floor = self.get_block(dx, -1, dz)?;

            if can_move_adj_no_place[dir] && !traverse_possible_no_place[dir] && floor != Avoid {
                let start = self.loc(dx, 0, dz);
                let collided_y = Self::drop_y(start, self.ctx.world)?;
                if let Some(collided_y) = collided_y {
                    let mut new_pos = self.loc(dx, 0, dz);
                    new_pos.y = collided_y + 1;

                    let mut value = self.move_node();
                    value.location = new_pos;

                    res.push(Neighbor {
                        value,
                        cost: self.cost_of(|c| c.fall),
                    });
                }
            }
        }

        let above = self.get_block(0, 2, 0)?;
        let floor = self.get_block(0, -1, 0)?;
        let feet = self.get_block(0, 0, 0)?;

        if above == Water || head == Water && above == WalkThrough {
            res.push(Neighbor {
                value: self.wrap(0, 1, 0),
                cost: self.cost_of(|c| c.ascend),
            });
        }

        if floor == Water || (floor == WalkThrough && head == Water) {
            res.push(Neighbor {
                value: self.wrap(0, -1, 0),
                cost: self.cost_of(|c| c.ascend),
            });
        }

        let can_micro_jump = above == WalkThrough && (floor == Solid || feet == Water);

        if can_micro_jump {
            // ascending adjacent
            for dir in CardinalDirection::iter() {
                let Change { dx, dz, .. } = dir.unit_change();

                // we can only move if we couldn't move adjacent without changing elevation
                if !can_move_adj_no_place[dir] {
                    let adj_above = matches!(self.get_block(dx, 2, dz)?, WalkThrough | Water);
                    let can_jump = adj_above
                        && adj_legs[dir] == Solid
                        && matches!(adj_head[dir], WalkThrough | Water);
                    if can_jump {
                        res.push(Neighbor {
                            value: self.wrap(dx, 1, dz),
                            cost: self.cost_of(|c| c.ascend),
                        });
                    }
                }
            }
        }

        // can full multi-block jump (i.e., jumping on bedrock)
        let can_jump = above == WalkThrough && floor != Water;

        if can_jump {
            // we can jump in a 3 block radius

            const RADIUS: i32 = 4;
            const RADIUS_S: usize = RADIUS as usize;

            // let mut not_jumpable = SmallVec::<[_; RADIUS_S * RADIUS_S]>::new();
            let mut not_jumpable = Vec::new();

            for dx in -RADIUS..=RADIUS {
                for dz in -RADIUS..=RADIUS {
                    let adj_above = self.get_block(dx, 2, dz)?;

                    let adj_above = adj_above == WalkThrough;
                    let adj_head = self.get_block(dx, 1, dz)? == WalkThrough;
                    let adj_feet = self.get_block(dx, 0, dz)? == WalkThrough;

                    if !(adj_above && adj_head && adj_feet) {
                        not_jumpable.push((dx, dz));
                    }
                }
            }

            let mut open = CenteredArray::init::<_, RADIUS_S>();

            // so we do not add the origin (it is already added)
            open[(0, 0)] = State::Closed;

            // we iterate through every single block which is not jumpable and set blocks
            // behind it as not jumpable as well
            for (block_dx, block_dz) in not_jumpable {
                // we will set blocks to closed in the direction of the block

                let mut update = |sign_x: i32, sign_z: i32| {
                    let increments = RADIUS - block_dx.abs().max(block_dz.abs()) + 1;

                    for inc in 0..increments {
                        let dx = block_dx + inc * sign_x;
                        let dz = block_dz + inc * sign_z;
                        open[(dx, dz)] = State::Closed;
                        if dx.abs() < RADIUS {
                            open[(dx + sign_x, dz)] = State::Closed;
                        }

                        if dz.abs() < RADIUS {
                            open[(dx, dz + sign_z)] = State::Closed;
                        }
                    }
                };

                let sign_x = block_dx.signum();
                let sign_z = block_dz.signum();

                if block_dx == 0 {
                    // special case: we need to update blocks in both directions
                    update(-1, sign_z);
                    update(0, sign_z);
                    update(1, sign_z);
                } else if block_dz == 0 {
                    // special case: we need to update blocks in both directions
                    update(sign_x, -1);
                    update(sign_x, 0);
                    update(sign_x, 1);
                } else {
                    // we only update blocks in the direction it is in
                    update(sign_x, sign_z);
                }
            }

            for dx in -RADIUS..=RADIUS {
                for dz in -RADIUS..=RADIUS {
                    let is_open = open[(dx, dz)] == State::Open;

                    let same_y = self.get_block(dx, -1, dz)?;

                    let same_y_possible = same_y == Solid;

                    let rad2 = f64::from(dx * dx + dz * dz);

                    const MIN_RAD: f64 = 1.1;
                    const MAX_RAD: f64 = 4.5;

                    if same_y_possible
                        && (MIN_RAD * MIN_RAD..=MAX_RAD * MAX_RAD).contains(&rad2)
                        && is_open
                    {
                        res.push(Neighbor {
                            value: self.wrap(dx, 0, dz),
                            cost: self.cost_of(|c| c.block_parkour),
                        });
                    }
                }
            }
        }

        Ok(res)
    }
}

#[derive(Copy, Clone, Debug, enum_map::Enum)]
pub enum CardinalDirection {
    North,
    South,
    West,
    East,
}

impl CardinalDirection {
    fn iter() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter()
    }

    pub const ALL: [Self; 4] = {
        use CardinalDirection::{East, North, South, West};
        [North, South, East, West]
    };
}

impl CardinalDirection {
    pub fn unit_change(self) -> Change {
        match self {
            Self::North => Change::new(1, 0, 0),
            Self::South => Change::new(-1, 0, 0),
            Self::West => Change::new(0, 0, 1),
            Self::East => Change::new(0, 0, -1),
        }
    }
}
