use chrono::Utc;
use log::{debug, trace};
use redis::{Client, Connection, RedisResult, RedisWrite};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fmt;
use std::net::{SocketAddr, UdpSocket};
use std::process::exit;
use std::str::{self, FromStr};
use std::time::Duration;
use unqlite::ffi::unqlite_open;
use unqlite::{UnQLite, KV};

static RECV_SOCKET_PORT: u16 = 8877;
// static SEND_SOCKET_PORT: u16 = 8878;
// static SEND_SERVER_ADDR: [u8; 4] = [0, 0, 0, 0];
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
    redis_connection: Connection,
}

impl GameCommandProcessor {
    pub fn new() -> Self {
        let redis_client =
            redis::Client::open("redis://127.0.0.1").expect("Failed to connect to redis");
        let redis_connection = redis_client
            .get_connection()
            .expect("Failed to get connection from redis");
        Self { redis_connection }
    }

    pub fn process(&mut self, game_command: GameCommand) -> String {
        let game_state = match game_command.command.as_str() {
            "S0;" => self.sync(game_command),
            "L1;" => self.login(game_command),
            "M0;" => self.r#move(game_command),
            "P0;" => self.play(game_command),
            "E0;" => exit(0),
            _ => "".to_string(),
        };
        trace!("Game state after process: {game_state}");
        game_state
    }

    pub fn get_current_game_state(&mut self) -> String {
        redis::cmd("GET")
            .arg("players")
            .query::<String>(&mut self.redis_connection)
            .unwrap_or("{}".to_string())
    }

    fn r#move(&mut self, game_command: GameCommand) -> String {
        match redis::cmd("GET")
            .arg("players")
            .query::<Option<String>>(&mut self.redis_connection)
            .expect("redis failed to get players")
        {
            Some(stored_players) => {
                let mut players: HashMap<String, Player> =
                    serde_json::from_str(&stored_players).unwrap();
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
                        redis::cmd("SET")
                            .arg("players")
                            .arg(serde_json::to_string(&players_clone).unwrap())
                            .query::<Option<String>>(&mut self.redis_connection)
                            .expect("redis failed to set players");
                        format!("{}", player.to_move_str())
                    }
                    None => "player not found for move".to_string(),
                }
            }
            None => "".to_string(),
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
                let players = redis::cmd("GET")
                    .arg("players")
                    .query::<Option<String>>(&mut self.redis_connection)
                    .expect("redis failed to get players");
                match players {
                    Some(players) => {
                        debug!("Login: Players: {}", players);
                        match serde_json::from_str::<HashMap<String, Player>>(&players) {
                            Ok(mut players_map) => {
                                players_map.insert(new_player.id.clone(), new_player.clone());
                                trace!("Login: Current players: {:?}", players);
                                redis::cmd("SET")
                                    .arg("players")
                                    .arg(serde_json::to_string(&players_map).unwrap())
                                    .query::<String>(&mut self.redis_connection)
                                    .expect("redis failed to set players");
                                "".to_string()
                            }
                            Err(_) => "".to_string(),
                        }
                    }
                    None => {
                        debug!("Login: No players, creating new hashmap.");
                        let mut player_data: HashMap<String, Player> = HashMap::new();
                        player_data.insert(new_player.id.clone(), new_player.clone());
                        match serde_json::to_string(&player_data) {
                            Ok(string) => {
                                redis::cmd("SET")
                                    .arg("players")
                                    .arg(string)
                                    .query::<String>(&mut self.redis_connection)
                                    .expect("redis failed to set players");
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
    fn sync(&mut self, game_command: GameCommand) -> String {
        "".to_string()
    }
    fn play(&mut self, game_command: GameCommand) -> String {
        "".to_string()
    }
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
