use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    time::Duration,
};

use futures::StreamExt;
use log::trace;
use tokio::{net::UdpSocket, sync::mpsc::Receiver};

use crate::components::Player;
use crate::processor::GameCommandProcessor;

// output string format:  "P0;Hashmap<String,Player>"
pub async fn send_socket_task(mut rx_send_addr: Receiver<(String, String, SocketAddr)>) {
    trace!("spawning send socket thread");
    let mut addresses: HashSet<(String, SocketAddr)> = HashSet::new();
    let send_socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 8878)))
        .await
        .unwrap();
    let mut game_processor = GameCommandProcessor::new();
    loop {
        match rx_send_addr.try_recv() {
            Ok((command, player_id, addr)) => match command.as_str() {
                "ADD" => {
                    addresses.insert((player_id, addr));
                }
                _ => {}
            },
            Err(_) => { //error!("{error}");}
            }
        }
        let mut game_state: HashMap<String, Player> = game_processor.get_current_game_state();
        send_to_all(&send_socket, &mut addresses, game_state).await;

        ::std::thread::sleep(Duration::new(0, 1_000_000_000 / 20));
    }
}

async fn send_to_all(
    socket: &UdpSocket,
    addresses: &mut HashSet<(String, SocketAddr)>,
    game_state: HashMap<String, Player>,
) {
    // addresses.insert(("obs".to_string(), SocketAddr::from(([127, 0, 0, 1], 9999))));
    let stream = futures::stream::iter(addresses.iter());
    stream
        .for_each_concurrent(None, |addr| async {
            let mut game_state_clone = game_state.clone();
            // println!("removing {}", addr.0);
            game_state_clone.remove(&addr.0);
            // println!("P0;{}", serde_json::to_string(&game_state_clone).unwrap());
            socket
                .send_to(
                    format!("P0;{}", serde_json::to_string(&game_state_clone).unwrap()).as_bytes(),
                    addr.1,
                )
                .await
                .unwrap();
            trace!("Sent {:?} to {}", game_state, addr.1);
        })
        .await;
}
