mod components;

use crate::components::*;
use log::{debug, trace};
use std::collections::VecDeque;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    let mut runtime: ServerRuntime = ServerRuntime::new();
    let mut messages: VecDeque<String> = VecDeque::new();
    loop {
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
