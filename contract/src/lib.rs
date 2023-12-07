use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::env;
use near_sdk::near_bindgen;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;

use near_rng::Rng;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    pub games: LookupMap<String, Game>,
    pub last_game_id: u64,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            games: LookupMap::new(b"g".to_vec()),
            last_game_id: 0,
        }
    }
}

#[near_bindgen]
impl Contract {
    /// Learn more about web4 here: https://web4.near.page
    pub fn web4_get(&self, request: Web4Request) -> Web4Response {
        if request.path == "/" {
            return Web4Response::Body {
                content_type: "text/html; charset=UTF-8".to_owned(),
                body: "<h1>Hello from Web4 on NEAR!</h1>".as_bytes().to_owned().into(),
            }
        }

        // check path starts with /games/
        if request.path.starts_with("/games") {
            let parts = request.path.split("/").collect::<Vec<&str>>();

            // check if game exists
            if request.path == "/games" {
                // return list of games
                // TODO: return list of games
                return Web4Response::Body {
                    content_type: "application/json".to_owned(),
                    body: serde_json::to_vec(&vec!["game_1", "game_2"]).unwrap().into(),
                }
            }

            // return game status
            let game_id = parts[2];

            match self.games.get(&game_id.to_string()) {
                Some(game) => {
                    return Web4Response::Body {
                        content_type: "application/json".to_owned(),
                        // return game as JSON
                        body: serde_json::to_vec(&game).unwrap().into(),
                    }
                },
                None => {
                    // if game does not exist, return 404
                    // TODO: Support HTTP error codes in boilerplate
                    return Web4Response::Body {
                        content_type: "text/html; charset=UTF-8".to_owned(),
                        body: "<h1>Game not found</h1>".as_bytes().to_owned().into(),
                    }
                }
            }
        }

        // return 404
        // TODO: Support HTTP error codes in boilerplate
        Web4Response::Body {
            content_type: "text/html; charset=UTF-8".to_owned(),
            body: "<h1>404 Not Found</h1>".as_bytes().to_owned().into(),
        }
    }

    pub fn create_game(&mut self) -> String {
        self.last_game_id += 1;
        let game_id = format!("{}", self.last_game_id);
        let player_id = env::predecessor_account_id();

        let mut rng = Rng::new(&env::random_seed());
        let game = Game {
            players: vec![player_id.to_string(), "".to_string()],
            current_player: 0,
            // TODO: Roll dice according to character sheet
            dice: vec![roll_dice(&mut rng, vec![4, 6, 8, 10, 20]), vec![]],
            captured: vec![vec![], vec![]],
        };

        self.games.insert(&game_id, &game);
        return game_id;
    }

    pub fn join_game(&mut self, game_id: String) -> () {
        let player_id = env::predecessor_account_id().to_string();

        match self.games.get(&game_id) {
            Some(mut game) => {
                // Check if the player has already joined
                if game.players.contains(&player_id) {
                    panic!("Player {} has already joined game {}", player_id, game_id);
                }

                // Find an empty slot for the player
                match game.players.iter().position(|p| p == "") {
                    Some(player_index) => {
                        // Assign the player to the game
                        game.players[player_index] = player_id.to_string();
                        // TODO: Roll dice according to character sheet
                        game.dice[player_index] = roll_dice(&mut Rng::new(&env::random_seed()), vec![4, 6, 8, 10, 20]);

                        // Update the game state
                        self.games.insert(&game_id, &game);
                    },
                    None => {
                        panic!("Game is full: {}", game_id);
                    }
                }
            }
            None => {
                panic!("Game not found: {}", game_id);
            }
        }
    }

    pub fn attack(&mut self, game_id: String, attacker_die_indices: Vec<u8>, defender_die_index: u8) -> () {
        let player_id = env::predecessor_account_id().to_string();

        match self.games.get(&game_id) {
            Some(mut game) => {
                let current_player_index = game.players.iter().position(|p| p == &player_id).unwrap();
                if game.current_player != current_player_index as u8 {
                    panic!("It is not your turn");
                }

                let attacker_dice_idx = game.current_player as usize;
                let defender_dice_idx = (game.current_player + 1) as usize % 2;

                // Perform power attack or skill attack based on the number of attacker dice indices
                let attack_value = attacker_die_indices.iter().fold(0, |acc, index| acc + game.dice[attacker_dice_idx][*index as usize].value);
                let attack_success = if attacker_die_indices.len() == 1 {
                    // Power attack
                    attack_value >= game.dice[defender_dice_idx][defender_die_index as usize].value
                } else {
                    // Skill attack
                    attack_value == game.dice[defender_dice_idx][defender_die_index as usize].value
                };

                if !attack_success {
                    panic!("Attack failed");
                }

                // Capture the die
                game.captured[current_player_index].push(game.dice[defender_dice_idx][defender_die_index as usize].size);
                game.dice[defender_dice_idx].remove(defender_die_index as usize);
                // Re-roll attacker dice
                let mut rng = Rng::new(&env::random_seed());
                attacker_die_indices.iter().for_each(|index| {
                    game.dice[attacker_dice_idx][*index as usize] = roll_die(&mut rng, game.dice[attacker_dice_idx][*index as usize].size);
                });
                // Switch to the next player
                game.current_player = (game.current_player + 1) % 2;

                // Check win condition
                if game.dice[defender_dice_idx].len() == 0 {
                    // TODO: End game
                }
            },
            None => {
                panic!("Game not found: {}", game_id);
            }
        }
    }
}

fn roll_die(rng: &mut Rng, size: u8) -> Die {
    Die {
        size,
        value: rng.rand_range_u32(1, size.into()) as u8,
    }
}

fn roll_dice(rng: &mut Rng, sizes: Vec<u8>) -> Vec<Die> {
    sizes.iter().map(|size| roll_die(rng, *size)).collect()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Web4Request {
    #[serde(rename = "accountId")]
    pub account_id: Option<String>,
    pub path: String,
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub query: std::collections::HashMap<String, Vec<String>>,
    pub preloads: Option<std::collections::HashMap<String, Web4Response>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde", untagged)]
pub enum Web4Response {
    Body {
        #[serde(rename = "contentType")]
        content_type: String,
        body: near_sdk::json_types::Base64VecU8,
    },
    BodyUrl {
        #[serde(rename = "bodyUrl")]
        body_url: String,
    },
    PreloadUrls {
        #[serde(rename = "preloadUrls")]
        preload_urls: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Die {
    size: u8,
    value: u8
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Game {
    players: Vec<String>,
    current_player: u8,
    dice: Vec<Vec<Die>>,
    captured: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_game() {
        let mut contact = Contract::default();
        contact.create_game();

        assert_eq!(contact.last_game_id, 1);
        let game = contact.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players.len(), 2);
        assert_eq!(game.players[0], "bob.near");
        assert_eq!(game.players[1], "");
        assert_eq!(game.current_player, 0);
        assert_eq!(game.dice.len(), 2);
        assert_eq!(game.dice[0].len(), 5);
        assert_eq!(game.dice[1].len(), 0);
        assert_eq!(game.captured.len(), 2);
        assert_eq!(game.captured[0].len(), 0);
        assert_eq!(game.captured[1].len(), 0);
    }
}
