mod components;

use crate::components::*;
use chrono::Utc;
use log::{debug, error, trace};
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::ops::Add;
use std::process::exit;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use unqlite::{Cursor, UnQLite, KV};

static RECV_SOCKET_PORT: u16 = 8877;
static SEND_SOCKET_PORT: u16 = 8878;
static SEND_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];
static RECV_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    env_logger::init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .worker_threads(10)
        .build()
        .unwrap();
    let rt = runtime.handle();
    rt.block_on(async move {
        // let mut send_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 8878)))
        // UdpSocket::bind(SocketAddr::from((src.ip(), send_port)))
        // .await
        // .expect("Cannot bind to server send port");
        let mut messages: VecDeque<String> = VecDeque::new();
        let channels_per_src: HashMap<String, String> = HashMap::new();
        let (tx_game_state, _) = broadcast::channel::<(SocketAddr, GameCommand)>(200);
        let (tx_incoming, _) = broadcast::channel::<String>(200);
        let (tx_outgoing, _) = broadcast::channel::<(SocketAddr, String)>(200);
        let mut tx_outgoing_subscriber = tx_outgoing.subscribe();
        trace!("1");
        // tokio::spawn(async move {
        //     let send_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 8878)))
        //         // UdpSocket::bind(SocketAddr::from((src.ip(), send_port)))
        //         .await
        //         .expect("Cannot bind to server send port");
        //     loop {
        //         match tx_outgoing_subscriber.recv().await {
        //             Ok((src, msg)) => {
        //                 println!("received {msg}");
        //                 let send_port = src.port() + 1;
        //                 match send_socket
        //                     .send_to(msg.as_bytes(), SocketAddr::from((src.ip(), send_port)))
        //                     .await
        //                 {
        //                     Ok(_) => {}
        //                     Err(error) => {
        //                         trace!("cannot send msg to client: {error}")
        //                     }
        //                 }
        //                 // .expect(&format!("Could not send message to {src}"));
        //                 // UdpSocket::bind()
        //             }
        //             Err(_) => {}
        //         }
        //     }
        // });
        // loop {
        // handle_game_command(&nosql, GameCommand::new()).await;
        //
        // trace!("Players: {:?}", runtime.players);
        // trace!("Addrs: {:?}", runtime.addrs);
        // runtime
        //     .players
        //     .clone()
        //     .iter()
        //     .filter(|entry| {
        //         entry.1.last_update
        //             < Utc::now().timestamp() - Duration::from_secs(6).as_secs() as i64
        //     })
        //     .for_each(|player_entry| {
        //         trace!("removing {:?}", player_entry);
        //         runtime.addrs.remove(player_entry.0);
        //         runtime.players.remove(&player_entry.1.id);
        //     });
        //
        // let active_observers: Vec<SocketAddr> = runtime
        //     .observers
        //     .iter()
        //     .map(|observer_src_addr| {
        //         let send_port = observer_src_addr.port() + 1;
        //         SocketAddr::from((observer_src_addr.ip(), send_port))
        //     })
        //     .collect::<Vec<SocketAddr>>();
        //
        // let active_players: Vec<SocketAddr> = runtime
        //     .players
        //     .iter()
        //     .filter(|entry| {
        //         entry.1.last_update
        //             > Utc::now().timestamp() - Duration::from_secs(6).as_secs() as i64
        //     })
        //     .map(|player| {
        //         let decoded_src = base64::decode(player.0).unwrap();
        //         trace!("decoded_src: {:?}", decoded_src);
        //         let decoded_src = std::str::from_utf8(&decoded_src).unwrap();
        //         trace!("decoded_src: {:?}", decoded_src);
        //         let decoded_src_parts = decoded_src.split(":").collect::<Vec<_>>();
        //         let recv_port = u16::from_str_radix(decoded_src_parts[1], 10).unwrap();
        //         let send_port = recv_port + 1;
        //         match runtime.addrs.get(player.0) {
        //             Some(addr) => Some(SocketAddr::from((*addr, send_port))),
        //             None => {
        //                 runtime.addrs.remove(player.0);
        //                 None
        //             }
        //         }
        //     })
        //     .filter(|option| option.is_some())
        //     .map(|socket_addr| socket_addr.unwrap())
        //     .into_iter()
        //     .collect::<Vec<SocketAddr>>();

        // trace!("Active players: {:?}", active_players);
        // trace!("what is on the stack: {:?}", messages);
        // let handled_msg = runtime.handle_message();
        // let btx_incoming = tx_incoming.clone();
        trace!("2");
        let mut rx_gs = tx_game_state.subscribe();
        let tx_outgoing_clone = tx_outgoing.clone();
        let mut tx_out_sub = tx_outgoing_clone.subscribe();
        let send_to_client_task = rt.spawn(async move {
            trace!("starting send thread");
            let send_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 8878)))
                .await
                .unwrap();
            loop {
                if !tx_out_sub.is_empty() {
                    match tx_out_sub.recv().await {
                        Ok((src, game_command)) => {
                            trace!("sending to socket");
                            let send_addr = SocketAddr::from((src.ip(), src.port() + 1));
                            match send_socket
                                .send_to(game_command.to_string().as_bytes(), send_addr)
                                .await
                            {
                                Ok(_) => {}
                                Err(error) => {
                                    trace!("send to socket error: {error}")
                                }
                            }
                        }
                        Err(error) => {
                            trace!("Nothing to handle off rx_game_state: {}", error)
                        }
                    }
                }
                ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
            }
        });

        let recv_game_state_task = rt.spawn(async move {
            trace!("spawning game state receiver thread");
            let mut game_command_processor = GameCommandProcessor::new();
            loop {
                // trace!("looping in rx_game_state");
                if !rx_gs.is_empty() {
                    match rx_gs.recv().await {
                        Ok((src, game_command)) => {
                            debug!("rx_game_state received: {:?}", game_command);
                            let game_state = game_command_processor.process(game_command);
                            debug!("game state: {}", game_state);
                            match tx_outgoing_clone.send((src, game_state)) {
                                Ok(_) => {
                                    trace!("sent outgoing")
                                }
                                Err(error) => {
                                    trace!("{error}")
                                }
                            }
                        }
                        Err(error) => {
                            trace!("Nothing to handle off rx_game_state: {}", error)
                        }
                    }
                }
                ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
            }
        });

        trace!("3");
        let recv_client_input_task = rt.spawn(async move {
            // let mut incoming_subscriber = tx_incoming.subscribe();
            trace!("spawning client receiver thread");
            let message_receiver: MessageReceiver = MessageReceiver::new();
            loop {
                match message_receiver.recv() {
                    Some((src, game_command)) => {
                        trace!("sending {:?} to tx", game_command);
                        match tx_game_state.send((src, game_command)) {
                            Ok(result) => {
                                debug!("sent {result} times")
                            }
                            Err(error) => {
                                trace!("error sending: {}", error)
                            }
                        }
                        // todo!("send this game command to a channel which will spawn a thread for every command received. Then it will send the game state to all subscribers of the channel responsible for sending the game state to clients.");
                        // tokio::spawn(async move {
                        //     let game_status = GameCommandProcessor::process(game_command);
                        //     // tx_incoming.send(game_state);
                        // });
                    } // match handled_msg {
                    None => {} //     Some((msg, src)) => {
                               //         trace!("handled_msg to push back on stack: {}", msg);
                               //         trace!("src key: {}", &src.to_string());
                               //         trace!(
                               //             "channels_per_src get: {}",
                               //             channels_per_src
                               //                 .get(&src.to_string())
                               //                 .unwrap_or(&"Yeah no".to_string())
                               //         );
                               //         if !channels_per_src.contains_key(&src.to_string()) {
                               //             channels_per_src.insert(src.to_string(), String::from("yes"));
                               //             trace!("src {src} added to channels");
                               //             trace!("Creating new thread for {src}");
                               //             let send_port = src.port() + 1;
                               //             // let send_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 8878)))
                               //             //     // UdpSocket::bind(SocketAddr::from((src.ip(), send_port)))
                               //             //     .await
                               //             //     .expect("Cannot bind to server send port");
                               //             let ff = tx_outgoing.clone();
                               //     tokio::spawn(async move {
                               //         loop {
                               //             match incoming_subscriber.recv().await {
                               //                 Ok(msg) => {
                               //                     trace!("handle msg from tx {msg}");
                               //                     match ff.send((src, msg)) {
                               //                         Ok(_) => {}
                               //                         Err(error) => {
                               //                             error!("send to outgoing {error}")
                               //                         }
                               //                     }
                               //                     // send_socket
                               //                     //     .send_to(
                               //                     //         msg.as_bytes(),
                               //                     //         SocketAddr::from((src.ip(), send_port)),
                               //                     //     )
                               //                     //     .await
                               //                     //     .expect(&format!("Could not send message to {src}"));
                               //                 }
                               //                 Err(error) => {
                               //                     error!("recv from tx: {error}")
                               //                 }
                               //             }
                               //         }
                               //     });
                               //     //         } else {
                               //     //             // trace!("src {src} added to channels");
                               //     //             // channels_per_src.insert(src.to_string(), String::from("yes"));
                               //     //         }
                               //     //         messages.push_back(msg);
                               //     //     }
                               //     //     _ => {}
                               //     // }
                               // }
                };
                ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
            }
        });
        trace!("4");
        tokio::select! {_ = send_to_client_task => {} _ = recv_client_input_task => {} _ = recv_game_state_task => {}}
    });

    // loop {
    // let next_msg = messages.pop_front();
    // match next_msg {
    //     Some(pop_msg) => {
    //         debug!("pop_front: {}", pop_msg);
    //         match tx_incoming.send(pop_msg.clone()) {
    //             Ok(_) => {
    //                 trace!("sent to tx successfully")
    //             }
    //             Err(error) => {
    //                 error!("sending to tx: {error}")
    //             }
    //         }
    //         // runtime.broadcast(&pop_msg, active_players, active_observers);
    //     }
    //     _ => {}
    // }
    // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    // }
}
async fn handle_game_command(nosql: &UnQLite, game_command: GameCommand) {
    let existing_game_data = nosql.kv_fetch("123");
    match existing_game_data {
        Ok(existing_game_data) => {
            let existing_game_data = String::from_utf8(existing_game_data).unwrap();
            println!("value of 123: {:?}", existing_game_data);
            let existing_game_data = existing_game_data.add(" and aaa lot");
            nosql
                .kv_store("123", existing_game_data.add(" and aaa lot"))
                .unwrap();
        }
        Err(error) => {
            println!("Storing key 123");
            nosql.kv_store("123", "a lot").unwrap();
        }
    }
}
