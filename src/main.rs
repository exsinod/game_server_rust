use base64;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Write};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::process::exit;
use std::str::{self, FromStr};
use std::time::Duration;
use tokio;

static SEND_SERVER_ADDR: &str = "127.0.0.1:8877";
static RECV_SERVER_ADDR: &str = "127.0.0.1:8878";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct ServerRuntime {
    send_socket: UdpSocket,
    recv_socket: UdpSocket,
    players: HashMap<String, Player>,
    addrs: HashMap<String, String>,
}

impl ServerRuntime {
    fn new() -> Self {
        let send_socket = UdpSocket::bind(SEND_SERVER_ADDR).unwrap();
        send_socket
            .set_read_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        send_socket
            .set_write_timeout(Some(Duration::new(0, 1000)))
            .unwrap();
        let recv_socket = UdpSocket::bind(RECV_SERVER_ADDR).unwrap();
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

    fn broadcast(&mut self, msg: &str) {
        for key in self.players.keys() {
            let decoded_src = base64::decode(key).unwrap();
            match self.addrs.get(str::from_utf8(&decoded_src).unwrap()) {
                Some(send_src) => {
                    let socket_addr = SocketAddr::from_str(send_src).unwrap();
                    println!("sending to {:?}", socket_addr);
                    match self.send_socket.send_to(&msg.as_bytes(), socket_addr) {
                        Ok(_) => {
                            println!("sent {} to {:?}", msg, socket_addr);
                            match self.send_socket.recv(&mut []) {
                                Ok(_) => {
                                    println!("recv from {:?}", socket_addr);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_message(&mut self) -> Option<String> {
        let mut buf = [0; 128];
        match self.recv_socket.recv_from(&mut buf) {
            Ok((number_of_bytes, src)) => {
                // println!("number_of_bytes: {}", number_of_bytes);
                // let src = SocketAddr::from_str("127.0.0.1:8767").unwrap();
                if number_of_bytes > 1 {
                    let encoded_src = base64::encode(src.to_string());
                    let op_context = Self::get_context_from(&buf, number_of_bytes);
                    let player_data_vec: Vec<&str> = op_context.split(";").collect();
                    println!("player_data in handle_message: {:?}", player_data_vec);
                    // println!("operation: {}", Self::get_operation_from(&buf));
                    return match Self::get_operation_from(&buf) {
                        "S0;" => {
                            println!("{:?}", player_data_vec);
                            let send_addr = player_data_vec[0];
                            if !self.addrs.contains_key(&src.to_string()) {
                                self.addrs.insert(src.to_string(), send_addr.to_string());
                            }
                            let player_data: Player = Player::new(
                                "".to_string(),
                                "".to_string(),
                                0,
                                true,
                                Point::new(0, 0),
                                Point::new(0, 0),
                                0,
                            );
                            Some(player_data.to_string())
                        }
                        "L1;" => Some(self.login(encoded_src, player_data_vec)),
                        "M0;" => Some(
                            self.r#move(encoded_src, u8::from_str(player_data_vec[1]).unwrap_or(4)),
                        ),
                        "P0;" => Some(self.play()),
                        "E0;" => exit(0),
                        _ => Some("unknown".to_string()),
                    };
                }
                match self.recv_socket.send_to(&[], src) {
                    Ok(_) => {}
                    _ => {}
                }
            }
            _ => {}
        }
        None
    }

    fn r#move(&mut self, player_id: String, direction: u8) -> String {
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
                format!("{}", player.to_move_str())
            }
            None => "player not found for move".to_string(),
        }
    }

    fn play(&mut self) -> String {
        format!("play")
    }

    fn login(&mut self, player_id: String, player_data_vec: Vec<&str>) -> String {
        if self.players.contains_key(player_data_vec[0]) {
            println!("Player already logged in...");
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
            );
            self.players.insert(player_id, player_data.clone());
            println!("New user logs in.  Current players: {:?}", self.players);
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct Player {
    id: String,
    char_name: String,
    skin: u8,
    logged_in: bool,
    pos: Point,
    team: u8,
    world_pos: Point,
}

impl Player {
    fn new(
        id: String,
        char_name: String,
        skin: u8,
        logged_in: bool,
        world_pos: Point,
        pos: Point,
        team: u8,
    ) -> Self {
        Self {
            id,
            char_name,
            skin,
            logged_in,
            pos,
            team,
            world_pos,
        }
    }
    fn to_move_str(&self) -> String {
        // "P0;blub_id;Primal;2;{};{};1;0;0",
        let props: Vec<String> = vec![
            "P0".to_string(),
            self.id.to_string(),
            self.char_name.to_string(),
            self.skin.to_string(),
            self.pos.to_string(),
            self.team.to_string(),
            self.world_pos.to_string(),
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
            self.team.to_string(),
            self.world_pos.to_string(),
        ];
        f.write_str(&props.join(";"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut runtime: ServerRuntime = ServerRuntime::new();
    let mut messages: VecDeque<String> = VecDeque::new();
    loop {
        // runtime.send_socket.connect("127.0.0.1:9999")?;
        // let test = runtime.send_socket.send(b"test")?;
        // println!("sent test: {:?}", test);
        // println!("what is on the stack: {:?}", messages);
        // tokio::select! {
        let next_msg = pop_front_message(&mut messages);
        match next_msg {
            Some(pop_msg) => {
                println!("pop_front: {}", pop_msg);
                runtime.broadcast(&pop_msg);
            }
            _ => {}
        }
        let handled_msg = handle_message(&mut runtime);
        match handled_msg {
            Some(msg) => {
                println!("handled_msg to push back on stack: {}", msg);
                messages.push_back(msg);
            }
            _ => {}
        }
        ::std::thread::sleep(Duration::new(0, 1_000));
    }
}

fn handle_message(runtime: &mut ServerRuntime) -> Option<String> {
    runtime.handle_message()
}

fn pop_front_message(messages: &mut VecDeque<String>) -> Option<String> {
    messages.pop_front()
}
