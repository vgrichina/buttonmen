use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, serde_json, near_bindgen, require};
use near_sdk::serde::{Deserialize, Serialize};

use near_rng::Rng;

const MAX_LATEST_GAMES: usize = 10;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    pub games: LookupMap<String, Game>,
    pub last_game_id: u64,
    pub latest_games: Vec<String>,
    pub web4_static_url: String,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            games: LookupMap::new(b"g".to_vec()),
            last_game_id: 0,
            latest_games: vec![],
            // NOTE: This points to web4.near.page static by default
            // TODO: Point to default deployment of this game frontend
            web4_static_url: "ipfs://bafkreig74di4midqzggkjfmtfu4c7gei3u6scihgkvig2k4mjrovcjl4ri".to_string(),
        }
    }
}

fn user_games_key(player_id: String) -> Vec<u8> {
    format!("ug:{}", player_id).as_bytes().to_vec()
}

fn get_user_games(player_id: String) -> Vec<String> {
    match env::storage_read(&user_games_key(player_id)) {
        Some(user_games_vec) => {
            let user_games_str = String::from_utf8(user_games_vec).unwrap();
            user_games_str.split(",").map(|s| s.to_string()).collect::<Vec<String>>()
        },
        None => vec![],
    }
}

fn add_user_game(player_id: String, game_id: String) -> () {
    let mut user_games_ids = get_user_games(player_id.to_string());
    user_games_ids.push(game_id);
    env::storage_write(&user_games_key(player_id), &user_games_ids.join(",").as_bytes());

    // TODO: Limit the number of games per user
}

#[near_bindgen]
impl Contract {
    /// Learn more about web4 here: https://web4.near.page
    pub fn web4_get(&self, request: Web4Request) -> Web4Response {
        if request.path == "/" || request.path.starts_with("/games/") {
            return self.serve_static("/index.html");
        }

        if request.path == "/config.js" {
            return Web4Response::Body {
                content_type: "application/javascript".to_owned(),
                body: format!("window._web4Config = {{ contractName: '{}' }};", env::current_account_id()).as_bytes().to_owned().into(),
            }
        }

        // check path starts with /games/
        if request.path.starts_with("/api/games") {
            if request.path == "/api/games" {
                return Web4Response::Body {
                    content_type: "application/json".to_owned(),
                    body: serde_json::to_vec(&self.latest_games.iter()
                        .map(|game_id| { self.games.get(&game_id.to_string()).unwrap() })
                        // TODO: Track games you joined separately
                        // .filter(|game| { game.players.contains(&"".to_string()) })
                        .collect::<Vec<Game>>()).unwrap().into(),
                }
            }

            let parts = request.path.split("/").collect::<Vec<&str>>();
            let game_id = parts[3];

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

        if request.path.starts_with("/api/users") {
            let parts = request.path.split("/").collect::<Vec<&str>>();
            let user_id = parts[3];

            if parts[4] == "games" {
                let user_games_ids = match env::storage_read(format!("ug:{}", user_id).as_bytes()) {
                    Some(user_games_vec) => {
                        let user_games_str = String::from_utf8(user_games_vec).unwrap();
                        user_games_str.split(",").map(|s| s.to_string()).collect::<Vec<String>>()
                    },
                    None => vec![],
                };

                return Web4Response::Body {
                    content_type: "application/json".to_owned(),
                    // return game as JSON
                    body: serde_json::to_vec(&user_games_ids.iter()
                        .map(|game_id| { self.games.get(&game_id.to_string()).unwrap() })
                        .collect::<Vec<Game>>()).unwrap().into(),
                }
            }

            // TODO: return 404?
        }


        return self.serve_static(request.path.as_str());
    }

    fn serve_static(&self, path: &str) -> Web4Response {
        Web4Response::BodyUrl {
            body_url: format!("{}{}", self.web4_static_url, path),
        }
    }

    pub fn create_game(&mut self) -> String {
        self.last_game_id += 1;
        let game_id = format!("{}", self.last_game_id);
        let player_id = env::predecessor_account_id();

        let mut rng = Rng::new(&env::random_seed());
        let game = Game {
            id: game_id.clone(),
            players: vec![player_id.to_string(), "".to_string()],
            current_player: 0,
            // TODO: Roll dice according to character sheet
            dice: vec![roll_dice(&mut rng, vec![4, 6, 8, 10, 20]), vec![]],
            captured: vec![vec![], vec![]],
        };

        self.games.insert(&game_id, &game);
        self.latest_games.push(game_id.clone());
        if self.latest_games.len() > MAX_LATEST_GAMES {
            self.latest_games.remove(0);
        }

        add_user_game(player_id.to_string(), game_id.clone());

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

                        add_user_game(player_id.to_string(), game_id.clone());
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
                let current_player_index = game.players.iter().position(|p| p == &player_id).unwrap_or_else(|| panic!("Player {} has not joined game {}", player_id, game_id));
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

                // Update the game state
                self.games.insert(&game_id, &game);
            },
            None => {
                panic!("Game not found: {}", game_id);
            }
        }
    }

    // TODO: Move this to a separate trait together with serve_static
    pub fn web4_setStaticUrl(&mut self, url: String) -> () {
        // TODO: Allow to set owner like in https://github.com/near/near-sdk-rs/blob/00226858199419aaa8c99f756bd192851666fb36/near-contract-standards/src/upgrade/mod.rs#L7
        require!(env::predecessor_account_id() == env::current_account_id(), "Only owner can set static URL");

        self.web4_static_url = url;
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Die {
    size: u8,
    value: u8
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Game {
    id: String,
    players: Vec<String>,
    current_player: u8,
    dice: Vec<Vec<Die>>,
    captured: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::testing_env;
    use near_sdk::test_utils::VMContextBuilder;

    fn login_as(player_id: &str) {
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(player_id.parse().unwrap())
            .build());
    }

    #[test]
    fn create_game() {
        let mut contract = Contract::default();
        contract.create_game();

        assert_eq!(contract.last_game_id, 1);
        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.id, "1");
        assert_eq!(game.players, vec!["bob.near".to_string(), "".to_string()]);
        assert_eq!(game.current_player, 0);
        assert_eq!(game.dice.len(), 2);
        assert_eq!(game.dice[0].len(), 5);
        assert_eq!(game.dice[1].len(), 0);
        assert_eq!(game.captured.len(), 2);
        assert_eq!(game.captured[0].len(), 0);
        assert_eq!(game.captured[1].len(), 0);
        assert_eq!(contract.latest_games, vec!["1".to_string()]);
    }

    #[test]
    #[should_panic(expected = "Player bob.near has already joined game 1")]
    fn join_game_same_player() {
        let mut contract = Contract::default();
        contract.create_game();
        contract.join_game("1".to_string());
    }

    #[test]
    fn join_game_other_player() {
        let mut contract = Contract::default();
        contract.create_game();

        login_as("alice.near");
        contract.join_game("1".to_string());

        assert_eq!(contract.last_game_id, 1);
        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 0);
        assert_eq!(game.dice.len(), 2);
        assert_eq!(game.dice[0].len(), 5);
        assert_eq!(game.dice[1].len(), 5);
        assert_eq!(game.captured.len(), 2);
        assert_eq!(game.captured[0].len(), 0);
        assert_eq!(game.captured[1].len(), 0);
    }

    #[test]
    #[should_panic(expected = "Game is full: 1")]
    fn join_game_full() {
        let mut contract = Contract::default();
        contract.create_game();

        login_as("alice.near");
        contract.join_game("1".to_string());

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id("eve.near".parse().unwrap())
            .build());
        contract.join_game("1".to_string());
    }

    #[test]
    #[should_panic(expected = "Game not found: 1")]
    fn join_game_not_found() {
        let mut contract = Contract::default();
        contract.join_game("1".to_string());
    }

    #[test]
    #[should_panic(expected = "It is not your turn")]
    fn attack_not_your_turn() {
        let mut contract = Contract::default();
        contract.create_game();

        login_as("alice.near");
        contract.join_game("1".to_string());
        contract.attack("1".to_string(), vec![0], 0);
    }

    #[test]
    #[should_panic(expected = "Player eve.near has not joined game 1")]
    fn attack_not_joined() {
        let mut contract = Contract::default();
        contract.create_game();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id("alice.near".parse().unwrap())
            .build());
        contract.join_game("1".to_string());

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id("eve.near".parse().unwrap())
            .build());
        contract.attack("1".to_string(), vec![0], 0);
    }

    #[test]
    #[should_panic(expected = "Attack failed")]
    fn attack_failed() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![Die { size: 4, value: 1 }], vec![Die { size: 4, value: 2 }]],
            captured: vec![vec![], vec![]],
        });

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id("bob.near".parse().unwrap())
            .build());
        contract.attack("1".to_string(), vec![0], 0);
    }

    #[test]
    fn attack_power_success() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![Die { size: 4, value: 4 }, Die { size: 6, value: 1} ], vec![Die { size: 4, value: 2 }]],
            captured: vec![vec![], vec![]],
        });

        contract.attack("1".to_string(), vec![0], 0);

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 1);
        // NOTE: The attacker's die is re-rolled. It's deterministic in tests
        assert_eq!(game.dice, vec![vec![Die { size: 4, value: 2 }, Die { size: 6, value: 1 }], vec![]]);
        assert_eq!(game.captured, vec![vec![4], vec![]]);
    }

    #[test]
    fn attack_skill_success() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![Die { size: 4, value: 2 }, Die { size: 6, value: 4 }], vec![Die { size: 10, value: 6 }]],
            captured: vec![vec![], vec![]],
        });

        contract.attack("1".to_string(), vec![0, 1], 0);

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 1);
        assert_eq!(game.dice, vec![vec![Die { size: 4, value: 2 }, Die { size: 6, value: 4 }], vec![]]);
        assert_eq!(game.captured, vec![vec![10], vec![]]);
    }

    #[test]
    fn attack_power_alice() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 1,
            dice: vec![vec![Die { size: 4, value: 4 }, Die { size: 6, value: 1} ], vec![Die { size: 4, value: 3 }]],
            captured: vec![vec![], vec![]],
        });

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id("alice.near".parse().unwrap())
            .build());
        contract.attack("1".to_string(), vec![0], 1);

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 0);
        // NOTE: The attacker's die is re-rolled. It's deterministic in tests
        assert_eq!(game.dice, vec![vec![Die { size: 4, value: 4 }], vec![Die { size: 4, value: 2 }]]);
        assert_eq!(game.captured, vec![vec![], vec![6]]);
    }

    fn request_path(path: &str) -> Web4Request {
        Web4Request {
            account_id: None,
            path: path.to_string(),
            params: std::collections::HashMap::new(),
            query: std::collections::HashMap::new(),
            preloads: None,
        }
    }

    #[test]
    fn web4_get_serve_index() {
        let contract = Contract::default();
        let response = contract.web4_get(request_path("/"));

        assert_eq!(response, Web4Response::BodyUrl {
            body_url: "ipfs://bafkreig74di4midqzggkjfmtfu4c7gei3u6scihgkvig2k4mjrovcjl4ri/index.html".to_string(),
        });
    }

    #[test]
    fn web4_get_serve_static() {
        let contract = Contract::default();
        let response = contract.web4_get(request_path("/static/style.css"));

        assert_eq!(response, Web4Response::BodyUrl {
            body_url: "ipfs://bafkreig74di4midqzggkjfmtfu4c7gei3u6scihgkvig2k4mjrovcjl4ri/static/style.css".to_string(),
        });
    }

    #[test]
    fn web4_get_latest_games_empty() {
        let contract = Contract::default();

        let response = contract.web4_get(request_path("/api/games"));
        assert_eq!(response, Web4Response::Body {
            content_type: "application/json".to_owned(),
            body: "[]".as_bytes().to_owned().into(),
        });
    }

    #[test]
    fn web4_get_latest_games() {
        let mut contract = Contract::default();
        let game1 = contract.create_game();
        let game2 = contract.create_game();

        login_as("alice.near");
        contract.join_game(game2.clone());

        let response = contract.web4_get(request_path("/api/games"));
        match response {
            Web4Response::Body { content_type, body } => {
                assert_eq!(content_type, "application/json".to_owned());
                assert_eq!(String::from_utf8(body.into()).unwrap(),
                    serde_json::to_string(&vec![
                        contract.games.get(&game1).unwrap(),
                        contract.games.get(&game2).unwrap(),
                    ]).unwrap());
            },
            _ => panic!("Unexpected response"),
        }
    }

    #[test]
    fn web4_get_your_games_empty() {
        let contract = Contract::default();

        let response = contract.web4_get(request_path("/api/users/bob.near/games"));
        assert_eq!(response, Web4Response::Body {
            content_type: "application/json".to_owned(),
            body: "[]".as_bytes().to_owned().into(),
        });
    }

    #[test]
    fn web4_get_your_games() {
        let mut contract = Contract::default();
        let game1 = contract.create_game();
        let game2 = contract.create_game();

        login_as("alice.near");
        contract.join_game(game2.clone());

        match contract.web4_get(request_path("/api/users/alice.near/games")) {
            Web4Response::Body { content_type, body } => {
                assert_eq!(content_type, "application/json".to_owned());
                assert_eq!(String::from_utf8(body.into()).unwrap(),
                    serde_json::to_string(&vec![
                        contract.games.get(&game2).unwrap()]).unwrap());
            },
            _ => panic!("Unexpected response"),
        }

        match contract.web4_get(request_path("/api/users/bob.near/games")) {
            Web4Response::Body { content_type, body } => {
                assert_eq!(content_type, "application/json".to_owned());
                assert_eq!(String::from_utf8(body.into()).unwrap(),
                    serde_json::to_string(&vec![
                        contract.games.get(&game1).unwrap(),
                        contract.games.get(&game2).unwrap(),
                    ]).unwrap());
            },
            _ => panic!("Unexpected response"),
        }
    }
}
