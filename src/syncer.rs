use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use chrono::Utc;
use log::{debug, trace};
use tokio::sync::mpsc::{self, Sender};

use crate::{
    components::{
        CommandContext, CommandType, GameCommand, LoginCommandContext, MoveCommandContext, Point,
    },
    processor::GameCommandProcessor,
};

static RECV_SOCKET_PORT: u16 = 8877;
static RECV_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];

pub async fn recv_sync_task() {
    trace!("spawning syncer thread");
    let mut game_command_processor: GameCommandProcessor = GameCommandProcessor::new();
    let recv_socket = UdpSocket::bind(SocketAddr::from((RECV_SERVER_ADDR, 8866))).unwrap();
    recv_socket
        .set_read_timeout(Some(Duration::new(0, 1000)))
        .unwrap();
    recv_socket
        .set_write_timeout(Some(Duration::new(0, 1000)))
        .unwrap();
    recv_socket.set_nonblocking(false).unwrap();

    loop {
        let mut buf = [0; 128];
        match recv_socket.recv(&mut buf) {
            // format is S0;player_id:{x: 0, y: 0}
            Ok(number_of_bytes) => {
                let sync_cmd = get_context_from(&buf[..number_of_bytes]).to_string();
                let sync_cmd = sync_cmd
                    .split(";")
                    .map(|s| String::from(s))
                    .collect::<Vec<String>>();
                println!("syncing {sync_cmd:?}");
                println!("syncing {:?}", sync_cmd[3]);
                println!("syncing {:?}", &sync_cmd[3]);
                println!("syncing blalblabl");
                let pos: Point = serde_json::from_str::<Point>(&sync_cmd[3]).unwrap();
                let player_id = &sync_cmd[2];
                println!("syncinga {:?}", pos);
                println!("syncinga {:?}", player_id);
                game_command_processor.update_pos(player_id.to_string(), pos)
            }
            Err(error) => {}
        }
    }
}
fn get_operation_from(buffer: &[u8]) -> &str {
    std::str::from_utf8(&buffer[0..3]).unwrap()
}

fn get_context_from(buffer: &[u8]) -> &str {
    std::str::from_utf8(&buffer[3..]).unwrap_or("no context")
}
