use core::fmt::{Debug, Formatter};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, serde_json, near_bindgen, require};
use near_sdk::serde::{Deserialize, Serialize};

use near_rng::Rng;

const MAX_LATEST_GAMES: usize = 10;

const DEFAULT_DICE: [Die; 5] = [
    Die { kind: DieKind::Normal, size: 4 },
    Die { kind: DieKind::Normal, size: 6 },
    Die { kind: DieKind::Normal, size: 8 },
    Die { kind: DieKind::Normal, size: 10 },
    Die { kind: DieKind::Normal, size: 20 },
];


#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum DieKind {
    Normal,
    Poision,
    Speed,
    SwingDice { low_size: u8, high_size: u8 },
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Die {
    kind: DieKind,
    size: u8,
}

impl Debug for Die {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO: Differentiate between dice types
        write!(f, "D{}", self.size)
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DieRoll {
    #[serde(flatten)]
    die: Die,
    value: u8
}

impl Debug for DieRoll {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.die, self.value)
    }
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Game {
    id: String,
    players: Vec<String>,
    current_player: u8,
    starting_dice: Vec<Vec<Die>>,
    dice: Vec<Vec<DieRoll>>,
    captured: Vec<Vec<u8>>,
    round: u8,
    scores: Vec<Vec<u8>>,

    // NOTE: This is reserved for future upgrades, can be replaced with enum later
    reserved: Option<()>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            players: vec![],
            current_player: 0,
            starting_dice: vec![DEFAULT_DICE.to_vec(), DEFAULT_DICE.to_vec()],
            dice: vec![vec![], vec![]],
            captured: vec![vec![], vec![]],
            round: 0,
            scores: vec![],
            reserved: None,
        }
    }
}

impl Game {
    pub fn is_round_over(&self) -> bool {
        self.dice.iter().any(|dice| dice.len() == 0)
    }

    pub fn is_game_over(&self) -> bool {
        // The first player to win three rounds wins the match.

        if self.round < 3 {
            return false;
        }

        let wins_by_player = (0..self.round).fold([0, 0], |mut wins, round| {
            if let Some(winner) = self.find_round_winner_idx(round) {
                wins[winner] += 1;
            }
            wins
        });

        if wins_by_player.iter().any(|wins| *wins >= 3) {
            return true;
        }

        return false;
    }

    pub fn is_pass_allowed(&self) -> bool {
        if self.current_player as usize > self.players.len() {
            // Game not started yet
            return false;
        }

        if self.is_round_over() {
            return false;
        }

        let power_attack = self.find_power_attack();
        if power_attack.is_some() {
            return false;
        }

        let skill_attack = self.find_skill_attack();
        if skill_attack.is_some() {
            return false;
        }

        return true;
    }

    fn find_round_winner_idx(&self, round: u8) -> Option<usize> {
        // Round winner is the player with most score in that round.
        // If both players have the same score, the round is a draw and is not counted.

        if !self.is_round_over() {
            return None;
        }

        let round_scores = &self.scores[round as usize];
        let max_score = round_scores.iter().max().unwrap();
        let max_score_count = round_scores.iter().filter(|score| *score == max_score).count();
        if max_score_count > 1 {
            return None;
        }

        return Some(round_scores.iter().position(|score| score == max_score).unwrap());
    }

    fn find_power_attack(&self) -> Option<(usize, usize)> {
        let current_player_index = self.current_player as usize;
        let other_player_index = (self.current_player as usize + 1) % 2;

        // Verify that power attack is not possible
        for attacker_die_index in 0..self.dice[current_player_index].len() {
            for defender_die_index in 0..self.dice[other_player_index].len() {
                if self.dice[current_player_index][attacker_die_index].value >= self.dice[other_player_index][defender_die_index].value {
                    return Some((attacker_die_index, defender_die_index));
                }
            }
        }

        return None;
    }

    fn find_skill_attack(&self) -> Option<(Vec<u8>, u8)> {
        let current_player_index = self.current_player as usize;
        let other_player_index = (self.current_player as usize + 1) % 2;

        fn find_skill_attack_recursive(attacker_dice_values: &[u8], defender_die_value: u8, selected_attacker_dice: Vec<u8>) -> Option<Vec<u8>> {
            if attacker_dice_values.len() == 0 {
                if defender_die_value == 0 {
                    return Some(selected_attacker_dice);
                } else {
                    return None;
                }
            }

            if let Some(result) = find_skill_attack_recursive(&attacker_dice_values[1..], defender_die_value, selected_attacker_dice.clone()) {
                return Some(result);
            }

            if defender_die_value >= attacker_dice_values[0] {
                if let Some(result) = find_skill_attack_recursive(&attacker_dice_values[1..], defender_die_value - attacker_dice_values[0], {
                    let mut selected_attacker_dice = selected_attacker_dice.clone();
                    selected_attacker_dice.push(0);
                    selected_attacker_dice
                }) {
                    return Some(result);
                }
            }

            return None;
        }

        // Verify that skill attack is not possible
        for defender_die_index in 0..self.dice[other_player_index].len() {
            let defender_die_value = self.dice[other_player_index][defender_die_index].value;
            let attacker_dice_values = self.dice[current_player_index].iter().map(|die| die.value).collect::<Vec<u8>>();

            let result = find_skill_attack_recursive(&attacker_dice_values, defender_die_value, vec![]);
            if result.is_some() {
                return Some((result.unwrap(), defender_die_value));
            }
        }

        return None;
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GameView {
    id: String,
    players: Vec<String>,
    current_player: u8,
    starting_dice: Vec<Vec<Die>>,
    dice: Vec<Vec<DieRoll>>,
    captured: Vec<Vec<u8>>,
    round: u8,
    scores: Vec<Vec<u8>>,

    is_pass_allowed: bool,
    is_round_over: bool,
    is_game_over: bool,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Player {
    id: String,
    games: Vec<String>,

    // TODO: more fields. Track wins / etc?


    // NOTE: This is reserved for future upgrades, can be replaced with enum later
    reserved: Option<()>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            games: vec![],
            reserved: None,
        }
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    pub games: LookupMap<String, Game>,
    pub players: LookupMap<String, Player>,
    pub last_game_id: u64,
    pub latest_games: Vec<String>,
    pub web4_static_url: String,

    // NOTE: This is reserved for future upgrades, can be replaced with enum later
    pub reserved: Option<()>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            games: LookupMap::new(b"g".to_vec()),
            players: LookupMap::new(b"p".to_vec()),
            last_game_id: 0,
            latest_games: vec![],
            // NOTE: This points to web4.near.page static by default
            // TODO: Point to default deployment of this game frontend
            web4_static_url: "ipfs://bafkreig74di4midqzggkjfmtfu4c7gei3u6scihgkvig2k4mjrovcjl4ri".to_string(),
            reserved: None,
        }
    }
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
                        .collect::<Vec<Game>>()).unwrap().into(),
                }
            }

            let parts = request.path.split("/").collect::<Vec<&str>>();
            let game_id = parts[3];

            match self.games.get(&game_id.to_string()) {
                Some(game) => {
                    let game_view = GameView {
                        id: game.id.clone(),
                        players: game.players.clone(),
                        current_player: game.current_player,
                        starting_dice: game.starting_dice.clone(),
                        dice: game.dice.clone(),
                        captured: game.captured.clone(),
                        round: game.round,
                        scores: game.scores.clone(),
                        is_pass_allowed: game.is_pass_allowed(),
                        is_round_over: game.is_round_over(),
                        is_game_over: game.is_game_over(),
                    };
                    return Web4Response::Body {
                        content_type: "application/json".to_owned(),
                        body: serde_json::to_vec(&game_view).unwrap().into(),
                    }
                },
                None => {
                    // game does not exist
                    return Web4Response::Error {
                        status: 404,
                        content_type: "text/plain".to_owned(),
                        body: format!("Game not found: {}", game_id).as_bytes().to_owned().into(),
                    }
                }
            }
        }

        if request.path.starts_with("/api/users") {
            let parts = request.path.split("/").collect::<Vec<&str>>();
            let user_id = parts[3];

            if parts[4] == "games" {
                let user_games_ids = match self.players.get(&user_id.to_string()) {
                    Some(player) => player.games.clone(),
                    None => vec![],
                };

                return Web4Response::Body {
                    content_type: "application/json".to_owned(),
                    body: serde_json::to_vec(&user_games_ids.iter()
                        .map(|game_id| { self.games.get(&game_id.to_string()).unwrap() })
                        .collect::<Vec<Game>>()).unwrap().into(),
                }
            }

            return resource_not_found(request.path);
        }


        return self.serve_static(request.path.as_str());
    }

    fn serve_static(&self, path: &str) -> Web4Response {
        Web4Response::BodyUrl {
            body_url: format!("{}{}", self.web4_static_url, path),
        }
    }

    pub fn create_game(&mut self, starting_dice: Vec<Die>) -> String {
        self.last_game_id += 1;
        let game_id = format!("{}", self.last_game_id);
        let player_id = env::predecessor_account_id();
        let mut rng = Rng::new(&env::random_seed());
        let game = Game {
            id: game_id.clone(),
            players: vec![player_id.to_string(), "".to_string()],
            current_player: 0xFF,
            starting_dice: vec![starting_dice.clone(), vec![]],
            dice: vec![roll_dice(&mut rng, starting_dice), vec![]],
            ..Default::default()
        };

        self.games.insert(&game_id, &game);
        self.latest_games.push(game_id.clone());
        if self.latest_games.len() > MAX_LATEST_GAMES {
            self.latest_games.remove(0);
        }

        let mut player = self.players.get(&player_id.to_string()).unwrap_or_else(|| {
            let player = Player {
                id: player_id.to_string(),
                ..Default::default()
            };
            player
        });
        player.games.push(game_id.clone());
        self.players.insert(&player_id.to_string(), &player);

        return game_id;
    }

    fn determine_starting_player(&self, dice: Vec<Vec<DieRoll>>) -> u8 {
        // Sorted dice from lowest to highest for every player
        let sorted_dice = dice.iter().cloned().map(|mut dice| {
            dice.sort_by(|a, b| a.value.cmp(&b.value));
            dice
        }).collect::<Vec<Vec<DieRoll>>>();

        // Whoever rolled the single lowest number will go first.
        // If the lowest dice are tied, compare the next lowest dice,
        // and so on until a starting player is determined.
        // TODO: If all numbers are tied, the round is a draw.
        for i in 0..sorted_dice[0].len() {
            if sorted_dice[1][i].value < sorted_dice[0][i].value {
                return 1;
            }
        }

        return 0;
    }

    pub fn join_game(&mut self, game_id: String, starting_dice: Vec<Die>) -> () {
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
                        game.starting_dice[player_index] = starting_dice.clone();
                        let mut rng = Rng::new(&env::random_seed());
                        game.dice[player_index] = roll_dice(&mut rng, game.starting_dice[player_index].clone());

                        // TODO: If all numbers are tied, the round is a draw.
                        game.current_player = self.determine_starting_player(game.dice.clone());

                        // Update the game state
                        self.games.insert(&game_id, &game);

                        let mut player = self.players.get(&player_id.to_string()).unwrap_or_else(|| {
                            let player = Player {
                                id: player_id.to_string(),
                                ..Default::default()
                            };
                            player
                        });
                        player.games.push(game_id.clone());
                        self.players.insert(&player_id.to_string(), &player);
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

                if game.is_round_over() {
                    panic!("Round is over");
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
                game.captured[current_player_index].push(game.dice[defender_dice_idx][defender_die_index as usize].die.size);
                game.dice[defender_dice_idx].remove(defender_die_index as usize);
                // Re-roll attacker dice
                let mut rng = Rng::new(&env::random_seed());
                attacker_die_indices.iter().for_each(|index| {
                    game.dice[attacker_dice_idx][*index as usize] = roll_die(&mut rng, &game.dice[attacker_dice_idx][*index as usize].die);
                });
                // Switch to the next player
                game.current_player = (game.current_player + 1) % 2;

                // Update the game state
                self.games.insert(&game_id, &game);
            },
            None => {
                panic!("Game not found: {}", game_id);
            }
        }
    }

    pub fn next_round(&mut self, game_id: String) -> () {
        let player_id = env::predecessor_account_id().to_string();

        match self.games.get(&game_id) {
            Some(mut game) => {
                if !game.players.iter().any(|p| p == &player_id) {
                    panic!("Player {} has not joined game {}", player_id, game_id);
                }

                if !game.is_round_over() {
                    panic!("Round is not over yet");
                }

                game.round += 1;
                let mut rng = Rng::new(&env::random_seed());
                game.dice = game.starting_dice.iter().map(|dice| roll_dice(&mut rng, dice.clone())).collect();
                game.scores.push(game.captured.iter().map(|captured| captured.iter().fold(0, |acc, size| acc + *size as u8)).collect());
                game.captured = vec![vec![], vec![]];
                game.current_player = self.determine_starting_player(game.dice.clone());

                self.games.insert(&game_id, &game);
            }
            None => {
                panic!("Game not found: {}", game_id);
            }
        }
    }


    pub fn pass(&mut self, game_id: String) -> () {
        let player_id = env::predecessor_account_id().to_string();

        match self.games.get(&game_id) {
            Some(mut game) => {
                let current_player_index = game.players.iter().position(|p| p == &player_id).unwrap_or_else(|| panic!("Player {} has not joined game {}", player_id, game_id));
                if game.current_player != current_player_index as u8 {
                    panic!("It is not your turn");
                }

                let power_attack = game.find_power_attack();
                if power_attack.is_some() {
                    panic!("Power attack is possible");
                }

                let skill_attack = game.find_skill_attack();
                if skill_attack.is_some() {
                    panic!("Skill attack is possible");
                }

                // Switch to the next player
                game.current_player = (game.current_player + 1) % 2;

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

fn roll_die(rng: &mut Rng, die: &Die) -> DieRoll {
    DieRoll {
        die: die.clone(),
        value: rng.rand_range_u32(1, die.size.into()) as u8,
    }
}

fn roll_dice(rng: &mut Rng, dice: Vec<Die>) -> Vec<DieRoll> {
    dice.iter().map(|die| roll_die(rng, &die)).collect()
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
    Error {
        status: u16,
        content_type: String,
        body: near_sdk::json_types::Base64VecU8,
    }
}

fn resource_not_found(path: String) -> Web4Response {
    Web4Response::Error {
        status: 404,
        content_type: "text/plain".to_owned(),
        body: format!("Resource not found: {}", path).as_bytes().to_owned().into(),
    }
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

    fn die(size: u8, value: u8) -> DieRoll {
        DieRoll {
            die: Die {
                kind: DieKind::Normal,
                size,
            },
            value,
        }
    }

    #[test]
    fn create_game() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        assert_eq!(contract.last_game_id, 1);
        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.id, "1");
        assert_eq!(game.players, vec!["bob.near".to_string(), "".to_string()]);
        assert_eq!(game.current_player, 0xff);
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
        contract.create_game(DEFAULT_DICE.to_vec());
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());
    }

    #[test]
    fn join_game_other_player() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        testing_env!(VMContextBuilder::new()
            // 32 bytes of random seed
            // used just so that current_player is not 0 because of the same roll
            .random_seed([
                1, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0
            ])
            .predecessor_account_id("alice.near".parse().unwrap())
            .build());
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());

        assert_eq!(contract.last_game_id, 1);
        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 1);
        assert_eq!(game.starting_dice, vec![DEFAULT_DICE.to_vec(), DEFAULT_DICE.to_vec()]);
        assert_eq!(game.dice, vec![
            vec![die(4, 2), die(6, 4), die(8, 5), die(10, 5), die(20, 5)],
            vec![die(4, 2), die(6, 3), die(8, 2), die(10, 1), die(20, 5)]]);
        assert_eq!(game.captured, vec![vec![], vec![]] as Vec<Vec<u8>>);
    }

    #[test]
    #[should_panic(expected = "Game is full: 1")]
    fn join_game_full() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id("eve.near".parse().unwrap())
            .build());
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());
    }

    #[test]
    #[should_panic(expected = "Game not found: 1")]
    fn join_game_not_found() {
        let mut contract = Contract::default();
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());
    }

    #[test]
    #[should_panic(expected = "It is not your turn")]
    fn attack_not_your_turn() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());
        contract.attack("1".to_string(), vec![0], 0);
    }

    #[test]
    #[should_panic(expected = "Player eve.near has not joined game 1")]
    fn attack_not_joined() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());

        login_as("eve.near");
        contract.attack("1".to_string(), vec![0], 0);
    }

    #[test]
    #[should_panic(expected = "Round is over")]
    fn attack_failed_round_is_over() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![die(4, 1)], vec![]],
            ..Default::default()
        });

        login_as("bob.near");
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
            dice: vec![vec![die(4, 1)], vec![die(4, 2)]],
            ..Default::default()
        });

        login_as("bob.near");
        contract.attack("1".to_string(), vec![0], 0);
    }

    #[test]
    fn attack_power_success() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![die(4, 4), die(6, 1) ], vec![die(4, 2)]],
            ..Default::default()
        });

        contract.attack("1".to_string(), vec![0], 0);

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 1);
        // NOTE: The attacker's die is re-rolled. It's deterministic in tests
        assert_eq!(game.dice, vec![vec![die(4, 2), die(6, 1)], vec![]]);
        assert_eq!(game.captured, vec![vec![4], vec![]]);
    }

    #[test]
    fn attack_skill_success() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![die(4, 2), die(6, 4)], vec![die(10, 6)]],
            ..Default::default()
        });

        contract.attack("1".to_string(), vec![0, 1], 0);

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 1);
        assert_eq!(game.dice, vec![vec![die(4, 2), die(6, 4)], vec![]]);
        assert_eq!(game.captured, vec![vec![10], vec![]]);
    }

    #[test]
    fn attack_power_alice() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 1,
            dice: vec![vec![die(4, 4), die(6, 1) ], vec![die(4, 3)]],
            ..Default::default()
        });

        login_as("alice.near");
        contract.attack("1".to_string(), vec![0], 1);

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 0);
        // NOTE: The attacker's die is re-rolled. It's deterministic in tests
        assert_eq!(game.dice, vec![vec![die(4, 4)], vec![die(4, 2)]]);
        assert_eq!(game.captured, vec![vec![], vec![6]]);
    }

    #[test]
    #[should_panic(expected = "It is not your turn")]
    fn pass_not_your_turn() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());
        contract.pass("1".to_string());
    }

    #[test]
    #[should_panic(expected = "Player eve.near has not joined game 1")]
    fn pass_not_joined() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());

        login_as("eve.near");
        contract.pass("1".to_string());
    }

    #[test]
    #[should_panic(expected = "Power attack is possible")]
    fn pass_power_attack_possible() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![vec![die(4, 1), die(6, 4) ], vec![die(4, 2)]],
            ..Default::default()
        });

        contract.pass("1".to_string());
    }

    #[test]
    #[should_panic(expected = "Skill attack is possible")]
    fn pass_skill_attack_possible() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![
                vec![die(4, 1), die(6, 1), die(10, 2)],
                vec![die(4, 3), die(8, 6)]],
            ..Default::default()
        });

        contract.pass("1".to_string());
    }

    #[test]
    fn pass_success() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![
                vec![die(4, 1), die(6, 1) ],
                vec![die(4, 3)]],
            ..Default::default()
        });

        contract.pass("1".to_string());

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.current_player, 1);
        assert_eq!(game.dice, vec![
            vec![die(4, 1), die(6, 1) ],
            vec![die(4, 3)]]);
        assert_eq!(game.captured, vec![vec![], vec![]] as Vec<Vec<u8>>);
    }

    #[test]
    #[should_panic(expected = "Round is not over yet")]
    fn next_round_not_over_yet() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());
        contract.next_round("1".to_string());
    }

    #[test]
    #[should_panic(expected = "Player eve.near has not joined game 1")]
    fn next_round_not_joined() {
        let mut contract = Contract::default();
        contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game("1".to_string(), DEFAULT_DICE.to_vec());

        login_as("eve.near");
        contract.next_round("1".to_string());
    }

    #[test]
    #[should_panic(expected = "Game not found: 1")]
    fn next_round_not_found() {
        let mut contract = Contract::default();
        contract.next_round("1".to_string());
    }

    #[test]
    fn next_round_success() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            round: 1,
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![
                vec![die(4, 1), die(6, 1) ],
                vec![]],
            captured: vec![vec![4, 6, 10], vec![10]],
            ..Default::default()
        });

        contract.next_round("1".to_string());

        let game = contract.games.get(&"1".to_string()).unwrap();
        assert_eq!(game.players, vec!["bob.near".to_string(), "alice.near".to_string()]);
        assert_eq!(game.round, 2);
        assert_eq!(game.current_player, 1);
        assert_eq!(game.dice, vec![
            vec![die(4, 2), die(6, 4), die(8, 5), die(10, 5), die(20, 5)],
            vec![die(4, 2), die(6, 2), die(8, 1), die(10, 4), die(20, 3)]]);
        assert_eq!(game.captured, vec![vec![], vec![]] as Vec<Vec<u8>>);
        assert_eq!(game.scores, vec![vec![20, 10]]);
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
    fn web4_get_game_state() {
        let mut contract = Contract::default();
        let game_id = contract.create_game(DEFAULT_DICE.to_vec());

        let response = contract.web4_get(request_path(&format!("/api/games/{}", game_id)));
        match response {
            Web4Response::Body { content_type, body } => {
                assert_eq!(content_type, "application/json".to_owned());
                assert_eq!(String::from_utf8(body.into()).unwrap(),
                    serde_json::to_string(&serde_json::json!({
                        "id": game_id,
                        "players": ["bob.near", ""],
                        "current_player": 0xff,
                        "starting_dice": [
                            [{"kind": "Normal", "size": 4}, {"kind": "Normal", "size": 6}, {"kind": "Normal", "size": 8}, {"kind": "Normal", "size": 10}, {"kind": "Normal", "size": 20}],
                            []],
                        "dice": [
                            [{"kind": "Normal", "size": 4, "value": 2}, {"kind": "Normal", "size": 6, "value": 4}, {"kind": "Normal", "size": 8, "value": 5}, {"kind": "Normal", "size": 10, "value": 5}, {"kind": "Normal", "size": 20, "value": 5}],
                            []
                        ],
                        "captured": [[], []],
                        "round": 0,
                        "scores": [],
                        "is_pass_allowed": false,
                        "is_round_over": true,
                        "is_game_over": false,
                    })).unwrap());

            },
            _ => panic!("Unexpected response"),
        }
    }

    #[test]
    fn web4_get_game_state_is_pass_allowed() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![
                vec![die(4, 1), die(6, 1)],
                vec![die(4, 3)]],
            ..Default::default()
        });

        let response = contract.web4_get(request_path(&format!("/api/games/1")));
        match response {
            Web4Response::Body { content_type, body } => {
                assert_eq!(content_type, "application/json".to_owned());
                assert_eq!(String::from_utf8(body.into()).unwrap(),
                    serde_json::to_string(&serde_json::json!({
                        "id": "1",
                        "players": ["bob.near", "alice.near"],
                        "current_player": 0,
                        "starting_dice": [
                            [{"kind": "Normal", "size": 4}, {"kind": "Normal", "size": 6}, {"kind": "Normal", "size": 8}, {"kind": "Normal", "size": 10}, {"kind": "Normal", "size": 20}],
                            [{"kind": "Normal", "size": 4}, {"kind": "Normal", "size": 6}, {"kind": "Normal", "size": 8}, {"kind": "Normal", "size": 10}, {"kind": "Normal", "size": 20}]
                        ],
                        "dice": [
                            [{"kind": "Normal", "size": 4, "value": 1}, {"kind": "Normal", "size": 6, "value": 1}],
                            [{"kind": "Normal", "size": 4, "value": 3}]
                        ],
                        "captured": [[], []],
                        "round": 0,
                        "scores": [],
                        "is_pass_allowed": true,
                        "is_round_over": false,
                        "is_game_over": false,
                    })).unwrap());
            },
            _ => panic!("Unexpected response"),
        }
    }

    #[test]
    fn web4_get_game_state_game_over() {
        let mut contract = Contract::default();
        contract.games.insert(&"1".to_string(), &Game {
            id: "1".to_string(),
            players: vec!["bob.near".to_string(), "alice.near".to_string()],
            current_player: 0,
            dice: vec![
                vec![die(4, 1), die(6, 1)],
                vec![]],
            round: 5,
            scores: vec![vec![20, 10], vec![10, 20], vec![20, 10], vec![10, 20], vec![10, 20]],
            ..Default::default()
        });

        let response = contract.web4_get(request_path(&format!("/api/games/1")));
        match response {
            Web4Response::Body { content_type, body } => {
                assert_eq!(content_type, "application/json".to_owned());
                assert_eq!(String::from_utf8(body.into()).unwrap(),
                    serde_json::to_string(&serde_json::json!({
                        "id": "1",
                        "players": ["bob.near", "alice.near"],
                        "current_player": 0,
                        "starting_dice": [
                            [{"kind": "Normal", "size": 4}, {"kind": "Normal", "size": 6}, {"kind": "Normal", "size": 8}, {"kind": "Normal", "size": 10}, {"kind": "Normal", "size": 20}],
                            [{"kind": "Normal", "size": 4}, {"kind": "Normal", "size": 6}, {"kind": "Normal", "size": 8}, {"kind": "Normal", "size": 10}, {"kind": "Normal", "size": 20}]
                        ],
                        "dice": [
                            [{"kind": "Normal", "size": 4, "value": 1}, {"kind": "Normal", "size": 6, "value": 1}],
                            []
                        ],
                        "captured": [[], []],
                        "round": 5,
                        "scores": [[20, 10], [10, 20], [20, 10], [10, 20], [10, 20]],
                        "is_pass_allowed": false,
                        "is_round_over": true,
                        "is_game_over": true,
                    })).unwrap());
            },
            _ => panic!("Unexpected response"),
        }
    }

    #[test]
    fn web4_get_game_state_not_found() {
        let contract = Contract::default();

        let response = contract.web4_get(request_path("/api/games/1"));
        assert_eq!(response, Web4Response::Error{
            status: 404,
            content_type: "text/plain".to_owned(),
            body: "Game not found: 1".as_bytes().to_owned().into(),
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
        let game1 = contract.create_game(DEFAULT_DICE.to_vec());
        let game2 = contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game(game2.clone(), DEFAULT_DICE.to_vec());

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
        let game1 = contract.create_game(DEFAULT_DICE.to_vec());
        let game2 = contract.create_game(DEFAULT_DICE.to_vec());

        login_as("alice.near");
        contract.join_game(game2.clone(), DEFAULT_DICE.to_vec());

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
