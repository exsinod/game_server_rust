use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use chrono::Utc;
use log::{debug, trace};
use tokio::sync::mpsc::{self, Sender};

use crate::components::{
    CommandContext, CommandType, GameCommand, LoginCommandContext, MoveCommandContext,
};

static RECV_SOCKET_PORT: u16 = 8877;
static RECV_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];

pub struct MessageReceiver {
    recv_socket: UdpSocket,
}
impl MessageReceiver {
    pub fn new() -> Self {
        let recv_socket =
            UdpSocket::bind(SocketAddr::from((RECV_SERVER_ADDR, RECV_SOCKET_PORT))).unwrap();
        recv_socket
            .set_read_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        recv_socket
            .set_write_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        recv_socket.set_nonblocking(false).unwrap();
        Self { recv_socket }
    }

    pub fn recv(&self) -> Option<(SocketAddr, GameCommand)> {
        let mut buf = [0; 128];
        match self.recv_socket.recv_from(&mut buf) {
            Ok((number_of_bytes, src)) => {
                if number_of_bytes > 1 {
                    match Self::return_game_command_if_recent(&buf, number_of_bytes) {
                        Some((timestamp, command, context)) => {
                            debug!(
                                "MessageReceiver: Command: {}, Command context: {:?}",
                                command, context
                            );
                            let data_vec: Vec<String> = context
                                .split(";")
                                .map(|dat| dat.to_string())
                                .collect::<Vec<String>>();
                            let command_context: CommandContext;
                            let game_command: Option<(SocketAddr, GameCommand)> = match command
                                .as_str()
                            {
                                "L1;" => {
                                    command_context = LoginCommandContext::from_login_cmd(data_vec);

                                    return Some((
                                        src,
                                        GameCommand::from(
                                            timestamp,
                                            CommandType::LoginCommand,
                                            command_context,
                                        ),
                                    ));
                                }
                                "M0;" => {
                                    command_context = MoveCommandContext::from_move_cmd(data_vec);
                                    let command: CommandType;
                                    if command_context.direction == 4 {
                                        command = CommandType::StopCommand;
                                    } else {
                                        command = CommandType::MoveCommand;
                                    }
                                    return Some((
                                        src,
                                        GameCommand::from(timestamp, command, command_context),
                                    ));
                                }
                                _ => None,
                            };
                            return game_command;
                        }
                        None => {}
                    }
                }
            }
            Err(_) => {}
        }
        None
    }

    fn return_game_command_if_recent(buffer: &[u8], size: usize) -> Option<(i64, String, String)> {
        // take a big enough slice to be safe
        let timestamp = std::str::from_utf8(&buffer[0..20]).unwrap();
        let timestamp = &timestamp
            .split(";")
            .map(|part| String::from_utf8(part.into()).unwrap())
            .collect::<Vec<String>>()[0];
        // println!(
        //     "handling timestamp: {timestamp} | current timestamp: {}",
        //     Utc::now().timestamp()
        // );
        let timestamp = i64::from_str_radix(timestamp, 10).unwrap();
        if timestamp > Utc::now().timestamp() - 100 {
            let bla = &buffer[(timestamp.to_string().len() + 1)..(size)];
            trace!("returning {:?}", String::from_utf8(bla.into()));
            Some((
                timestamp,
                Self::get_operation_from(&bla).to_string(),
                Self::get_context_from(&bla).to_string(),
            ))
        } else {
            None
        }
    }

    fn get_operation_from(buffer: &[u8]) -> &str {
        std::str::from_utf8(&buffer[0..3]).unwrap()
    }

    fn get_context_from(buffer: &[u8]) -> &str {
        std::str::from_utf8(&buffer[3..]).unwrap_or("no context")
    }
}

pub async fn recv_client_input_task(
    tx_process: Sender<GameCommand>,
    tx_send_addr: mpsc::Sender<(String, String, SocketAddr)>,
) {
    trace!("spawning client receiver thread");
    let message_receiver = MessageReceiver::new();

    loop {
        let tx_send_addr = tx_send_addr.clone();
        match message_receiver.recv() {
            Some((src, game_command)) => {
                trace!("Received game command from client: {:?}", game_command);
                let send_addr = SocketAddr::from((src.ip(), src.port() + 1));
                tx_send_addr
                    .send((
                        "ADD".to_string(),
                        game_command.context.player_id.clone(),
                        send_addr,
                    ))
                    .await
                    .unwrap();
                println!("send {} to tx_game_command", game_command.timestamp);
                tx_process.try_send(game_command).unwrap();
            }
            None => {}
        }
    }
}
