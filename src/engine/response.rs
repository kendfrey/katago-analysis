use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serde_with::skip_serializing_none;

use crate::{Model, Player};

/// A response from the analysis engine.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "Value", try_from = "Value")]
#[expect(
    clippy::large_enum_variant,
    reason = "Boxing AnalysisResponse would be inconvenient, and very little would be gained"
)]
pub enum Response {
    /// The result of analyzing a position.
    Analyze(AnalysisResponse),

    /// Indicates that analysis was terminated before analyzing the specified position.
    NoResults {
        /// The request ID.
        id: String,

        /// The position index, where 0 is the position before the first move.
        turn_number: usize,
    },

    /// KataGo's version information.
    QueryVersion {
        /// The request ID.
        id: String,

        /// A string indicating the most recent KataGo release version that this version is a descendant of,
        /// such as `"1.6.1"`.
        version: String,

        /// The precise git hash this KataGo version was compiled from, or the string `"<omitted>"` if KataGo was
        /// compiled separately from its repo or without Git support.
        git_hash: String,
    },

    /// Indicates that the cache was cleared.
    ClearCache {
        /// The request ID.
        id: String,
    },

    /// Acknowledgement of a terminate request. The engine will proceed to send [`NoResults`][Response::NoResults] or
    /// partial [`Analyze`][Response::Analyze] responses for each position after they have been terminated.
    Terminate {
        /// The request ID.
        id: String,

        /// The ID of the request being terminated.
        terminate_id: String,

        /// The positions being terminated, if specified in the request.
        turn_numbers: Option<Vec<usize>>,
    },

    /// Acknowledgement of a request to terminate all analyses. The engine will proceed to send
    /// [`NoResults`][Response::NoResults] or partial [`Analyze`][Response::Analyze] responses for each position
    /// after they have been terminated.
    TerminateAll {
        /// The request ID.
        id: String,

        /// The positions being terminated, if specified in the request.
        turn_numbers: Option<Vec<usize>>,
    },

    /// Information about the currently loaded neural network models.
    QueryModels {
        /// The request ID.
        id: String,

        /// A list of available models.
        models: Vec<Model>,
    },

    /// An error with no known associated request.
    GeneralError {
        /// The error message.
        error: String,
    },

    /// An error in processing a request.
    FieldError {
        /// The request ID.
        id: String,

        /// The error message.
        error: String,

        /// The request field which is the source of the error.
        field: String,
    },

    /// A warning in processing a request. The engine will still generate analysis responses for the request.
    FieldWarning {
        /// The request ID.
        id: String,

        /// The warning message.
        warning: String,

        /// The request field which is the source of the warning.
        field: String,
    },
}

impl From<Response> for Value {
    fn from(response: Response) -> Self {
        match response {
            Response::Analyze(response) => {
                serde_json::to_value(response).expect("response should be serializable")
            }
            Response::NoResults { id, turn_number } => json!({
                "id": id,
                "isDuringSearch": false,
                "noResults": true,
                "turnNumber": turn_number,
            }),
            Response::QueryVersion {
                id,
                version,
                git_hash,
            } => json!({
                "id": id,
                "action": "query_version",
                "version": version,
                "git_hash": git_hash,
            }),
            Response::ClearCache { id } => json!({
                "id": id,
                "action": "clear_cache",
            }),
            Response::Terminate {
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
                        .expect("value should be an object")
                        .insert("turnNumbers".to_string(), json!(turn_numbers));
                }
                value
            }
            Response::TerminateAll { id, turn_numbers } => {
                let mut value = json!({
                        "id": id,
                        "action": "terminate_all",
                    }
                );
                if let Some(turn_numbers) = turn_numbers {
                    value
                        .as_object_mut()
                        .expect("value should be an object")
                        .insert("turnNumbers".to_string(), json!(turn_numbers));
                }
                value
            }
            Response::QueryModels { id, models } => json!({
                "id": id,
                "action": "query_models",
                "models": models,
            }),
            Response::GeneralError { error } => json!({
                "error": error,
            }),
            Response::FieldError { id, error, field } => json!({
                "id": id,
                "error": error,
                "field": field,
            }),
            Response::FieldWarning { id, warning, field } => json!({
                "id": id,
                "warning": warning,
                "field": field,
            }),
        }
    }
}

impl TryFrom<Value> for Response {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        deserialize_field!(id, String);
        deserialize_field!(version, String);
        deserialize_field!(git_hash, String, rename = "git_hash");
        deserialize_field!(terminate_id, String);
        deserialize_field!(turn_numbers, Option<Vec<usize>>);
        deserialize_field!(models, Vec<Model>);
        deserialize_field!(error, String);
        deserialize_field!(warning, String);
        deserialize_field!(field, String);
        deserialize_field!(turn_number, usize);

        match value.get("action").and_then(|v| v.as_str()) {
            Some("query_version") => Ok(Response::QueryVersion {
                id: id(&value)?,
                version: version(&value)?,
                git_hash: git_hash(&value)?,
            }),
            Some("clear_cache") => Ok(Response::ClearCache { id: id(&value)? }),
            Some("terminate") => Ok(Response::Terminate {
                id: id(&value)?,
                terminate_id: terminate_id(&value)?,
                turn_numbers: turn_numbers(&value)?,
            }),
            Some("terminate_all") => Ok(Response::TerminateAll {
                id: id(&value)?,
                turn_numbers: turn_numbers(&value)?,
            }),
            Some("query_models") => Ok(Response::QueryModels {
                id: id(&value)?,
                models: models(&value)?,
            }),
            _ => {
                if let Some(map) = value.as_object() {
                    if map.contains_key("error") {
                        return if map.contains_key("id") {
                            Ok(Response::FieldError {
                                id: id(&value)?,
                                error: error(&value)?,
                                field: field(&value)?,
                            })
                        } else {
                            Ok(Response::GeneralError {
                                error: error(&value)?,
                            })
                        };
                    } else if map.contains_key("warning") {
                        return Ok(Response::FieldWarning {
                            id: id(&value)?,
                            warning: warning(&value)?,
                            field: field(&value)?,
                        });
                    } else if map.contains_key("noResults") {
                        return Ok(Response::NoResults {
                            id: id(&value)?,
                            turn_number: turn_number(&value)?,
                        });
                    }
                }
                serde_json::from_value(value).map(Response::Analyze)
            }
        }
    }
}

/// The result of analyzing a position.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResponse {
    /// The request ID.
    pub id: String,

    /// Whether this is a partial analysis result. `false` indicates no more responses will be sent.
    pub is_during_search: bool,

    /// The position index, where 0 is the position before the first move.
    pub turn_number: usize,

    /// The list of moves the engine considered.
    pub move_infos: Vec<MoveInfo>,

    /// Information about the root position.
    pub root_info: RootInfo,

    /// The ownership prediction, in row-major order.
    pub ownership: Option<Vec<f64>>,

    /// The standard deviation of the ownership prediction, in row-major order.
    pub ownership_stdev: Option<Vec<f64>>,

    /// The policy prediction, in row-major order with the pass move at the end.
    pub policy: Option<Vec<f64>>,

    /// The humanSL policy prediction, in row-major order with the pass move at the end.
    pub human_policy: Option<Vec<f64>>,
}

/// The result of analyzing a candidate move.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveInfo {
    /// The move location in GTP format (`"A1"`, `"pass"`, etc.). This corresponds to the `move` field in KataGo's
    /// response.
    #[serde(rename = "move")]
    pub mv: String,

    /// The number of visits invested in this move.
    pub visits: u32,

    /// The number of visits the root "wants" to invest in this move.
    pub edge_visits: u32,

    /// The winrate, in the range [0, 1].
    pub winrate: f64,

    /// The predicted number of points that the current side is leading by.
    pub score_lead: f64,

    /// The predicted standard deviation of the score lead.
    pub score_stdev: f64,

    /// The predicted score at the end of the game after selfplay.
    pub score_selfplay: f64,

    /// The policy prior of this move.
    pub prior: f64,

    /// The predicted probability that the game will have a void result.
    pub no_result_value: Option<f64>,

    /// The humanSL policy prior of this move.
    pub human_prior: Option<f64>,

    /// The utility of this move.
    pub utility: f64,

    /// The LCB of this move's winrate.
    pub lcb: f64,

    /// The LCB of this move's utility.
    pub utility_lcb: f64,

    /// The total weight of this move's visits.
    pub weight: f64,

    /// The total weight of the visits the root "wants" to invest in this move.
    pub edge_weight: f64,

    /// The relative ranking of this move, where 0 is best.
    pub order: usize,

    /// The value used to determine the move ranking.
    pub play_selection_value: f64,

    /// If present, indicates the move that was actually searched to get the evaluation of this move.
    pub is_symmetry_of: Option<String>,

    /// The principal variation for this move.
    pub pv: Vec<String>,

    /// The number of visits invested in each position in the principal variation.
    pub pv_visits: Option<Vec<u32>>,

    /// The number of visits invested in each move in the principal variation.
    pub pv_edge_visits: Option<Vec<u32>>,

    /// The ownership prediction, in row-major order.
    pub ownership: Option<Vec<f64>>,

    /// The standard deviation of the ownership prediction, in row-major order.
    pub ownership_stdev: Option<Vec<f64>>,
}

/// The result of analyzing the root position.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootInfo {
    /// The winrate, in the range [0, 1].
    pub winrate: f64,

    /// The predicted number of points that the current side is leading by.
    pub score_lead: f64,

    /// The predicted score at the end of the game after selfplay.
    pub score_selfplay: f64,

    /// The utility.
    pub utility: f64,

    /// The number of visits received.
    pub visits: u32,

    /// The hash of this position.
    pub this_hash: String,

    /// The hash of this position that is invariant under board symmetries.
    pub sym_hash: String,

    /// The player to move.
    pub current_player: Player,

    /// The winrate prediction from the neural network.
    pub raw_winrate: f64,

    /// The score lead prediction from the neural network.
    pub raw_lead: f64,

    /// The selfplay score prediction from the neural network.
    pub raw_score_selfplay: f64,

    /// The selfplay score standard deviation prediction from the neural network.
    pub raw_score_selfplay_stdev: f64,

    /// The void result probability prediction from the neural network.
    pub raw_no_result_prob: f64,

    /// The short-term winrate uncertainty prediction from the neural network.
    pub raw_st_wr_error: f64,

    /// The short-term score uncertainty prediction from the neural network.
    pub raw_st_score_error: f64,

    /// A measure of how much meaningful game is left until the winner is known, predicted by the neural network.
    pub raw_var_time_left: f64,

    /// The winrate prediction from the humanSL neural network.
    pub human_winrate: Option<f64>,

    /// The score prediction from the humanSL neural network.
    pub human_score_mean: Option<f64>,

    /// The score standard deviation prediction from the humanSL neural network.
    pub human_score_stdev: Option<f64>,

    /// The short-term winrate uncertainty prediction from the humanSL neural network.
    pub human_st_wr_error: Option<f64>,

    /// The short-term score uncertainty prediction from the humanSL neural network.
    pub human_st_score_error: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Enabled;

    #[test]
    fn response_serialize_deserialize() {
        let response = Response::Analyze(AnalysisResponse {
            id: "request1".to_string(),
            is_during_search: false,
            turn_number: 0,
            move_infos: vec![
                MoveInfo {
                    mv: "D4".to_string(),
                    visits: 10,
                    edge_visits: 100,
                    winrate: 0.5,
                    score_lead: 0.0,
                    score_stdev: 1.0,
                    score_selfplay: 0.0,
                    prior: 0.1,
                    no_result_value: Some(0.0),
                    human_prior: Some(0.1),
                    utility: 0.5,
                    lcb: 0.0,
                    utility_lcb: 0.0,
                    weight: 100.0,
                    edge_weight: 1000.0,
                    order: 0,
                    play_selection_value: 3.7,
                    is_symmetry_of: None,
                    pv: vec!["D4".to_string(), "pass".to_string()],
                    pv_visits: Some(vec![5, 1]),
                    pv_edge_visits: Some(vec![50, 10]),
                    ownership: Some(vec![0.0, 0.5, 1.0]),
                    ownership_stdev: Some(vec![0.0, 0.1, 0.2]),
                },
                MoveInfo {
                    mv: "Q16".to_string(),
                    visits: 10,
                    edge_visits: 100,
                    winrate: 0.5,
                    score_lead: 0.0,
                    score_stdev: 1.0,
                    score_selfplay: 0.0,
                    prior: 0.1,
                    no_result_value: None,
                    human_prior: Some(0.1),
                    utility: 0.5,
                    lcb: 0.0,
                    utility_lcb: 0.0,
                    weight: 100.0,
                    edge_weight: 1000.0,
                    order: 0,
                    play_selection_value: 3.7,
                    is_symmetry_of: Some("D4".to_string()),
                    pv: vec![],
                    pv_visits: None,
                    pv_edge_visits: None,
                    ownership: None,
                    ownership_stdev: None,
                },
            ],
            root_info: RootInfo {
                winrate: 0.5,
                score_lead: 0.0,
                score_selfplay: 0.0,
                utility: 0.5,
                visits: 100,
                this_hash: "b".to_string(),
                sym_hash: "a".to_string(),
                current_player: Player::Black,
                raw_winrate: 0.5,
                raw_lead: 0.0,
                raw_score_selfplay: 0.0,
                raw_score_selfplay_stdev: 1.0,
                raw_no_result_prob: 0.0,
                raw_st_wr_error: 0.0,
                raw_st_score_error: 0.0,
                raw_var_time_left: 4.2,
                human_winrate: Some(0.5),
                human_score_mean: Some(0.0),
                human_score_stdev: None,
                human_st_wr_error: None,
                human_st_score_error: None,
            },
            ownership: Some(vec![0.0, 0.5, 1.0]),
            ownership_stdev: None,
            policy: None,
            human_policy: None,
        });
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn no_results_serialize_deserialize() {
        let response = Response::NoResults {
            id: "request1".to_string(),
            turn_number: 1,
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn query_version_serialize_deserialize() {
        let response = Response::QueryVersion {
            id: "request1".to_string(),
            version: "1.0".to_string(),
            git_hash: "1234".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn clear_cache_serialize_deserialize() {
        let response = Response::ClearCache {
            id: "request1".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn terminate_serialize_deserialize() {
        let response = Response::Terminate {
            id: "request1".to_string(),
            terminate_id: "request".to_string(),
            turn_numbers: None,
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);

        let response = Response::Terminate {
            id: "request1".to_string(),
            terminate_id: "request".to_string(),
            turn_numbers: Some(vec![0, 1]),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn terminate_all_serialize_deserialize() {
        let response = Response::TerminateAll {
            id: "request1".to_string(),
            turn_numbers: None,
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);

        let response = Response::TerminateAll {
            id: "request1".to_string(),
            turn_numbers: Some(vec![0, 1]),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn query_models_serialize_deserialize() {
        let response = Response::QueryModels {
            id: "request1".to_string(),
            models: vec![
                Model {
                    name: "model1".to_string(),
                    internal_name: "model1-a".to_string(),
                    max_batch_size: 100,
                    uses_humansl_profile: false,
                    version: 1,
                    using_fp16: Enabled::Auto,
                },
                Model {
                    name: "model2".to_string(),
                    internal_name: "model2-a".to_string(),
                    max_batch_size: 1000,
                    uses_humansl_profile: true,
                    version: 2,
                    using_fp16: Enabled::True,
                },
            ],
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn general_error_serialize_deserialize() {
        let response = Response::GeneralError {
            error: "There was an error".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn field_error_serialize_deserialize() {
        let response = Response::FieldError {
            id: "request1".to_string(),
            error: "There was an error".to_string(),
            field: "someField".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn field_warning_serialize_deserialize() {
        let response = Response::FieldWarning {
            id: "request1".to_string(),
            warning: "There was a warning".to_string(),
            field: "someField".to_string(),
        };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);
    }
}
