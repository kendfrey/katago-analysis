use serde::Serialize;
use serde_json::{Value, json};
use serde_with::skip_serializing_none;

use crate::{Bonus, Config, Player, Rules};

/// A request to the analysis engine.
#[derive(Debug, Clone, Serialize)]
#[serde(into = "Value")]
pub enum Request {
    /// Request the engine to analyze one or more positions.
    Analyze(AnalysisRequest),

    /// Request KataGo's version information.
    QueryVersion {
        /// The request ID.
        id: String,
    },

    /// Clear the neural network cache.
    ClearCache {
        /// The request ID.
        id: String,
    },

    /// Terminate a specific analysis request.
    Terminate {
        /// The request ID.
        id: String,

        /// The ID of the request to terminate.
        terminate_id: String,

        /// If provided, only terminate the analysis for the specified turn numbers.
        turn_numbers: Option<Vec<usize>>,
    },

    /// Terminate all pending analysis requests.
    TerminateAll {
        /// The request ID.
        id: String,

        /// If provided, only terminate the analysis for the specified turn numbers.
        turn_numbers: Option<Vec<usize>>,
    },

    /// Request information about the available neural network models.
    QueryModels {
        /// The request ID.
        id: String,
    },
}

impl From<Request> for Value {
    fn from(request: Request) -> Self {
        match request {
            Request::Analyze(request) => serde_json::to_value(request).unwrap(),
            Request::QueryVersion { id } => json!({
                "id": id,
                "action": "query_version",
            }),
            Request::ClearCache { id } => json!({
                "id": id,
                "action": "clear_cache",
            }),
            Request::Terminate {
                id,
                terminate_id,
                turn_numbers,
            } => {
                let mut value = json!({
                        "id": id,
                        "action": "terminate",
                        "terminateId": terminate_id,
                    }
                );
                if let Some(turn_numbers) = turn_numbers {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert("turnNumbers".to_string(), json!(turn_numbers));
                }
                value
            }
            Request::TerminateAll { id, turn_numbers } => {
                let mut value = json!({
                        "id": id,
                        "action": "terminate_all",
                    }
                );
                if let Some(turn_numbers) = turn_numbers {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert("turnNumbers".to_string(), json!(turn_numbers));
                }
                value
            }
            Request::QueryModels { id } => json!({
                "id": id,
                "action": "query_models",
            }),
        }
    }
}

/// A request to analyze one or more positions.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRequest {
    /// The request ID.
    pub id: String,

    /// The ruleset for this game.
    pub rules: Rules,

    /// The komi for this game.
    pub komi: Option<f64>,

    /// Bonus points white receives in handicap games.
    pub white_handicap_bonus: Option<Bonus>,

    /// The board width.
    pub board_x_size: u8,

    /// The board height.
    pub board_y_size: u8,

    /// The stones on the board before the first move.
    pub initial_stones: Option<Vec<(Player, String)>>,

    /// The player to move in the initial position.
    pub initial_player: Option<Player>,

    /// The moves played in the game. Move locations can be in GTP format (`"A1"`, `"pass"`, etc.) or explicit
    /// coordinates (`"(0,0)"`).
    pub moves: Vec<(Player, String)>,

    /// The positions to analyze, where 0 is the position before the first move.
    /// If not provided, only the final position will be analyzed.
    /// The engine will return a separate response for each position.
    pub analyze_turns: Option<Vec<usize>>,

    /// The maximum number of visits to use.
    pub max_visits: Option<u32>,

    /// Config overrides for this request.
    pub override_settings: Option<Config>,

    /// Report partial analysis results every this many seconds.
    pub report_during_search_every: Option<f64>,
}

impl AnalysisRequest {
    /// Creates a new analysis request with the minimum required parameters.
    pub fn new(
        id: String,
        rules: Rules,
        board_x_size: u8,
        board_y_size: u8,
        moves: Vec<(Player, String)>,
    ) -> Self {
        Self {
            id,
            rules,
            komi: None,
            white_handicap_bonus: None,
            board_x_size,
            board_y_size,
            initial_stones: None,
            initial_player: None,
            moves,
            analyze_turns: None,
            max_visits: None,
            override_settings: None,
            report_during_search_every: None,
        }
    }

    /// Sets komi.
    pub fn with_komi(mut self, komi: f64) -> Self {
        self.komi = Some(komi);
        self
    }

    /// Sets white's handicap bonus.
    pub fn with_white_handicap_bonus(mut self, bonus: Bonus) -> Self {
        self.white_handicap_bonus = Some(bonus);
        self
    }

    /// Sets the initial position before the first move.
    pub fn with_initial_stones(mut self, initial_stones: Vec<(Player, String)>) -> Self {
        self.initial_stones = Some(initial_stones);
        self
    }

    /// Sets the player to move in the initial position.
    pub fn with_initial_player(mut self, initial_player: Player) -> Self {
        self.initial_player = Some(initial_player);
        self
    }

    /// Analyzes the specified positions. The position before the first move is turn 0.
    pub fn with_analyze_turns(mut self, analyze_turns: Vec<usize>) -> Self {
        self.analyze_turns = Some(analyze_turns);
        self
    }

    /// Sets the maximum number of visits to use.
    pub fn with_max_visits(mut self, max_visits: u32) -> Self {
        self.max_visits = Some(max_visits);
        self
    }

    /// Overrides config settings for this request.
    pub fn with_override_settings(mut self, config: Config) -> Self {
        self.override_settings = Some(config);
        self
    }

    /// Gets partial analysis results every this many seconds.
    pub fn with_report_during_search_every(mut self, seconds: f64) -> Self {
        self.report_during_search_every = Some(seconds);
        self
    }
}
