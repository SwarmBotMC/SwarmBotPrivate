/*
 * Copyright (c) 2021 Andrew Gazelka - All Rights Reserved.
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 * Written by Andrew Gazelka <andrew.gazelka@gmail.com>, 6/29/21, 8:41 PM
 */

use crate::client::bot::{process_command};

use crate::client::state::global::GlobalState;
use crate::client::state::local::{LocalState};
use crate::protocol::{InterfaceOut};
use crate::storage::block::{BlockLocation, BlockState};
use crate::storage::blocks::ChunkLocation;
use crate::storage::chunk::ChunkColumn;
use crate::types::{Chat, Dimension, Location, LocationOrigin};

pub trait InterfaceIn {
    fn on_chat(&mut self, message: Chat);
    fn on_death(&mut self);
    fn on_dimension_change(&mut self, dimension: Dimension);
    fn on_move(&mut self, location: Location);
    fn on_recv_chunk(&mut self, location: ChunkLocation, column: ChunkColumn, new: bool);
    fn on_entity_move(&mut self, id: u32, location: LocationOrigin);
    fn on_block_change(&mut self, location: BlockLocation, state: BlockState);
    fn on_entity_destroy(&mut self, id: u32);
    fn on_entity_spawn(&mut self, id: u32, location: Location);
    fn on_disconnect(&mut self, reason: &str);
    fn on_socket_close(&mut self);
}

pub struct SimpleInterfaceIn<'a, I: InterfaceOut> {
    global: &'a mut GlobalState,
    local: &'a mut LocalState,
    out: &'a mut I,
}

impl<I: InterfaceOut> SimpleInterfaceIn<'a, I> {
    pub fn new(local: &'a mut LocalState, global: &'a mut GlobalState, out: &'a mut I) -> SimpleInterfaceIn<'a, I> {
        SimpleInterfaceIn {
            local,
            global,
            out,
        }
    }
}


impl<'a, I: InterfaceOut> InterfaceIn for SimpleInterfaceIn<'a, I> {
    fn on_chat(&mut self, message: Chat) {
        println!("{}", message.clone().colorize());
        if let Some(msg) = message.player_message() {
            if let Some(cmd) = msg.into_cmd() {
                let name = cmd.command;
                let args_str: Vec<&str> = cmd.args.iter().map(|x| x.as_str()).collect();
                process_command(&name, &args_str, self.local, self.global, self.out);
            }
        }
    }

    fn on_death(&mut self) {
        self.local.follower = None;
        self.local.travel_problem = None;
        self.local.last_problem = None;
        self.out.respawn();
        self.out.send_chat("I died... oof... well I guess I should respawn");
    }

    fn on_dimension_change(&mut self, dimension: Dimension) {
        self.local.dimension = dimension;
    }

    fn on_move(&mut self, location: Location) {
        println!("before loc {}", self.local.physics.location());
        println!("received on move {}", location);
        self.local.physics.teleport(location);
    }

    fn on_recv_chunk(&mut self, location: ChunkLocation, column: ChunkColumn, new: bool) {
        // println!("getting chunk at {}, {}", location.0 << 4, location.1 << 4);
        if new {
            self.global.world_blocks.add_column(location, column);
        } else {
            self.global.world_blocks.modify_column(location, column);
        }
    }

    fn on_entity_move(&mut self, id: u32, location: LocationOrigin) {
        self.global.world_entities.update_entity(id, self.local.bot_id, location);
    }

    fn on_block_change(&mut self, location: BlockLocation, state: BlockState) {
        self.global.world_blocks.set_block(location, state);
    }


    fn on_entity_destroy(&mut self, id: u32) {
        self.global.world_entities.remove_entity(id, self.local.bot_id);
    }

    fn on_entity_spawn(&mut self, id: u32, location: Location) {
        self.global.world_entities.put_entity(id, self.local.bot_id, location);
    }

    fn on_disconnect(&mut self, _reason: &str) {
        self.local.disconnected = true;
    }

    fn on_socket_close(&mut self) {}
}
