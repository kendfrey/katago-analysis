#[cfg(feature = "sgf-parse")]
use sgf_parse::{SgfNode, go::Prop};

use crate::*;

/// A game record to be analyzed, along with analysis settings.
#[derive(Debug, Clone)]
pub struct AnalysisRequest {
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
    pub initial_stones: Option<Vec<(Player, Coord)>>,

    /// The player to move in the initial position.
    pub initial_player: Option<Player>,

    /// The moves played in the game.
    pub moves: Vec<(Player, Move)>,

    /// The maximum number of visits to use.
    pub max_visits: Option<u32>,

    /// Root policy temperature.
    pub root_policy_temperature: Option<f64>,

    /// Root FPU reduction max.
    pub root_fpu_reduction_max: Option<f64>,

    /// The maximum length of the principal variation to return, not including the first move.
    pub analysis_pv_len: Option<usize>,

    /// Whether to return the ownership prediction.
    pub include_ownership: bool,

    /// Whether to return the standard deviation of the ownership prediction.
    pub include_ownership_stdev: bool,

    /// Whether to return the ownership prediction for each move.
    pub include_moves_ownership: bool,

    /// Whether to return the standard deviation of the ownership prediction for each move.
    pub include_moves_ownership_stdev: bool,

    /// Whether to return the neural network policy output.
    pub include_policy: bool,

    /// Whether to return the number of visits for each position in the principal variation.
    pub include_pv_visits: bool,

    /// Whether to return the predicted probability that the game will have a void result.
    pub include_no_result_value: bool,

    /// Moves which are forbidden.
    pub avoid_moves: Option<Vec<RestrictedMoves>>,

    /// Moves which are allowed. If specified, all other moves are forbidden.
    pub allow_moves: Option<Vec<RestrictedMoves>>,

    /// Config overrides for this request.
    pub override_settings: Option<Config>,

    /// Report partial analysis results every this many seconds.
    pub report_during_search_every: Option<f64>,

    /// The priority of this request.
    pub priority: Option<i32>,
}

impl AnalysisRequest {
    /// Creates a new analysis request with the minimum required parameters.
    pub fn new(
        rules: Rules,
        board_x_size: u8,
        board_y_size: u8,
        moves: Vec<(Player, Move)>,
    ) -> Self {
        Self {
            rules,
            komi: None,
            white_handicap_bonus: None,
            board_x_size,
            board_y_size,
            initial_stones: None,
            initial_player: None,
            moves,
            max_visits: None,
            root_policy_temperature: None,
            root_fpu_reduction_max: None,
            analysis_pv_len: None,
            include_ownership: false,
            include_ownership_stdev: false,
            include_moves_ownership: false,
            include_moves_ownership_stdev: false,
            include_policy: false,
            include_pv_visits: false,
            include_no_result_value: false,
            avoid_moves: None,
            allow_moves: None,
            override_settings: None,
            report_during_search_every: None,
            priority: None,
        }
    }

    /// Converts this request into the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn into_engine_request(
        self,
        id: String,
        analyze_turns: Vec<usize>,
        priorities: Option<Vec<i32>>,
    ) -> engine::AnalysisRequest {
        engine::AnalysisRequest {
            id,
            rules: self.rules,
            komi: self.komi,
            white_handicap_bonus: self.white_handicap_bonus,
            board_x_size: self.board_x_size,
            board_y_size: self.board_y_size,
            initial_stones: self.initial_stones.map(|s| {
                s.into_iter()
                    .map(|(p, c)| (p, c.to_gtp(self.board_y_size)))
                    .collect()
            }),
            initial_player: self.initial_player,
            moves: self
                .moves
                .into_iter()
                .map(|(p, m)| (p, m.to_gtp(self.board_y_size)))
                .collect(),
            analyze_turns: Some(analyze_turns),
            max_visits: self.max_visits,
            root_policy_temperature: self.root_policy_temperature,
            root_fpu_reduction_max: self.root_fpu_reduction_max,
            analysis_pv_len: self.analysis_pv_len,
            include_ownership: self.include_ownership,
            include_ownership_stdev: self.include_ownership_stdev,
            include_moves_ownership: self.include_moves_ownership,
            include_moves_ownership_stdev: self.include_moves_ownership_stdev,
            include_policy: self.include_policy,
            include_pv_visits: self.include_pv_visits,
            include_no_result_value: self.include_no_result_value,
            avoid_moves: self.avoid_moves.map(|m| {
                m.into_iter()
                    .map(|rm| rm.into_engine_restricted_moves(self.board_y_size))
                    .collect()
            }),
            allow_moves: self.allow_moves.map(|m| {
                m.into_iter()
                    .map(|rm| rm.into_engine_restricted_moves(self.board_y_size))
                    .collect()
            }),
            override_settings: self.override_settings,
            report_during_search_every: self.report_during_search_every,
            priority: self.priority,
            priorities,
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
    pub fn with_initial_stones(mut self, initial_stones: Vec<(Player, Coord)>) -> Self {
        self.initial_stones = Some(initial_stones);
        self
    }

    /// Sets the player to move in the initial position.
    pub fn with_initial_player(mut self, initial_player: Player) -> Self {
        self.initial_player = Some(initial_player);
        self
    }

    /// Sets the maximum number of visits to use.
    pub fn with_max_visits(mut self, max_visits: u32) -> Self {
        self.max_visits = Some(max_visits);
        self
    }

    /// Sets the root policy temperature.
    pub fn with_root_policy_temperature(mut self, root_policy_temperature: f64) -> Self {
        self.root_policy_temperature = Some(root_policy_temperature);
        self
    }

    /// Sets the root FPU reduction max.
    pub fn with_root_fpu_reduction_max(mut self, root_fpu_reduction_max: f64) -> Self {
        self.root_fpu_reduction_max = Some(root_fpu_reduction_max);
        self
    }

    /// Sets the maximum length of the principal variation to return, not including the first move.
    pub fn with_analysis_pv_len(mut self, analysis_pv_len: usize) -> Self {
        self.analysis_pv_len = Some(analysis_pv_len);
        self
    }

    /// Includes the ownership prediction.
    pub fn with_ownership(mut self) -> Self {
        self.include_ownership = true;
        self
    }

    /// Includes the standard deviation of the ownership prediction.
    pub fn with_ownership_stdev(mut self) -> Self {
        self.include_ownership_stdev = true;
        self
    }

    /// Includes the ownership prediction for each move.
    pub fn with_moves_ownership(mut self) -> Self {
        self.include_moves_ownership = true;
        self
    }

    /// Includes the standard deviation of the ownership prediction for each move.
    pub fn with_moves_ownership_stdev(mut self) -> Self {
        self.include_moves_ownership_stdev = true;
        self
    }

    /// Includes the neural network policy output.
    pub fn with_policy(mut self) -> Self {
        self.include_policy = true;
        self
    }

    /// Includes the number of visits for each position in the principal variation.
    pub fn with_pv_visits(mut self) -> Self {
        self.include_pv_visits = true;
        self
    }

    /// Includes the predicted probability that the game will have a void result.
    pub fn with_no_result_value(mut self) -> Self {
        self.include_no_result_value = true;
        self
    }

    /// Sets moves which are forbidden.
    pub fn with_avoid_moves(mut self, avoid_moves: Vec<RestrictedMoves>) -> Self {
        self.avoid_moves = Some(avoid_moves);
        self
    }

    /// Sets moves which are allowed.
    pub fn with_allow_moves(mut self, allow_moves: Vec<RestrictedMoves>) -> Self {
        self.allow_moves = Some(allow_moves);
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

    /// Sets the priority of this request.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }
}

#[cfg(feature = "sgf-parse")]
impl From<&SgfNode<Prop>> for AnalysisRequest {
    /// Creates an analysis request from the root [`SgfNode`] of a game tree.
    ///
    /// This will set [`rules`](AnalysisRequest::rules), [`komi`](AnalysisRequest::komi) (if present),
    /// [`board_x_size`](AnalysisRequest::board_x_size), [`board_y_size`](AnalysisRequest::board_y_size),
    /// [`initial_stones`](AnalysisRequest::initial_stones) (if present),
    /// [`initial_player`](AnalysisRequest::initial_player) (if present), and [`moves`](AnalysisRequest::moves),
    /// based on the SGF data.
    ///
    /// Rules are determined by the first of the following that applies:
    /// - If `RU` is present, its value will be used as a [named ruleset](Rules::Named).
    /// - If `KM` is present and greater than 6.5, [Chinese rules](Rules::chinese) will be used.
    /// - Otherwise, [Japanese rules](Rules::japanese) will be used.
    fn from(root: &SgfNode<Prop>) -> Self {
        let (width, height) = match root.get_property("SZ") {
            Some(Prop::SZ((w, h))) => (*w, *h),
            _ => (19, 19),
        };

        let komi = match root.get_property("KM") {
            Some(Prop::KM(k)) => Some(*k),
            _ => None,
        };

        let rules = match root.get_property("RU") {
            Some(Prop::RU(r)) => Rules::Named(r.text.clone()),
            _ => match komi {
                Some(k) if k > 6.5 => Rules::chinese(),
                _ => Rules::japanese(),
            },
        };

        let moves: Vec<(Player, Move)> = root
            .main_variation()
            .filter_map(|m| match m.get_move() {
                Some(Prop::B(m)) => Some((Player::Black, (*m).into())),
                Some(Prop::W(m)) => Some((Player::White, (*m).into())),
                _ => None,
            })
            .collect();

        let mut initial_stones: Vec<(Player, Coord)> = vec![];
        if let Some(Prop::AB(ps)) = root.get_property("AB") {
            initial_stones.extend(ps.iter().map(|p| (Player::Black, (*p).into())));
        }
        if let Some(Prop::AW(ps)) = root.get_property("AW") {
            initial_stones.extend(ps.iter().map(|p| (Player::White, (*p).into())));
        }

        let initial_player: Option<Player> = match root.get_property("PL") {
            Some(Prop::PL(p)) => Some((*p).into()),
            _ => None,
        };

        let mut request = Self::new(rules, width, height, moves);
        request.komi = komi;
        if !initial_stones.is_empty() {
            request = request.with_initial_stones(initial_stones);
        }
        request.initial_player = initial_player;
        request
    }
}

/// A list of moves that are either forbidden with [`AnalysisRequest::avoid_moves`] or allowed with
/// [`AnalysisRequest::allow_moves`].
#[derive(Debug, Clone)]
pub struct RestrictedMoves {
    /// The player the move restriction applies to.
    pub player: Player,

    /// The list of moves.
    pub moves: Vec<Move>,

    /// The search depth within which the restriction applies.
    pub until_depth: u32,
}

impl RestrictedMoves {
    /// Converts this restriction into the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn into_engine_restricted_moves(self, height: u8) -> engine::RestrictedMoves {
        engine::RestrictedMoves {
            player: self.player,
            moves: self.moves.into_iter().map(|m| m.to_gtp(height)).collect(),
            until_depth: self.until_depth,
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "sgf-parse")]
    mod sgf {
        use std::collections::HashSet;

        use crate::{AnalysisRequest, Coord, Move, Player, Rules};

        #[test]
        fn from_sgf() {
            let sgf = "(;;B[pd](;B[dp];W[])(;W[dp];B[pp]))";
            let request =
                AnalysisRequest::from(sgf_parse::go::parse(sgf).unwrap().first().unwrap());
            assert_eq!(request.rules, Rules::japanese());
            assert_eq!(request.komi, None);
            assert_eq!(request.board_x_size, 19);
            assert_eq!(request.board_y_size, 19);
            assert_eq!(request.initial_stones, None);
            assert_eq!(request.initial_player, None);
            assert_eq!(
                request.moves,
                vec![
                    (Player::Black, Move::Move(Coord(15, 3))),
                    (Player::Black, Move::Move(Coord(3, 15))),
                    (Player::White, Move::Pass),
                ]
            );
        }

        #[test]
        fn size() {
            let sgf = "(;SZ[9:13];B[aa];W[im])";
            let request =
                AnalysisRequest::from(sgf_parse::go::parse(sgf).unwrap().first().unwrap());
            assert_eq!(request.rules, Rules::japanese());
            assert_eq!(request.komi, None);
            assert_eq!(request.board_x_size, 9);
            assert_eq!(request.board_y_size, 13);
            assert_eq!(request.initial_stones, None);
            assert_eq!(request.initial_player, None);
            assert_eq!(
                request.moves,
                vec![
                    (Player::Black, Move::Move(Coord(0, 0))),
                    (Player::White, Move::Move(Coord(8, 12))),
                ]
            );
        }

        #[test]
        fn komi() {
            let sgf = "(;KM[7.5];B[pd];W[dp])";
            let request =
                AnalysisRequest::from(sgf_parse::go::parse(sgf).unwrap().first().unwrap());
            assert_eq!(request.rules, Rules::chinese());
            assert_eq!(request.komi, Some(7.5));
            assert_eq!(request.board_x_size, 19);
            assert_eq!(request.board_y_size, 19);
            assert_eq!(request.initial_stones, None);
            assert_eq!(request.initial_player, None);
            assert_eq!(
                request.moves,
                vec![
                    (Player::Black, Move::Move(Coord(15, 3))),
                    (Player::White, Move::Move(Coord(3, 15))),
                ]
            );
        }

        #[test]
        fn rules() {
            let sgf = "(;RU[aga];B[pd];W[dp])";
            let request =
                AnalysisRequest::from(sgf_parse::go::parse(sgf).unwrap().first().unwrap());
            assert_eq!(request.rules, Rules::Named("aga".to_string()));
            assert_eq!(request.komi, None);
            assert_eq!(request.board_x_size, 19);
            assert_eq!(request.board_y_size, 19);
            assert_eq!(request.initial_stones, None);
            assert_eq!(request.initial_player, None);
            assert_eq!(
                request.moves,
                vec![
                    (Player::Black, Move::Move(Coord(15, 3))),
                    (Player::White, Move::Move(Coord(3, 15))),
                ]
            );
        }

        #[test]
        fn initial_stones() {
            let sgf = "(;AB[pd][dp]AW[dd][pp];B[cc];W[qc])";
            let request =
                AnalysisRequest::from(sgf_parse::go::parse(sgf).unwrap().first().unwrap());
            assert_eq!(request.rules, Rules::japanese());
            assert_eq!(request.komi, None);
            assert_eq!(request.board_x_size, 19);
            assert_eq!(request.board_y_size, 19);
            assert_eq!(
                request
                    .initial_stones
                    .map(|s| HashSet::<(Player, Coord)>::from_iter(s)),
                Some(HashSet::from_iter(vec![
                    (Player::Black, Coord(15, 3)),
                    (Player::Black, Coord(3, 15)),
                    (Player::White, Coord(3, 3)),
                    (Player::White, Coord(15, 15)),
                ]))
            );
            assert_eq!(request.initial_player, None);
            assert_eq!(
                request.moves,
                vec![
                    (Player::Black, Move::Move(Coord(2, 2))),
                    (Player::White, Move::Move(Coord(16, 2))),
                ]
            );
        }

        #[test]
        fn initial_player() {
            let sgf = "(;PL[W];B[pd];W[dp])";
            let request =
                AnalysisRequest::from(sgf_parse::go::parse(sgf).unwrap().first().unwrap());
            assert_eq!(request.rules, Rules::japanese());
            assert_eq!(request.komi, None);
            assert_eq!(request.board_x_size, 19);
            assert_eq!(request.board_y_size, 19);
            assert_eq!(request.initial_stones, None);
            assert_eq!(request.initial_player, Some(Player::White));
            assert_eq!(
                request.moves,
                vec![
                    (Player::Black, Move::Move(Coord(15, 3))),
                    (Player::White, Move::Move(Coord(3, 15))),
                ]
            );
        }
    }
}
