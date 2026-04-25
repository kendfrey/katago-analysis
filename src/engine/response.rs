use serde::Deserialize;
use serde_json::{Map, Value};

/// A response from the analysis engine.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Value")]
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
        #[serde(default)]
        turn_numbers: Option<Vec<usize>>,
    },

    /// Acknowledgement of a request to terminate all analyses. The engine will proceed to send
    /// [`NoResults`][Response::NoResults] or partial [`Analyze`][Response::Analyze] responses for each position
    /// after they have been terminated.
    TerminateAll {
        /// The request ID.
        id: String,

        /// The positions being terminated, if specified in the request.
        #[serde(default)]
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

impl TryFrom<Value> for Response {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        fn try_parse_field_error(map: &Map<String, Value>) -> Option<Response> {
            let error = map.get("error")?.as_str()?;
            let field = map.get("field")?.as_str()?;
            let id = map.get("id")?.as_str()?;
            Some(Response::FieldError {
                id: id.to_string(),
                error: error.to_string(),
                field: field.to_string(),
            })
        }

        fn try_parse_general_error(map: &Map<String, Value>) -> Option<Response> {
            let error = map.get("error")?.as_str()?;
            Some(Response::GeneralError {
                error: error.to_string(),
            })
        }

        fn try_parse_field_warning(map: &Map<String, Value>) -> Option<Response> {
            let warning = map.get("warning")?.as_str()?;
            let field = map.get("field")?.as_str()?;
            let id = map.get("id")?.as_str()?;
            Some(Response::FieldWarning {
                id: id.to_string(),
                warning: warning.to_string(),
                field: field.to_string(),
            })
        }

        fn try_parse_query_version(map: &Map<String, Value>) -> Option<Response> {
            let action = map.get("action")?.as_str()?;
            if action != "query_version" {
                return None;
            }
            let id = map.get("id")?.as_str()?;
            let version = map.get("version")?.as_str()?;
            let git_hash = map.get("git_hash")?.as_str()?;
            Some(Response::QueryVersion {
                id: id.to_string(),
                version: version.to_string(),
                git_hash: git_hash.to_string(),
            })
        }

        fn try_parse_clear_cache(map: &Map<String, Value>) -> Option<Response> {
            let action = map.get("action")?.as_str()?;
            if action != "clear_cache" {
                return None;
            }
            let id = map.get("id")?.as_str()?;
            Some(Response::ClearCache { id: id.to_string() })
        }

        fn try_parse_no_results(map: &Map<String, Value>) -> Option<Response> {
            map.get("noResults")?;
            let id = map.get("id")?.as_str()?;
            let turn_number = map.get("turnNumber")?.as_u64()? as usize;
            Some(Response::NoResults {
                id: id.to_string(),
                turn_number,
            })
        }

        fn try_parse_terminate(map: &Map<String, Value>) -> Option<Response> {
            let action = map.get("action")?.as_str()?;
            if action != "terminate" {
                return None;
            }
            let id = map.get("id")?.as_str()?;
            let terminate_id = map.get("terminateId")?.as_str()?;
            let turn_numbers = map
                .get("turnNumbers")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            Some(Response::Terminate {
                id: id.to_string(),
                terminate_id: terminate_id.to_string(),
                turn_numbers,
            })
        }

        fn try_parse_terminate_all(map: &Map<String, Value>) -> Option<Response> {
            let action = map.get("action")?.as_str()?;
            if action != "terminate_all" {
                return None;
            }
            let id = map.get("id")?.as_str()?;
            let turn_numbers = map
                .get("turnNumbers")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            Some(Response::TerminateAll {
                id: id.to_string(),
                turn_numbers,
            })
        }

        fn try_parse_query_models(map: &Map<String, Value>) -> Option<Response> {
            let action = map.get("action")?.as_str()?;
            if action != "query_models" {
                return None;
            }
            let id = map.get("id")?.as_str()?;
            let models = map.get("models")?;
            Some(Response::QueryModels {
                id: id.to_string(),
                models: serde_json::from_value(models.clone()).ok()?,
            })
        }

        fn try_parse_analysis(value: Value) -> Option<Response> {
            serde_json::from_value(value).ok().map(Response::Analyze)
        }

        let map = value.as_object().ok_or("expected object")?;
        try_parse_field_error(map)
            .or_else(|| try_parse_general_error(map))
            .or_else(|| try_parse_field_warning(map))
            .or_else(|| try_parse_query_version(map))
            .or_else(|| try_parse_clear_cache(map))
            .or_else(|| try_parse_no_results(map))
            .or_else(|| try_parse_terminate(map))
            .or_else(|| try_parse_terminate_all(map))
            .or_else(|| try_parse_query_models(map))
            .or_else(|| try_parse_analysis(value))
            .ok_or("unrecognized response format".to_string())
    }
}

/// Information about a neural network model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// The model name.
    pub name: String,

    /// The internal name.
    pub internal_name: String,

    /// The maximum batch size.
    pub max_batch_size: usize,

    /// Whether it uses a humanSL profile.
    #[serde(rename = "usesHumanSLProfile")]
    pub uses_humansl_profile: bool,

    /// The model version.
    pub version: usize,

    /// Whether FP16 is used for this model. If this is [`Auto`][Enabled::Auto],
    /// it will be enabled if the backend deems it to be beneficial.
    #[serde(rename = "usingFP16")]
    pub using_fp16: Enabled,
}

/// The enabled state of a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Enabled {
    /// The feature is disabled.
    False,

    /// The feature is enabled.
    True,

    /// The feature will be automatically enabled or disabled based on what the engine thinks is best.
    Auto,
}

/// The result of analyzing a position.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResponse {
    /// The request ID.
    pub id: String,

    /// The position index, where 0 is the position before the first move.
    pub turn_number: usize,

    /// The list of moves the engine considered.
    pub move_infos: Vec<MoveInfo>,
}

/// The result of analyzing a candidate move.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveInfo {
    /// The move location in GTP format (`"A1"`, `"pass"`, etc.). This corresponds to the `move` field in KataGo's
    /// response.
    #[serde(rename = "move")]
    pub mv: String,

    /// The number of visits invested in this move.
    pub visits: usize,

    /// The winrate, in the range [0, 1].
    pub winrate: f64,

    /// The predicted number of points that the current side is leading by.
    pub score_lead: f64,
}
