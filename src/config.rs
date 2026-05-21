use serde::Serialize;
use serde_json::{Map, Value};

/// KataGo configuration as used in [.cfg files](crate::engine::LaunchOptions::config_path), the
/// [`-override-config`](crate::engine::LaunchOptions::with_override_config) command line argument, and the
/// [`overrideSettings`](crate::engine::AnalysisRequest::with_override_settings) property of analysis requests.
///
/// ```
/// # use katago_analysis::*;
/// let override_config = Config::new()
///     .with("maxVisits", 1000)
///     .with("reportAnalysisWinratesAs", "BLACK");
/// ```
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct Config(Map<String, Value>);

impl Config {
    /// Creates a new configuration with no options set.
    pub fn new() -> Self {
        Default::default()
    }

    /// Adds a configuration setting.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }

    /// Removes a configuration setting.
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.0.remove(key);
        self
    }

    /// Adds a configuration setting and returns the new configuration.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.insert(key, value);
        self
    }

    /// Serializes the configuration to the format expected by the `-override-config` command line argument.
    ///
    /// If a setting's value cannot be serialized, returns an [`Err`] containing the key of the invalid setting.
    pub fn to_command_line_arg(&self) -> Result<String, String> {
        Ok(self
            .0
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => return Err(k.clone()),
                };
                Ok(format!("{}={}", k, value))
            })
            .collect::<Result<Vec<String>, String>>()?
            .join(","))
    }

    /// Sets maximum length of the principal variation.
    pub fn with_analysis_pv_len(self, analysis_pv_len: usize) -> Self {
        self.with("analysisPVLen", analysis_pv_len)
    }

    /// Sets whether to use anti-mirror play.
    pub fn with_anti_mirror(self, anti_mirror: bool) -> Self {
        self.with("antiMirror", anti_mirror)
    }

    /// Sets the humanSL profile.
    pub fn with_human_sl_profile(self, human_sl_profile: impl Into<String>) -> Self {
        self.with("humanSLProfile", human_sl_profile.into())
    }

    /// Sets whether to ignore the moves that led up to the position being analyzed.
    pub fn with_ignore_pre_root_history(self, ignore_pre_root_history: bool) -> Self {
        self.with("ignorePreRootHistory", ignore_pre_root_history)
    }

    /// Sets the maximum time per position.
    pub fn with_max_time(self, max_time: f64) -> Self {
        self.with("maxTime", max_time)
    }

    /// Sets the maximum number of visits per position.
    pub fn with_max_visits(self, max_visits: u32) -> Self {
        self.with("maxVisits", max_visits)
    }

    /// Sets the number of analysis threads.
    pub fn with_num_analysis_threads(self, num_analysis_threads: u32) -> Self {
        self.with("numAnalysisThreads", num_analysis_threads)
    }

    /// Sets the number of search threads per analysis thread.
    pub fn with_num_search_threads_per_analysis_thread(self, num_search_threads: u32) -> Self {
        self.with("numSearchThreadsPerAnalysisThread", num_search_threads)
    }

    /// Sets the playout doubling advantage.
    pub fn with_playout_doubling_advantage(self, playout_doubling_advantage: f64) -> Self {
        self.with("playoutDoublingAdvantage", playout_doubling_advantage)
    }

    /// Reports winrates relative to the given side.
    pub fn with_report_analysis_winrates_as(self, report_analysis_winrates_as: Side) -> Self {
        self.with("reportAnalysisWinratesAs", report_analysis_winrates_as)
    }

    /// Sets the number of board symmetries to sample at the root of the search.
    pub fn with_root_num_symmetries_to_sample(self, root_num_symmetries_to_sample: u8) -> Self {
        self.with("rootNumSymmetriesToSample", root_num_symmetries_to_sample)
    }

    /// Sets the wide root noise.
    pub fn with_wide_root_noise(self, wide_root_noise: f64) -> Self {
        self.with("wideRootNoise", wide_root_noise)
    }
}

/// A side which values can be calculated relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The black player.
    Black,

    /// The white player.
    White,

    /// The player to move.
    SideToMove,
}

impl From<Side> for Value {
    fn from(side: Side) -> Self {
        Value::String(
            match side {
                Side::Black => "BLACK",
                Side::White => "WHITE",
                Side::SideToMove => "SIDETOMOVE",
            }
            .to_string(),
        )
    }
}
