use base64;
use chrono::{DateTime, Utc};
use log::{debug, error, trace};
use std::collections::HashMap;
use std::fmt;
use std::net::{SocketAddr, UdpSocket};
use std::process::exit;
use std::str::{self, FromStr};
use std::time::Duration;

static RECV_SOCKET_PORT: u16 = 8877;
static SEND_SOCKET_PORT: u16 = 8878;
static SEND_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];
static RECV_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];

pub struct ServerRuntime {
    send_socket: UdpSocket,
    recv_socket: UdpSocket,
    pub players: HashMap<String, Player>,
    addrs: HashMap<String, [u8; 4]>,
}

impl ServerRuntime {
    pub fn new() -> Self {
        let send_socket =
            UdpSocket::bind(SocketAddr::from((SEND_SERVER_ADDR, SEND_SOCKET_PORT))).unwrap();
        send_socket
            .set_read_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        send_socket
            .set_write_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        let recv_socket =
            UdpSocket::bind(SocketAddr::from((RECV_SERVER_ADDR, RECV_SOCKET_PORT))).unwrap();
        recv_socket
            .set_read_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        recv_socket
            .set_write_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        Self {
            send_socket,
            recv_socket,
            players: HashMap::new(),
            addrs: HashMap::new(),
        }
    }

    pub fn broadcast(&mut self, msg: &str) {
        trace!("players: {:?}", self.players);
        for key in self.players.keys() {
            let decoded_src = base64::decode(key).unwrap();
            let decoded_src = str::from_utf8(&decoded_src).unwrap();
            let decoded_src_parts = decoded_src.split(":").collect::<Vec<_>>();
            let recv_port = u16::from_str(decoded_src_parts[1]).unwrap();
            let send_port = recv_port + 1;
            match self.addrs.get(key) {
                Some(client_addr) => {
                    trace!("sending for key: {:?}", decoded_src);
                    trace!("sending for value: {:?}", self.addrs.get(key));
                    trace!("sending to send port: {:?}", send_port);
                    match self
                        .send_socket
                        .send_to(&msg.as_bytes(), SocketAddr::from((*client_addr, send_port)))
                    {
                        _ => {}
                    }
                }
                None => {}
            }
        }
    }

    pub fn handle_message(&mut self) -> Option<String> {
        let mut buf = [0; 128];
        match self.recv_socket.recv_from(&mut buf) {
            Ok((number_of_bytes, src)) => {
                let mut result_command: Option<String> = None;
                if number_of_bytes > 1 {
                    trace!("socket addr: {}", src);
                    let encoded_src = base64::encode(src.to_string());
                    let op_context = Self::get_context_from(&buf, number_of_bytes);
                    let player_data_vec: Vec<&str> = op_context.split(";").collect();
                    debug!("player_data in handle_message: {:?}", player_data_vec);
                    debug!("operation: {}", Self::get_operation_from(&buf));
                    result_command = match Self::get_operation_from(&buf) {
                        "S0;" => Some(self.sync(encoded_src, player_data_vec)),
                        "L1;" => Some(self.login(encoded_src, player_data_vec)),
                        "M0;" => Some(
                            self.r#move(encoded_src, u8::from_str(player_data_vec[1]).unwrap_or(4)),
                        ),
                        "P0;" => Some(self.play()),
                        "E0;" => exit(0),
                        _ => Some("unknown".to_string()),
                    };
                } else {
                    debug!("no data");
                }
                match self.recv_socket.send_to(&[], src) {
                    Ok(_) => {
                        trace!("handle msg sent")
                    }
                    Err(error) => {
                        error!("ack handle msg: {}", error)
                    }
                }
                return result_command;
            }
            Err(error) => {
                error!("recv handle msg: {}", error);
            }
        }
        return None;
    }

    fn r#move(&mut self, player_id: String, direction: u8) -> String {
        trace!("players: {:?}", self.players);
        let player = self.players.get_mut(&player_id);
        match player {
            Some(player) => {
                match direction {
                    0 => player.pos.y -= 10,
                    1 => player.pos.x += 10,
                    2 => player.pos.y += 10,
                    3 => player.pos.x -= 10,
                    _ => {}
                }
                player.velocity = direction;
                player.last_update = Utc::now();
                format!("{}", player.to_move_str())
            }
            None => "player not found for move".to_string(),
        }
    }

    fn sync(&mut self, player_id: String, player_data_vec: Vec<&str>) -> String {
        let client_addr_parts: &Vec<u8> = &player_data_vec[0]
            .split(".")
            .map(|part| u8::from_str_radix(part, 10).unwrap())
            .collect::<Vec<u8>>();
        let client_addr = [
            client_addr_parts[0],
            client_addr_parts[1],
            client_addr_parts[2],
            client_addr_parts[3],
        ];
        debug!("syncing {:?} to {:?}", player_id, client_addr);
        self.addrs.insert(player_id, client_addr);
        format!("sync")
    }

    fn play(&mut self) -> String {
        format!("play")
    }

    fn login(&mut self, player_id: String, player_data_vec: Vec<&str>) -> String {
        if self.players.contains_key(player_data_vec[0]) {
            debug!("Player already logged in...");
            let player = self.players.get(player_data_vec[0]).unwrap();
            format!("{}", player)
        } else {
            let player_data: Player = Player::new(
                player_id.clone(),
                player_data_vec[0].to_string(),
                0,
                true,
                Point::new(0, 0),
                Point::new(0, 0),
                0,
                0,
                Utc::now(),
            );
            self.players.insert(player_id, player_data.clone());
            debug!("New user logs in.  Current players: {:?}", self.players);
            format!("{}", &player_data)
        }
    }

    fn get_operation_from(buffer: &[u8]) -> &str {
        str::from_utf8(&buffer[0..3]).unwrap()
    }

    fn get_context_from(buffer: &[u8], size: usize) -> &str {
        str::from_utf8(&buffer[3..size]).unwrap_or("no context")
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}
impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&*format!("{};{}", self.x, self.y))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Player {
    id: String,
    char_name: String,
    skin: u8,
    logged_in: bool,
    pos: Point,
    velocity: u8,
    team: u8,
    world_pos: Point,
    pub last_update: DateTime<Utc>,
}

impl Player {
    fn new(
        id: String,
        char_name: String,
        skin: u8,
        logged_in: bool,
        world_pos: Point,
        pos: Point,
        velocity: u8,
        team: u8,
        last_update: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            char_name,
            skin,
            logged_in,
            pos,
            velocity,
            team,
            world_pos,
            last_update,
        }
    }
    fn to_move_str(&self) -> String {
        // format: "P0;blub_id;Primal;2;{};{};1;0;0",
        let props: Vec<String> = vec![
            "P0".to_string(),
            self.id.to_string(),
            self.char_name.to_string(),
            self.skin.to_string(),
            self.pos.to_string(),
            self.velocity.to_string(),
            self.team.to_string(),
            self.world_pos.to_string(),
            self.last_update.to_string(),
        ];
        let move_str = props.join(";");
        move_str
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let props = vec![
            self.id.to_string(),
            self.logged_in.to_string(),
            self.pos.to_string(),
            self.velocity.to_string(),
            self.team.to_string(),
            self.world_pos.to_string(),
        ];
        f.write_str(&props.join(";"))
    }
}
