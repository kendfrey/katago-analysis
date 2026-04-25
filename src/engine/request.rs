use serde::Serialize;
use serde_json::{Value, json};
use serde_with::skip_serializing_none;

use crate::{Config, Player, Rules};

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

    /// The board width.
    pub board_x_size: u8,

    /// The board height.
    pub board_y_size: u8,

    /// The moves played in the game. Move locations can be in GTP format (`"A1"`, `"pass"`, etc.) or explicit
    /// coordinates (`"(0,0)"`).
    pub moves: Vec<(Player, String)>,

    /// The positions to analyze, where 0 is the position before the first move.
    /// If not provided, only the final position will be analyzed.
    /// The engine will return a separate response for each position.
    pub analyze_turns: Option<Vec<usize>>,

    /// Config overrides for this request.
    pub override_settings: Option<Config>,
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
            board_x_size,
            board_y_size,
            moves,
            analyze_turns: None,
            override_settings: None,
        }
    }

    /// Analyzes the specified positions. The position before the first move is turn 0.
    pub fn with_analyze_turns(mut self, analyze_turns: Vec<usize>) -> Self {
        self.analyze_turns = Some(analyze_turns);
        self
    }

    /// Overrides config settings for this request.
    pub fn with_override_settings(mut self, config: Config) -> Self {
        self.override_settings = Some(config);
        self
    }
}
