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

    /// Config overrides for this request.
    pub override_settings: Option<Config>,

    /// Report partial analysis results every this many seconds.
    pub report_during_search_every: Option<f64>,
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
            override_settings: None,
            report_during_search_every: None,
        }
    }

    /// Converts this request into the lower-level equivalent used by the [`engine`] module.
    ///
    /// You probably don't need to use this unless you're directly using the lower-level API in the [`engine`] module.
    pub fn into_engine_request(
        self,
        id: String,
        analyze_turns: Vec<usize>,
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
            override_settings: self.override_settings,
            report_during_search_every: self.report_during_search_every,
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
