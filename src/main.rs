mod components;

use crate::components::*;
use chrono::Utc;
use log::{debug, trace};
use std::collections::hash_map::Values;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    let mut runtime: ServerRuntime = ServerRuntime::new();
    let mut messages: VecDeque<String> = VecDeque::new();
    loop {
        let players = runtime.players.clone();
        let disconnected_players: Vec<&String> = players
            .iter()
            .filter(|entry| {
                entry.1.last_update.timestamp()
                    < Utc::now().timestamp() - Duration::from_secs(6).as_secs() as i64
            })
            .map(|entry| entry.0)
            .collect();
        for player_id in disconnected_players {
            debug!("Removing player: {}", player_id);
            runtime.players.remove(player_id);
        }

        trace!("what is on the stack: {:?}", messages);
        let next_msg = messages.pop_front();
        match next_msg {
            Some(pop_msg) => {
                debug!("pop_front: {}", pop_msg);
                runtime.broadcast(&pop_msg);
            }
            _ => {}
        }
        let handled_msg = runtime.handle_message();
        match handled_msg {
            Some(msg) => {
                trace!("handled_msg to push back on stack: {}", msg);
                messages.push_back(msg);
            }
            _ => {}
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }
}
