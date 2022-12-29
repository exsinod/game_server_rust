use base64;
use chrono::Utc;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fmt::{self};
use std::net::{SocketAddr, UdpSocket};
use std::process::exit;
use std::str::{self, FromStr};
use std::time::Duration;
use unqlite::{Cursor, UnQLite, KV};

static RECV_SOCKET_PORT: u16 = 8877;
static SEND_SOCKET_PORT: u16 = 8878;
static SEND_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];
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
                    trace!("Received msg from socket addr: {}", src);
                    let command = Self::get_operation_from(&buf);
                    let command_context = Self::get_context_from(&buf, number_of_bytes);
                    debug!(
                        "Command: {}, Command context: {:?}",
                        command, command_context
                    );
                    return Some((
                        src,
                        GameCommand::from(command.to_string(), command_context.to_string()),
                    ));
                }
            }
            Err(error) => {
                // trace!("Receiving: {error}"),
            }
        }
        None
    }

    fn get_operation_from(buffer: &[u8]) -> &str {
        str::from_utf8(&buffer[0..3]).unwrap()
    }

    fn get_context_from(buffer: &[u8], size: usize) -> &str {
        str::from_utf8(&buffer[3..size]).unwrap_or("no context")
    }
}

pub struct GameCommandProcessor {
    datastore: UnQLite,
}

impl GameCommandProcessor {
    pub fn new() -> Self {
        let datastore = UnQLite::create_temp();
        Self { datastore }
    }

    pub fn process(&mut self, game_command: GameCommand) -> String {
        let game_state = match game_command.command.as_str() {
            "S0;" => sync(game_command),
            "L1;" => self.login(game_command),
            "M0;" => self.r#move(game_command),
            "P0;" => play(),
            "E0;" => exit(0),
            _ => "".to_string(),
        };
        trace!("Game state: {game_state}");
        game_state
    }

    pub fn broadcast(
        &mut self,
        msg: &str,
        mut active_players: Vec<SocketAddr>,
        mut observers: Vec<SocketAddr>,
    ) {
        active_players.append(&mut observers);
        // for addr in active_players {
        //     trace!("sending for key: {:?}", addr);
        //     match self.send_socket.send_to(&msg.as_bytes(), addr) {
        //         _ => {}
        //     }
        // }
        // for key in self.players.keys() {
        //     let decoded_src = base64::decode(key).unwrap();
        //     let decoded_src = str::from_utf8(&decoded_src).unwrap();
        //     let decoded_src_parts = decoded_src.split(":").collect::<Vec<_>>();
        //     let recv_port = u16::from_str(decoded_src_parts[1]).unwrap();
        //     let send_port = recv_port + 1;
        //     match self.addrs.get(key) {
        //         Some(client_addr) => {
        //             trace!("sending for key: {:?}", decoded_src);
        //             trace!("sending for value: {:?}", self.addrs.get(key));
        //             trace!("sending to send port: {:?}", send_port);
        //             match self
        //                 .send_socket
        //                 .send_to(&msg.as_bytes(), SocketAddr::from((*client_addr, send_port)))
        //             {
        //                 _ => {}
        //             }
        //         }
        //         None => {}
        //     }
        // }
    }

    pub fn get_current_game_state(&mut self) -> String {
        String::new()
    }

    pub fn handle_command(&mut self, game_command: GameCommand) -> Option<String> {
        // let mut buf = [0; 128];
        // match self.recv_socket.recv_from(&mut buf) {
        //     Ok((number_of_bytes, src)) => {
        //         let mut result_command: Option<(String, SocketAddr)> = None;
        //         if number_of_bytes > 1 {
        //             trace!("socket addr: {}", src);
        //             let encoded_src = base64::encode(src.to_string());
        //             let op_context = Self::get_context_from(&buf, number_of_bytes);
        // let command: Vec<&str> = command.split(";").collect();
        //             debug!("player_data in handle_message: {:?}", player_data_vec);
        //             debug!("operation: {}", Self::get_operation_from(&buf));
        // result_command = match Self::get_operation_from(&buf) {
        let game_state: Option<String> = match game_command.command.as_str() {
            // "S0;" => Some((self.sync(encoded_src, player_data_vec), src))
            // "L1;".to_owned() => self.login(src, player_data_vec),
            // "M0;".to_owned() => Some((
            //     self.r#move(encoded_src, u8::from_str(player_data_vec[1]).unwrap_or(4)),
            //     src,
            // )),
            // "P0;" => Some(self.play()),
            // "E0;".to_owned() => exit(0),
            _ => None,
        };
        // } else {
        //     debug!("no data");
        // }
        // match self.recv_socket.send_to(&[], src) {
        //     Ok(_) => {
        //         trace!("handle msg sent")
        //     }
        //     Err(error) => {
        //         error!("ack handle msg: {}", error)
        //     }
        // }
        //         return result_command;
        //     }
        //     Err(error) => {
        //         error!("recv handle msg: {}", error);
        //     }
        // }
        return None;
    }

    fn r#move(&mut self, game_command: GameCommand) -> String {
        let stored_players = self.datastore.kv_fetch("players").unwrap();
        let mut players: HashMap<String, Player> =
            serde_json::from_str(&String::from_utf8(stored_players).unwrap()).unwrap();
        trace!("Players in move {:?}", players);
        let mut players_clone = players.clone();
        let player_id = &game_command.data[0];
        let player = players.get_mut(player_id);
        let direction = u8::from_str(&game_command.data[1]).unwrap();
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
                player.last_update = Utc::now().timestamp();
                let player_clone = player.clone();
                players_clone.insert(player_id.to_string(), player_clone);
                self.datastore
                    .kv_store("players", serde_json::to_string(&players_clone).unwrap())
                    .unwrap();
                format!("{}", player.to_move_str())
            }
            None => "player not found for move".to_string(),
        }
    }

    fn login(&mut self, game_command: GameCommand) -> String {
        match game_command.data[1].as_str() {
            "observer" => {
                trace!("observer added");
                // self.observers.push(player_id);
                // None
                "".to_string()
            }
            "player" => {
                let new_player: Player = Player::new(
                    game_command.data[0].to_string(),
                    game_command.data[0].to_string(),
                    0,
                    true,
                    Point::new(0, 0),
                    Point::new(0, 0),
                    0,
                    0,
                    Utc::now().timestamp(),
                );
                match self.datastore.kv_fetch("players") {
                    Ok(players) => match serde_json::from_str::<HashMap<String, Player>>(
                        &String::from_utf8(players).unwrap(),
                    ) {
                        Ok(mut players) => {
                            players.insert(new_player.id.clone(), new_player.clone());
                            trace!("player added");
                            "".to_string()
                        }
                        Err(_) => "".to_string(),
                    },
                    Err(_) => {
                        let mut player_data: HashMap<String, Player> = HashMap::new();
                        player_data.insert(new_player.id.clone(), new_player.clone());
                        match serde_json::to_string(&player_data) {
                            Ok(string) => {
                                self.datastore.kv_store("players", string).unwrap();
                            }
                            Err(_) => {}
                        }
                        "".to_string()
                    }
                }
            }
            _ => "".to_string(),
        }
    }
}

fn r#move(game_command: GameCommand) -> String {
    // trace!(
    //     "datastore players: {:?}",
    //     self.datastore.kv_fetch("players").unwrap()
    // );
    // let player = self.players.get_mut(&player_id);
    // match player {
    //     Some(player) => {
    //         match direction {
    //             0 => player.pos.y -= 10,
    //             1 => player.pos.x += 10,
    //             2 => player.pos.y += 10,
    //             3 => player.pos.x -= 10,
    //             _ => {}
    //         }
    //         player.velocity = direction;
    //         player.last_update = Utc::now().timestamp();
    //         format!("{}", player.to_move_str())
    //     }
    //     None => "player not found for move".to_string(),
    // }
    "".to_string()
}

fn sync(game_command: GameCommand) -> String {
    // debug!("syncing {:?} to {:?}", player_src_addr, client_addr);
    format!("sync")
}

fn login(datastore: UnQLite, game_command: GameCommand) -> String {
    match game_command.data[1].as_str() {
        "observer" => {
            trace!("observer added");
            // self.observers.push(player_id);
            // None
            "".to_string()
        }
        "player" => {
            let player_data: Player = Player::new(
                game_command.data[0].to_string(),
                game_command.data[0].to_string(),
                0,
                true,
                Point::new(0, 0),
                Point::new(0, 0),
                0,
                0,
                Utc::now().timestamp(),
            );
            datastore.kv_store("players", player_data.to_string());
            trace!("player added");
            "".to_string()
            // if self.players.contains_key(&player_id) {
            //     debug!("Player already logged in...");
            //     // let player = self.players.get(player_id).unwrap();
            //     // Some((format!("{}", player), player_src_addr))
            // } else {
            //     let player_data: Player = Player::new(
            //         player_id.to_string(),
            //         player_id.to_string(),
            //         0,
            //         true,
            //         Point::new(0, 0),
            //         Point::new(0, 0),
            //         0,
            //         0,
            //         Utc::now().timestamp(),
            //     );
            //     self.players.insert(player_id, player_data.clone());
            //     debug!("New user logs in.  Current players: {:?}", self.players);
            //     // Some((format!("{}", &player_data), player_src_addr))
            // }
        }
        _ => "".to_string(),
    }
}

fn get_operation_from(buffer: &[u8]) -> &str {
    str::from_utf8(&buffer[0..3]).unwrap()
}

fn get_context_from(buffer: &[u8], size: usize) -> &str {
    str::from_utf8(&buffer[3..size]).unwrap_or("no context")
}

fn play() -> String {
    format!("play")
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct GameCommand {
    command: String,
    data: Vec<String>,
}

impl GameCommand {
    pub fn from(command: String, data: String) -> Self {
        let data = data
            .split(";")
            .map(|dat| dat.to_string())
            .collect::<Vec<String>>();
        Self { command, data }
    }

    pub fn to_string(&mut self) -> String {
        "".to_string()
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Point {
    x: i32,
    y: i32,
}
impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&*format!("{};{}", self.x, self.y))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    char_name: String,
    skin: u8,
    logged_in: bool,
    pos: Point,
    velocity: u8,
    team: u8,
    world_pos: Point,
    pub last_update: i64,
}

impl Player {
    pub fn new(
        id: String,
        char_name: String,
        skin: u8,
        logged_in: bool,
        world_pos: Point,
        pos: Point,
        velocity: u8,
        team: u8,
        last_update: i64,
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
    pub fn to_move_str(&self) -> String {
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
