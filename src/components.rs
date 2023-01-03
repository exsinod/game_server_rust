use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::{self, FromStr};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CommandContext {
    pub player_id: String,
    pub direction: u8,
    pub player_type: String,
    pub skin: u8,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct LoginCommandContext {
    pub player_id: String,
    pub player_type: String,
    pub skin: u8,
}
impl LoginCommandContext {
    pub fn from_login_cmd(data: Vec<String>) -> CommandContext {
        CommandContext {
            player_id: data[0].clone(),
            player_type: data[2].clone(),
            direction: 0,
            skin: u8::from_str(&data[1]).unwrap(),
        }
    }
}
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct StationnaryCommandContext {
    pub player_id: String,
    pub direction: u8,
    pub skin: u8,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct MoveCommandContext {
    pub player_id: String,
    pub direction: u8,
    pub skin: u8,
}
impl MoveCommandContext {
    pub fn from_move_cmd(data: Vec<String>) -> CommandContext {
        CommandContext {
            player_id: data[0].clone(),
            direction: u8::from_str(&data[1]).unwrap(),
            skin: 0,
            player_type: "".to_string(),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum CommandType {
    MoveCommand,
    LoginCommand,
    StopCommand,
}

pub trait GetCmd {
    fn get_cmd(&self) -> CommandContext;
}

pub trait MoveCmd {}
pub trait LoginCmd {}
pub trait StationnaryCmd {}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct GameCommand {
    pub timestamp: i64,
    pub command: CommandType,
    pub context: CommandContext,
}

impl GameCommand {
    pub fn from(timestamp: i64, command: CommandType, context: CommandContext) -> Self {
        Self {
            timestamp,
            command,
            context,
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
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
    pub pos: Point,
    pub velocity: u8,
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
