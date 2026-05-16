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
}

impl AnalysisResult {
    /// Creates a result from the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn from_engine_response(response: AnalysisResponse, height: u8) -> Self {
        AnalysisResult {
            is_during_search: response.is_during_search,
            turn_number: response.turn_number,
            move_infos: response
                .move_infos
                .into_iter()
                .map(|info| MoveInfo::from_engine_move_info(info, height))
                .collect(),
            root_info: RootInfo::from_engine_root_info(response.root_info),
        }
    }
}
