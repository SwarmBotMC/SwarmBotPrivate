/*
 * Copyright (c) 2021 Andrew Gazelka - All Rights Reserved.
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 * Written by Andrew Gazelka <andrew.gazelka@gmail.com>, 6/27/21, 3:15 PM
 */

use std::ops::{IndexMut};

use crate::client::pathfind::context::{GlobalContext, MoveNode};
use crate::client::pathfind::moves::cenetered_arr::CenteredArray;
use crate::client::pathfind::moves::Movements::TraverseCardinal;
use crate::client::pathfind::traits::{Neighbor, Progression};
use crate::storage::block::{BlockLocation, SimpleType};
use crate::storage::blocks::WorldBlocks;
use std::i32::MAX;

pub const MAX_FALL: i32 = 3;

mod cenetered_arr;

enum MoveResult {
    Edge,
    Invalid,
    Realized(Neighbor<BlockLocation>),
}

pub enum Movements {
    TraverseCardinal(CardinalDirection),
}

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

impl Movements {
    const ALL: [Movements; 4] = {
        [
            TraverseCardinal(CardinalDirection::North),
            TraverseCardinal(CardinalDirection::West),
            TraverseCardinal(CardinalDirection::South),
            TraverseCardinal(CardinalDirection::East),
        ]
    };

    pub fn obtain_all(on: &MoveNode, ctx: &GlobalContext) -> Progression<MoveNode> {
        let BlockLocation { x, y, z } = on.location;
        let w = ctx.world;

        macro_rules! get_block {
            ($x: expr, $y: expr, $z:expr) => {{
                let res: Option<SimpleType> = w.get_block_simple(BlockLocation::new($x,$y,$z));
                res
            }};
        }

        macro_rules! wrap {
            ($block_loc: expr) => {{
                let mut node = MoveNode::from(&on);
                node.location = $block_loc;
                node
            }};
        }

        let (head, multiplier) = match get_block!(x, y + 1, z) {
            None => return Progression::Edge,
            Some(inner) => {
                // we do not like our head in water (breathing is nice)
                let multiplier = if inner == Water { ctx.path_config.costs.no_breathe_mult } else { 1.0 };
                (inner, multiplier)
            }
        };


        use crate::storage::block::SimpleType::*;

        // cache adjacent leg block types
        let mut adj_legs = [WalkThrough; 4];
        let mut adj_head = [WalkThrough; 4];

        // if adj_legs && adj_head is true for any idx
        let mut can_move_adj_noplace = [false; 4];

        for (idx, direction) in CardinalDirection::ALL.iter().enumerate() {
            let Change { dx, dz, .. } = direction.unit_change();

            let legs = get_block!(x + dx, y, z + dz);
            let head = get_block!(x + dx, y + 1, z + dz);

            match (legs, head) {
                (Some(legs), Some(head)) => {
                    adj_legs[idx] = legs;
                    adj_head[idx] = head;
                    can_move_adj_noplace[idx] = matches!(legs, WalkThrough | Water) && matches!(head, WalkThrough | Water);
                }
                _ => return Progression::Edge,
            };
        }

        // what we are going to turn for progressoins
        let mut res = vec![];

        let mut traverse_possible_no_place = [false; 4];

        // moving adjacent without changing elevation
        for (idx, direction) in CardinalDirection::ALL.iter().enumerate() {
            let Change { dx, dz, .. } = direction.unit_change();
            if can_move_adj_noplace[idx] {
                let floor = get_block!(x + dx, y - 1, z + dz).unwrap();
                let walkable = floor == Solid || adj_legs[idx] == Water || adj_head[idx] == Water;
                traverse_possible_no_place[idx] = walkable;
                if walkable {
                    res.push(Neighbor {
                        value: wrap!(BlockLocation::new(x + dx, y, z + dz)),
                        cost: ctx.path_config.costs.block_walk * multiplier,
                    })
                }
            }
        }

        // descending adjacent
        for (idx, direction) in CardinalDirection::ALL.iter().enumerate() {
            let Change { dx, dz, .. } = direction.unit_change();

            let floor = get_block!(x + dx, y - 1, z + dz).unwrap();
            if can_move_adj_noplace[idx] && !traverse_possible_no_place[idx] && floor != Avoid {
                let start = BlockLocation::new(x + dx, y, z + dz);
                let collided_y = drop_y(start, w);
                if let Some(collided_y) = collided_y {
                    let new_pos = BlockLocation::new(x + dx, collided_y + 1, z + dz);

                    res.push(Neighbor {
                        value: wrap!(new_pos),
                        cost: ctx.path_config.costs.fall * multiplier,
                    })
                }
            }
        }

        let above = get_block!(x, y + 2, z).unwrap();
        let floor = get_block!(x, y - 1, z).unwrap();

        if above == Water || head == Water {
            res.push(Neighbor {
                value: wrap!(BlockLocation::new(x,y+1,z)),
                cost: ctx.path_config.costs.ascend * multiplier,
            });
        }

        // if it is water the jump will be too high
        let can_jump = above == WalkThrough && floor != Water;

        if can_jump {

            // ascending adjacent
            for (idx, direction) in CardinalDirection::ALL.iter().enumerate() {
                let Change { dx, dz, .. } = direction.unit_change();

                // we can only move if we couldn't move adjacent without changing elevation
                if !can_move_adj_noplace[idx] {
                    let adj_above = get_block!(x+dx, y+2, z+dz).unwrap() == WalkThrough;
                    let can_jump = adj_above && adj_legs[idx] == Solid && adj_head[idx] == WalkThrough;
                    if can_jump {
                        res.push(Neighbor {
                            value: wrap!(BlockLocation::new(x+dx,y+1,z+dz)),
                            cost: ctx.path_config.costs.ascend * multiplier,
                        });
                    }
                }
            }

            // we can jump in a 3 block radius

            const RADIUS: i32 = 4;
            const RADIUS_S: usize = RADIUS as usize;

            // let mut not_jumpable = SmallVec::<[_; RADIUS_S * RADIUS_S]>::new();
            let mut not_jumpable = Vec::new();
            let mut edge = false;


            'check_loop:
            for dx in -RADIUS..=RADIUS {
                for dz in -RADIUS..=RADIUS {
                    let adj_above = get_block!(x+dx, y+2, z+dz);
                    if adj_above == None {
                        edge = true;
                        break 'check_loop;
                    }

                    // if dx.abs() + dz.abs() > RADIUS {
                    //     continue 'check_loop;
                    // }

                    let adj_above = adj_above.unwrap() == WalkThrough;
                    let adj_head = get_block!(x+dx, y+1, z+dz).unwrap() == WalkThrough;
                    let adj_feet = get_block!(x+dx, y, z+dz).unwrap() == WalkThrough;
                    if !(adj_above && adj_head && adj_feet) {
                        not_jumpable.push((dx, dz));
                    }
                }
            }

            if edge {
                return Progression::Edge;
            }


            let mut open = CenteredArray::init::<_, RADIUS_S>();

            // so we do not add the origin (it is already added)
            open[(0, 0)] = State::Closed;

            for (block_dx, block_dz) in not_jumpable {
                if block_dx == 0 {
                    let range = if block_dz > 0 { block_dz..=RADIUS } else { (-RADIUS)..=block_dz };
                    for dz in range {
                        open[(0, dz)] = State::Closed;
                        open[(1, dz)] = State::Closed;
                        open[(-1, dz)] = State::Closed;
                    }
                } else if block_dz == 0 {
                    let range = if block_dx > 0 { block_dx..=RADIUS } else { (-RADIUS)..=block_dx };
                    for dx in range {
                        open[(dx, 0)] = State::Closed;
                        open[(dx, 1)] = State::Closed;
                        open[(dx, -1)] = State::Closed;
                    }
                } else {
                    // we are on a corner
                    let sign_x = block_dx.signum(); // -1
                    let sign_z = block_dz.signum(); // + 1

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
                }
            }

            for dx in -RADIUS..=RADIUS {
                for dz in -RADIUS..=RADIUS {
                    let is_open = open[(dx, dz)] == State::Open;

                    let has_floor = get_block!(x+dx, y - 1, z+dz).unwrap() == Solid;

                    let rad2 = (dx*dx + dz*dz) as f64;

                    const MIN_RAD: f64 = 2.5;
                    const MAX_RAD: f64 = 4.5;

                    if rad2 <= MAX_RAD * MAX_RAD && rad2 > MIN_RAD * MIN_RAD && is_open && has_floor {
                        res.push(Neighbor {
                            value: wrap!(BlockLocation::new(x+dx,y,z+dz)),
                            cost: ctx.path_config.costs.block_walk * multiplier,
                        });
                    }
                }
            }
        }


        Progression::Movements(res)
    }
}

fn drop_y(start: BlockLocation, world: &WorldBlocks) -> Option<i16> {
    let BlockLocation { x, y: init_y, z } = start;

    // only falling we could do would be into the void
    if init_y < 2 {
        return None;
    }

    let mut travelled = 1;
    for y in (0..=(init_y - 2)).rev() {
        let loc = BlockLocation::new(x, y, z);
        let block_type = world.get_block_simple(loc).unwrap();
        match block_type {
            SimpleType::Solid => {
                return (travelled <= MAX_FALL).then(|| y);
            }
            SimpleType::Water => {
                return Some(y);
            }
            SimpleType::Avoid => {
                return None;
            }
            SimpleType::WalkThrough => {}
        }

        travelled += 1;
    }


    None
}

pub enum CardinalDirection {
    North,
    South,
    West,
    East,
}

pub enum CardinalDirection3D {
    Plane(CardinalDirection),
    Up,
    Down,
}

impl CardinalDirection3D {
    pub const ALL: [CardinalDirection3D; 6] = {
        use CardinalDirection::*;
        use CardinalDirection3D::*;
        [
            Plane(North),
            Plane(South),
            Plane(East),
            Plane(West),
            Down,
            Up,
        ]
    };

    pub const ALL_BUT_UP: [CardinalDirection3D; 5] = {
        use CardinalDirection::*;
        use CardinalDirection3D::*;
        [Plane(North), Plane(South), Plane(East), Plane(West), Down]
    };
}

impl CardinalDirection {
    pub const ALL: [CardinalDirection; 4] = {
        use CardinalDirection::*;
        [North, South, East, West]
    };
}

pub struct Change {
    pub dx: i32,
    pub dy: i16,
    pub dz: i32,
}

impl Change {
    fn new(dx: i32, dy: i16, dz: i32) -> Change {
        Change { dx, dy, dz }
    }
}

impl CardinalDirection3D {
    pub fn unit_change(&self) -> Change {
        todo!()
    }
}

impl CardinalDirection {
    fn unit_change(&self) -> Change {
        match self {
            CardinalDirection::North => Change::new(1, 0, 0),
            CardinalDirection::South => Change::new(-1, 0, 0),
            CardinalDirection::West => Change::new(0, 0, 1),
            CardinalDirection::East => Change::new(0, 0, -1)
        }
    }
}
