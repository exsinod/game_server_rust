mod components;

use crate::components::*;
use chrono::Utc;
use log::{debug, trace};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::runtime::Handle;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use unqlite::UnQLite;

static LOOP_DURATION: Duration = Duration::new(0, 1_000_000_000u32 / 20);

// static RECV_SOCKET_PORT: u16 = 8877;
// static SEND_SOCKET_PORT: u16 = 8878;
// static SEND_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];
// static RECV_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];

// type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    env_logger::init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .worker_threads(10)
        .build()
        .unwrap();
    let rt = runtime.handle();
    rt.block_on(async move {
        let (tx_game_command, rx_game_command) = broadcast::channel::<(SocketAddr, GameCommand)>(200);
        let (tx_outgoing, _) = broadcast::channel::<(SocketAddr, String)>(200);
        let (tx_send_addr, mut rx_send_addr) = mpsc::channel::<(String, SocketAddr)>(200);

        let send_socket_task = rt.spawn(async move {
            let mut addresses: HashSet<SocketAddr> = HashSet::new();
            let send_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 8878)))
                .await
                .unwrap();
            trace!("spawning send socket thread");
            loop {
                match rx_send_addr.try_recv() {
                    Ok((command, addr)) => match command.as_str() {
                        "ADD" => {
                            addresses.insert(addr);
                        }
                        _ => {}
                    },
                    Err(_error) => {}
                }
                let game_state = GameCommandProcessor::new().get_current_game_state();
                addresses.iter().for_each(|addr| {
                    trace!(
                        "sending game state: {} on udp socket addr: {}",
                        game_state,
                        addr.to_string()
                    );
                    
                    send_socket
                        .try_send_to(game_state.as_bytes(), *addr)
                        .unwrap();
                });
                ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
            }
        });

        let (one, two, three) = tokio::join! { rt.spawn(recv_client_input_task(tx_game_command, tx_send_addr)), send_socket_task, rt.spawn(process_game_state(rx_game_command, tx_outgoing)) };
    });
}

async fn process_game_state(
    mut rx_game_command: Receiver<(SocketAddr, GameCommand)>,
    tx_outgoing: Sender<(SocketAddr, String)>,
) {
    trace!("spawning game state receiver thread");

    loop {
        if !rx_game_command.is_empty() {
            match rx_game_command.recv().await {
                Ok((src, game_command)) => {
                    let mut game_command_processor = GameCommandProcessor::new();
                    debug!("Processing received command: {:?}", game_command);
                    let game_state = game_command_processor.process(game_command);
                    // debug!("New game state: {}", game_state);
                    // match tx_outgoing.send((src, game_state)) {
                    //     Ok(_) => {}
                    //     Err(error) => {
                    //         trace!("{error}")
                    //     }
                    // }
                }
                Err(error) => {
                    trace!("Nothing to handle off rx_game_command: {}", error)
                }
            }
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }
}

async fn recv_client_input_task(
    tx_game_command: Sender<(SocketAddr, GameCommand)>,
    tx_send_addr: mpsc::Sender<(String, SocketAddr)>,
) {
    trace!("spawning client receiver thread");
    let message_receiver: MessageReceiver = MessageReceiver::new();
    loop {
        match message_receiver.recv() {
            Some((src, game_command)) => {
                trace!("Received message: {:?}", game_command);
                tx_send_addr.send(("ADD".to_string(), src)).await.unwrap();
                match tx_game_command.send((src, game_command)) {
                    Ok(result) => {
                        trace!("Sent received message {result} times.");
                    }
                    Err(error) => {
                        trace!("Error sending received message: {}", error);
                    }
                }
                //     if !clients.contains_key(&src.to_string()) {
                //         trace!("yeah no key here");
                //         tokio::spawn(async move {
                //             trace!("Starting send thread for src: {}", src);
                //             loop {
                //                 if !tx_out_sub.is_empty() {
                //                     match tx_out_sub.recv().await {
                //                         Ok((src, game_command)) => {
                //                             trace!(
                //                                 "previous send: {} now: {}, loop duration: {}",
                //                                 previous_send,
                //                                 Utc::now().timestamp(),
                //                                 LOOP_DURATION.as_millis()
                //                             );
                //                             if previous_send
                //                                 < Utc::now().timestamp_millis()
                //                                     - LOOP_DURATION.as_millis() as i64
                //                             {
                //                                 trace!("sending to socket");
                //                                 let send_addr =
                //                                     SocketAddr::from((src.ip(), src.port() + 1));
                //                                 match tx_send_to_socket
                //                                     .send((send_addr, game_command))
                //                                     .await
                //                                 {
                //                                     Ok(_) => {
                //                                         trace!("send to socket success")
                //                                     }
                //                                     Err(error) => {
                //                                         trace!("send to socket error: {error}")
                //                                     }
                //                                 }
                //                                 previous_send = Utc::now().timestamp_millis();
                //                             }
                //                         }
                //                         Err(error) => {
                //                             trace!("Nothing to handle off rx_game_state: {}", error)
                //                         }
                //                     }
                //                 }
                //                 ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
                //             }
                //         });
                //         clients.insert(src.to_string(), src.to_string());
                //     }
            }
            None => {}
        };
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }
}
