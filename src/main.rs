mod components;
mod processor;
mod receiver;
mod sender;
mod syncer;

use crate::processor::start_game_command_processor;
use crate::receiver::recv_client_input_task;
use crate::sender::send_socket_task;
use crate::{components::*, syncer::recv_sync_task};

use std::net::SocketAddr;
use tokio::sync::mpsc::{self, channel};

// static LOOP_DURATION: Duration = Duration::new(0, 1_000_000_000u32 / 20);

fn main() {
    env_logger::init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .worker_threads(10)
        .build()
        .unwrap();
    let rt = runtime.handle();
    rt.block_on(async move {
        let (tx_process, rx_process) = channel::<GameCommand>(200);
        let (tx_send_addr, rx_send_addr) = mpsc::channel::<(String, String, SocketAddr)>(200);

        let (_, _, _, _) = tokio::join! { rt.spawn(recv_sync_task()), rt.spawn(start_game_command_processor(rx_process)), rt.spawn(recv_client_input_task(tx_process, tx_send_addr)), rt.spawn(send_socket_task(rx_send_addr)) };
    });
}
