use std::ops::Index;

use crate::{engine::AnalysisResponse, *};

/// The result of analyzing a position.
#[derive(Debug, Clone)]
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
}

impl AnalysisResult {
    /// Creates a result from the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_engine_response(response: AnalysisResponse, width: u8, height: u8) -> Self {
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
        }
    }
}

/// The result of analyzing a candidate move.
#[derive(Debug, Clone)]
pub struct MoveInfo {
    /// The move location in GTP format (`"A1"`, `"pass"`, etc.). This corresponds to the `move` field in KataGo's
    /// response.
    pub mv: Move,

    /// The number of visits invested in this move.
    pub visits: u32,

    /// The winrate, in the range [0, 1].
    pub winrate: f64,

    /// The predicted number of points that the current side is leading by.
    pub score_lead: f64,

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
            winrate: info.winrate,
            score_lead: info.score_lead,
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
#[derive(Debug, Clone)]
pub struct RootInfo {
    /// The winrate, in the range [0, 1].
    pub winrate: f64,

    /// The number of visits received.
    pub visits: u32,
}

impl RootInfo {
    /// Creates a root analysis from the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_engine_root_info(info: engine::RootInfo) -> Self {
        RootInfo {
            winrate: info.winrate,
            visits: info.visits,
        }
    }
}

/// A 2D matrix representing the game board.
///
/// (0, 0) is the top-left corner of the board.
#[derive(Debug, Clone)]
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
