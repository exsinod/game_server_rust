use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use log::{debug, trace};
use redis::Connection;
use tokio::sync::mpsc::Receiver;

use crate::components::{CommandType, GameCommand, Player, Point};

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

    pub fn process(&mut self, game_command: &GameCommand) {
        trace!("in process: {game_command:?}");
        match game_command.command {
            CommandType::LoginCommand => {
                trace!("its a login!!");
                self.login(game_command);
            }
            CommandType::MoveCommand => {
                trace!("its a move!!");
                self.r#move(game_command);
            }
            _ => {}
        };
    }

    pub fn update_pos(&mut self, player_id: String, pos: Point) {
        println!("update position in sync of player {player_id}");
        let player = redis::cmd("HGET")
            .arg("players")
            .arg(&player_id)
            .query::<String>(&mut self.redis_connection)
            .unwrap_or(String::new());
        let mut player: Player = serde_json::from_str(&player).unwrap();
        player.pos.x = pos.x;
        player.pos.y = pos.y;
        redis::cmd("HSET")
            .arg("players")
            .arg(&player_id)
            .arg(serde_json::to_string(&player).unwrap())
            .query::<u32>(&mut self.redis_connection)
            .expect("redis failed to set players");
    }

    pub fn get_current_game_state(&mut self) -> HashMap<String, Player> {
        let all_players = redis::cmd("HVALS")
            .arg("players")
            .query::<Vec<String>>(&mut self.redis_connection)
            .unwrap_or(Vec::new());
        let all_players = all_players
            .iter()
            .map(|player| {
                let player: Player = serde_json::from_str(player).unwrap();
                (player.id.clone(), player)
            })
            .collect::<HashMap<String, Player>>();
        return all_players;
    }

    pub fn _get_last_updated(&mut self) -> String {
        redis::cmd("GET")
            .arg("updated")
            .query::<String>(&mut self.redis_connection)
            .unwrap_or(0.to_string())
    }

    fn r#move(&mut self, game_command: &GameCommand) {
        match redis::cmd("HGET")
            .arg("players")
            .arg(&game_command.context.player_id)
            .query::<Option<String>>(&mut self.redis_connection)
            .expect("redis failed to get players")
        {
            Some(stored_player) => {
                let player: Option<Player> = serde_json::from_str(&stored_player).unwrap_or(None);
                trace!("Player in move {:?}", player);
                let direction = &game_command.context.direction;
                match player {
                    Some(mut player) => {
                        match direction {
                            0 => player.pos.y -= 5,
                            1 => player.pos.x += 5,
                            2 => player.pos.y += 5,
                            3 => player.pos.x -= 5,
                            _ => {}
                        }
                        player.velocity = *direction;
                        player.last_update = Utc::now().timestamp();
                        redis::cmd("HSET")
                            .arg("players")
                            .arg(&game_command.context.player_id)
                            .arg(serde_json::to_string(&player).unwrap())
                            .query::<u32>(&mut self.redis_connection)
                            .expect("redis failed to set players");
                        redis::cmd("SET")
                            .arg("updated")
                            .arg(Utc::now().timestamp())
                            .query::<String>(&mut self.redis_connection)
                            .expect("redis failed to set updated");
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    fn login(&mut self, game_command: &GameCommand) {
        debug!("Login command: {:?}", game_command);
        match game_command.context.player_type.as_str() {
            "observer" => {
                trace!("observer added");
            }
            "player" => {
                trace!("player added");
                let new_player: Player = Player::new(
                    game_command.context.player_id.clone(),
                    game_command.context.player_id.clone(),
                    game_command.context.skin,
                    true,
                    Point::new(0, 0),
                    Point::new(0, 0),
                    0,
                    0,
                    Utc::now().timestamp(),
                );
                redis::cmd("HSET")
                    .arg("players")
                    .arg(&game_command.context.player_id)
                    .arg(serde_json::to_string(&new_player).unwrap())
                    .query::<u32>(&mut self.redis_connection)
                    .expect("redis failed to set players");
            }
            _ => {}
        }
    }
    fn _sync(&mut self, _game_command: GameCommand) -> String {
        "".to_string()
    }
    fn _play(&mut self, _game_command: GameCommand) -> String {
        "".to_string()
    }
}

pub async fn start_game_command_processor(mut rx_process: Receiver<GameCommand>) {
    trace!("spawning game command processor thread");
    let mut game_command_processor = GameCommandProcessor::new();
    let mut move_commands: HashMap<String, GameCommand> = HashMap::new();
    loop {
        match rx_process.try_recv() {
            Ok(game_command) => {
                println!("Processor: will process {game_command:?}");
                if game_command.command == CommandType::MoveCommand {
                    move_commands.insert(game_command.context.player_id.clone(), game_command);
                } else {
                    game_command_processor.process(&game_command);
                    move_commands.remove(&game_command.context.player_id);
                }
            }
            Err(_) => {}
        }

        move_commands.values().for_each(|cmd| {
            trace!("Processor: processing {cmd:?}");
            game_command_processor.process(&cmd);
        });
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 20));
    }
}
