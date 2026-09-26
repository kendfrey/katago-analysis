use std::ops::Index;

use crate::{engine::AnalysisResponse, *};

/// The result of analyzing a position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    /// Whether this is a partial analysis result. `false` indicates the position is finished analyzing.
    pub is_during_search: bool,

    /// The position index, where 0 is the position before the first move.
    pub turn_number: usize,

    /// The list of moves the engine considered.
    pub move_infos: Vec<MoveInfo>,

    /// Information about the root position.
    pub root_info: RootInfo,

    /// The ownership prediction.
    pub ownership: Option<Matrix<f64>>,

    /// The standard deviation of the ownership prediction.
    pub ownership_stdev: Option<Matrix<f64>>,

    /// The policy prediction.
    pub policy: Option<Matrix<f64>>,

    /// The pass policy prediction.
    pub policy_pass: Option<f64>,

    /// The humanSL policy prediction.
    pub human_policy: Option<Matrix<f64>>,

    /// The humanSL pass policy prediction.
    pub human_policy_pass: Option<f64>,
}

impl AnalysisResult {
    /// Creates a result from the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_engine_response(mut response: AnalysisResponse, width: u8, height: u8) -> Self {
        AnalysisResult {
            is_during_search: response.is_during_search,
            turn_number: response.turn_number,
            move_infos: response
                .move_infos
                .into_iter()
                .map(|info| MoveInfo::from_engine_move_info(info, width, height))
                .collect(),
            root_info: RootInfo::from_engine_root_info(response.root_info),
            ownership: response.ownership.map(|m| Matrix::from_raw(m, width)),
            ownership_stdev: response.ownership_stdev.map(|m| Matrix::from_raw(m, width)),
            policy_pass: response.policy.as_mut().and_then(|p| p.pop()),
            policy: response.policy.map(|p| Matrix::from_raw(p, width)),
            human_policy_pass: response.human_policy.as_mut().and_then(|p| p.pop()),
            human_policy: response.human_policy.map(|p| Matrix::from_raw(p, width)),
        }
    }
}

/// The result of analyzing a candidate move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveInfo {
    /// The move location. This corresponds to the `move` field in KataGo's
    /// response.
    #[serde(rename = "move")]
    pub mv: Move,

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
    pub is_symmetry_of: Option<Coord>,

    /// The principal variation for this move.
    pub pv: Vec<Move>,

    /// The number of visits invested in each position in the principal variation.
    pub pv_visits: Option<Vec<u32>>,

    /// The number of visits invested in each move in the principal variation.
    pub pv_edge_visits: Option<Vec<u32>>,

    /// The ownership prediction.
    pub ownership: Option<Matrix<f64>>,

    /// The standard deviation of the ownership prediction.
    pub ownership_stdev: Option<Matrix<f64>>,
}

impl MoveInfo {
    /// Creates a move analysis from the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_engine_move_info(info: engine::MoveInfo, width: u8, height: u8) -> Self {
        MoveInfo {
            mv: Move::from_gtp(&info.mv, height).expect("invalid move"),
            visits: info.visits,
            edge_visits: info.edge_visits,
            winrate: info.winrate,
            score_lead: info.score_lead,
            score_stdev: info.score_stdev,
            score_selfplay: info.score_selfplay,
            prior: info.prior,
            no_result_value: info.no_result_value,
            human_prior: info.human_prior,
            utility: info.utility,
            lcb: info.lcb,
            utility_lcb: info.utility_lcb,
            weight: info.weight,
            edge_weight: info.edge_weight,
            order: info.order,
            play_selection_value: info.play_selection_value,
            is_symmetry_of: info
                .is_symmetry_of
                .map(|c| Coord::from_gtp(&c, height).expect("invalid move")),
            pv: info
                .pv
                .into_iter()
                .map(|mv| Move::from_gtp(&mv, height).expect("invalid move"))
                .collect(),
            pv_visits: info.pv_visits,
            pv_edge_visits: info.pv_edge_visits,
            ownership: info.ownership.map(|m| Matrix::from_raw(m, width)),
            ownership_stdev: info.ownership_stdev.map(|m| Matrix::from_raw(m, width)),
        }
    }
}

/// The result of analyzing the root position.
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

impl RootInfo {
    /// Creates a root analysis from the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_engine_root_info(info: engine::RootInfo) -> Self {
        RootInfo {
            winrate: info.winrate,
            score_lead: info.score_lead,
            score_selfplay: info.score_selfplay,
            utility: info.utility,
            visits: info.visits,
            this_hash: info.this_hash,
            sym_hash: info.sym_hash,
            current_player: info.current_player,
            raw_winrate: info.raw_winrate,
            raw_lead: info.raw_lead,
            raw_score_selfplay: info.raw_score_selfplay,
            raw_score_selfplay_stdev: info.raw_score_selfplay_stdev,
            raw_no_result_prob: info.raw_no_result_prob,
            raw_st_wr_error: info.raw_st_wr_error,
            raw_st_score_error: info.raw_st_score_error,
            raw_var_time_left: info.raw_var_time_left,
            human_winrate: info.human_winrate,
            human_score_mean: info.human_score_mean,
            human_score_stdev: info.human_score_stdev,
            human_st_wr_error: info.human_st_wr_error,
            human_st_score_error: info.human_st_score_error,
        }
    }
}

/// A 2D matrix representing the game board.
///
/// (0, 0) is the top-left corner of the board.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Matrix<T> {
    stride: usize,

    /// The raw data stored in row-major order.
    pub raw: Vec<T>,
}

impl<T> Matrix<T> {
    /// Gets the value at the given coordinates.
    pub fn get(&self, x: u8, y: u8) -> &T {
        &self.raw[(y as usize) * self.stride + (x as usize)]
    }

    /// Creates a matrix from the raw data.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_raw(raw: Vec<T>, stride: u8) -> Self {
        Self {
            raw,
            stride: stride as usize,
        }
    }
}

impl<T> Index<Coord> for Matrix<T> {
    type Output = T;

    fn index(&self, Coord(x, y): Coord) -> &Self::Output {
        self.get(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_serialize_deserialize() {
        let result = AnalysisResult {
            is_during_search: false,
            turn_number: 0,
            move_infos: vec![],
            root_info: RootInfo {
                winrate: 0.5,
                score_lead: 0.0,
                score_selfplay: 0.0,
                utility: 0.5,
                visits: 100,
                this_hash: "b".to_string(),
                sym_hash: "a".to_string(),
                current_player: Player::Black,
                raw_winrate: 0.0,
                raw_lead: 0.0,
                raw_score_selfplay: 0.0,
                raw_score_selfplay_stdev: 0.0,
                raw_no_result_prob: 0.0,
                raw_st_wr_error: 0.0,
                raw_st_score_error: 0.0,
                raw_var_time_left: 0.0,
                human_winrate: None,
                human_score_mean: None,
                human_score_stdev: None,
                human_st_wr_error: None,
                human_st_score_error: None,
            },
            ownership: None,
            ownership_stdev: None,
            policy: None,
            policy_pass: None,
            human_policy: None,
            human_policy_pass: None,
        };
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: AnalysisResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(result, deserialized);

        let result = AnalysisResult {
            is_during_search: true,
            turn_number: 1,
            move_infos: vec![
                MoveInfo {
                    mv: Move::Move(Coord(1, 0)),
                    visits: 900,
                    edge_visits: 9000,
                    winrate: 1.0,
                    score_lead: 1.0,
                    score_stdev: 1.0,
                    score_selfplay: 1.0,
                    prior: 0.4,
                    no_result_value: Some(0.1),
                    human_prior: Some(0.5),
                    utility: 1.0,
                    lcb: 1.0,
                    utility_lcb: 1.0,
                    weight: 1.0,
                    edge_weight: 1.0,
                    order: 0,
                    play_selection_value: 100.0,
                    is_symmetry_of: Some(Coord(1, 1)),
                    pv: vec![Move::Move(Coord(1, 1)), Move::Pass],
                    pv_visits: Some(vec![100, 10]),
                    pv_edge_visits: Some(vec![1000, 100]),
                    ownership: Some(Matrix::from_raw(vec![1.0, 0.0, 0.9, 0.1], 2)),
                    ownership_stdev: Some(Matrix::from_raw(vec![0.0, 0.0, 0.1, 0.1], 2)),
                },
                MoveInfo {
                    mv: Move::Pass,
                    visits: 100,
                    edge_visits: 0,
                    winrate: 0.9,
                    score_lead: 0.0,
                    score_stdev: 0.0,
                    score_selfplay: 0.0,
                    prior: 0.2,
                    no_result_value: Some(0.0),
                    human_prior: Some(0.4),
                    utility: 0.0,
                    lcb: 0.0,
                    utility_lcb: 0.0,
                    weight: 0.0,
                    edge_weight: 0.0,
                    order: 1,
                    play_selection_value: 10.0,
                    is_symmetry_of: None,
                    pv: vec![],
                    pv_visits: Some(vec![]),
                    pv_edge_visits: Some(vec![]),
                    ownership: Some(Matrix::from_raw(vec![1.0, 0.0, 0.9, 0.1], 2)),
                    ownership_stdev: Some(Matrix::from_raw(vec![0.0, 0.0, 0.1, 0.1], 2)),
                },
            ],
            root_info: RootInfo {
                winrate: 1.0,
                score_lead: 4.2,
                score_selfplay: 3.7,
                utility: 1.0,
                visits: 1000,
                this_hash: "2".to_string(),
                sym_hash: "1".to_string(),
                current_player: Player::White,
                raw_winrate: 0.9,
                raw_lead: 4.0,
                raw_score_selfplay: 3.0,
                raw_score_selfplay_stdev: 0.1,
                raw_no_result_prob: 0.1,
                raw_st_wr_error: 0.1,
                raw_st_score_error: 0.1,
                raw_var_time_left: 3.14,
                human_winrate: Some(0.8),
                human_score_mean: Some(3.5),
                human_score_stdev: Some(0.2),
                human_st_wr_error: Some(0.1),
                human_st_score_error: Some(0.1),
            },
            ownership: Some(Matrix::from_raw(vec![1.0, 0.0, 0.9, 0.1], 2)),
            ownership_stdev: Some(Matrix::from_raw(vec![0.0, 0.0, 0.1, 0.1], 2)),
            policy: Some(Matrix::from_raw(vec![-1.0, -1.0, 0.4, 0.4], 2)),
            policy_pass: Some(0.2),
            human_policy: Some(Matrix::from_raw(vec![-1.0, -1.0, 0.5, 0.1], 2)),
            human_policy_pass: Some(0.4),
        };
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: AnalysisResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(result, deserialized);
    }
}
