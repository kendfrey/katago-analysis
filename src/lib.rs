//! A wrapper around KataGo's analysis protocol.
//!
//! See [KataGo Parallel Analysis Engine](https://github.com/lightvector/KataGo/blob/master/docs/Analysis_Engine.md)
//! for official documentation of the analysis engine.
//!
//! Note: The asynchronous methods in this library must be called from within a Tokio runtime.
//!
//! # Examples
//!
//! After launching an [`Engine`](engine::Engine), the primary entry point for using this library is [`Analyzer`].
//!
//! ```
//! use katago_analysis::{
//!     AnalysisRequest, Analyzer, Coord, Move, Player, Result, Rules,
//!     engine::{Engine, LaunchOptions},
//! };
//!
//! async fn example(
//!     katago_path: String,
//!     analysis_config_path: String,
//!     model_path: String,
//! ) -> Result<()> {
//!     let options = LaunchOptions::new(katago_path, analysis_config_path, model_path);
//!     let mut analyzer: Analyzer = Engine::launch(&options)?.into();
//!
//!     let request = AnalysisRequest::new(
//!         Rules::chinese(),
//!         19,
//!         19,
//!         vec![
//!             (Player::Black, Move::Move(Coord(15, 3))),
//!             (Player::White, Move::Move(Coord(3, 15))),
//!         ],
//!     );
//!
//!     let results = analyzer.analyze_game(request).await?;
//!     for i in 0..results.len() {
//!         println!(
//!             "Move {i}: {:.1}%",
//!             results.get(&i).unwrap().root_info.winrate * 100.0
//!         );
//!     }
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]

use std::{io, sync::Arc};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod engine;

mod analyzer;
pub use analyzer::*;

mod config;
pub use config::*;

mod request;
pub use request::*;

mod result;
pub use result::*;

mod rules;
pub use rules::*;

/// The type of results returned by methods in this library.
pub type Result<T> = std::result::Result<T, Error>;

/// The type of results which may contain warnings returned by the analysis engine.
///
/// See also: [`WarningHandling`]
pub type WarningResult<T, W = WarningsAsErrors> = Result<<W as WarningHandling>::OkType<T>>;

/// Errors that can occur while interacting with the analysis engine.
#[derive(Debug, Clone, Error)]
pub enum Error {
    /// An I/O error occurred while launching, writing to, or reading from the analysis engine.
    #[error("I/O error: {0}")]
    Io(#[from] Arc<io::Error>),

    /// An error occurred while serializing or deserializing a message.
    #[error("serialization error: {0}")]
    Serialization(#[from] Arc<serde_json::Error>),

    /// The engine's stdin was unavailable after launch.
    #[error("stdin unavailable")]
    StdinUnavailable,

    /// The engine's stdout was unavailable after launch.
    #[error("stdout unavailable")]
    StdoutUnavailable,

    /// The specified config setting's value could not be serialized.
    #[error("unserializable config value for key {0}")]
    UnserializableConfig(String),

    /// The analysis engine returned an error response without specifying which request caused it.
    ///
    /// When this error occurs, all pending requests will return this error, even if they might have otherwise
    /// succeeded.
    #[error("invalid request: {error}")]
    KataGoGeneralError {
        /// The error message provided by KataGo.
        error: String,
    },

    /// The analysis engine returned an error response.
    ///
    /// When this error occurs, all positions still being analyzed as part of the associated request will return this
    /// error, even if they might have otherwise succeeded.
    #[error("invalid field {field}: {error}")]
    KataGoFieldError {
        /// The error message provided by KataGo.
        error: String,

        /// The request field that caused the error.
        field: String,
    },

    /// The analysis engine returned a warning response which was converted to an error.
    ///
    /// See also: [`WarningHandling`]
    #[error("unhandled warnings: {0:?}")]
    UnhandledWarnings(Vec<Warning>),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(Arc::new(e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(Arc::new(e))
    }
}

/// A warning returned by the analysis engine.
#[derive(Debug, Clone)]
pub struct Warning {
    /// The warning message provided by KataGo.
    pub warning: String,

    /// The request field that caused the warning.
    pub field: String,
}

/// Specifies how warnings from the analysis engine should be handled.
///
/// # Warning Handling
///
/// [`Analyzer`] can handle analysis engine warnings in several ways. The `Ok` result type of methods that can produce
/// warnings will vary depending on which strategy is chosen.
///
/// - [`Analyzer<WarningsAsErrors>`](WarningsAsErrors) will return the successful result directly, or return [`Error::UnhandledWarnings`]
///   when a warning occurs. This is the default strategy.
/// - [`Analyzer<ReturnWarnings>`](ReturnWarnings) will return a result wrapped in a [`MaybeWarnings`].
/// - [`Analyzer<IgnoreWarnings>`](IgnoreWarnings) will return the successful result directly, and ignore any warnings that occur.
///   This is not recommended unless you expect warnings to occur and don't intend to handle them.
///
/// # Examples
///
/// ### [`WarningsAsErrors`]
///
/// ```
/// # use katago_analysis::*;
/// async fn example(
///     analyzer: &mut Analyzer,
///     request: AnalysisRequest
/// ) -> Result<()> {
///     let result: AnalysisResult = analyzer
///         .analyze(request)
///         .await? // Warnings will cause this to fail with an error
///         .unwrap();
///     println!("{:.1}%", result.root_info.winrate * 100.0);
///     Ok(())
/// }
/// ```
///
/// ### [`ReturnWarnings`]
///
/// ```
/// # use katago_analysis::*;
/// async fn example(
///     analyzer: &mut Analyzer<ReturnWarnings>,
///     request: AnalysisRequest,
/// ) -> Result<()> {
///     let result: AnalysisResult = analyzer
///         .analyze(request)
///         .await?
///         .inspect_warnings(|warnings| {
///             for warning in warnings {
///                println!("{warning:?}");
///             }
///         })
///         .value
///         .unwrap();
///     println!("{:.1}%", result.root_info.winrate * 100.0);
///     Ok(())
/// }
/// ```
///
/// Warnings can also be converted back to errors:
///
/// ```
/// # use katago_analysis::*;
/// async fn example(
///     analyzer: &mut Analyzer<ReturnWarnings>,
///     request: AnalysisRequest,
/// ) -> Result<()> {
///     let result: AnalysisResult = analyzer
///         .analyze(request)
///         .await?
///         .into_result()? // Warnings will cause this to fail with an error
///         .unwrap();
///     println!("{:.1}%", result.root_info.winrate * 100.0);
///     Ok(())
/// }
/// ```
///
/// ### [`IgnoreWarnings`]
///
/// ```
/// # use katago_analysis::*;
/// async fn example(
///     analyzer: &mut Analyzer<IgnoreWarnings>,
///     request: AnalysisRequest
/// ) -> Result<()> {
///     let result: AnalysisResult = analyzer
///         .analyze(request)
///         .await? // Warnings will be ignored
///         .unwrap();
///     println!("{:.1}%", result.root_info.winrate * 100.0);
///     Ok(())
/// }
/// ```
pub trait WarningHandling {
    /// The `Ok` result type.
    type OkType<T>;

    /// Creates a successful result containing the given value and no warnings.
    fn ok<T>(val: T) -> WarningResult<T, Self>;

    /// Updates the successful result, preserving errors and warnings.
    fn set_result<T>(result: &mut WarningResult<T, Self>, value: T);

    /// Adds a new warning to the result.
    fn add_warning<T>(result: &mut WarningResult<T, Self>, warning: Warning);

    /// Applies a function to the successful values of two results, merging warnings and errors.
    fn merge<T, U, V>(
        a: WarningResult<T, Self>,
        b: WarningResult<U, Self>,
        f: impl FnOnce(T, U) -> V,
    ) -> WarningResult<V, Self>;
}

/// Warnings will return [`Error::UnhandledWarnings`].
///
/// See also: [`WarningHandling`]
#[derive(Debug, Default, Clone)]
pub struct WarningsAsErrors;

impl WarningHandling for WarningsAsErrors {
    type OkType<T> = T;

    fn ok<T>(val: T) -> WarningResult<T, Self> {
        Ok(val)
    }

    fn set_result<T>(result: &mut WarningResult<T, Self>, value: T) {
        if let Ok(r) = result {
            *r = value;
        }
    }

    fn add_warning<T>(result: &mut WarningResult<T, Self>, warning: Warning) {
        match result {
            Ok(_) => *result = Err(Error::UnhandledWarnings(vec![warning])),
            Err(Error::UnhandledWarnings(warnings)) => warnings.push(warning),
            _ => {}
        }
    }

    fn merge<T, U, V>(
        a: WarningResult<T, Self>,
        b: WarningResult<U, Self>,
        f: impl FnOnce(T, U) -> V,
    ) -> WarningResult<V, Self> {
        Ok(f(a?, b?))
    }
}

/// Warnings will be returned in a [`MaybeWarnings`].
///
/// See also: [`WarningHandling`]
#[derive(Debug, Default, Clone)]
pub struct ReturnWarnings;

impl WarningHandling for ReturnWarnings {
    type OkType<T> = MaybeWarnings<T>;

    fn ok<T>(val: T) -> WarningResult<T, Self> {
        Ok(MaybeWarnings {
            value: val,
            warnings: None,
        })
    }

    fn set_result<T>(result: &mut WarningResult<T, Self>, value: T) {
        if let Ok(r) = result {
            r.value = value;
        }
    }

    fn add_warning<T>(result: &mut WarningResult<T, Self>, warning: Warning) {
        if let Ok(r) = result {
            match r.warnings.as_mut() {
                Some(warnings) => warnings.push(warning),
                None => r.warnings = Some(vec![warning]),
            }
        }
    }

    fn merge<T, U, V>(
        a: WarningResult<T, Self>,
        b: WarningResult<U, Self>,
        f: impl FnOnce(T, U) -> V,
    ) -> WarningResult<V, Self> {
        let MaybeWarnings {
            value: a,
            warnings: a_warnings,
        } = a?;
        let MaybeWarnings {
            value: b,
            warnings: b_warnings,
        } = b?;
        Ok(MaybeWarnings {
            value: f(a, b),
            warnings: a_warnings.or(b_warnings),
        })
    }
}

/// A result that may contain warnings.
#[derive(Debug, Default, Clone)]
pub struct MaybeWarnings<T> {
    /// The successful result value.
    pub value: T,

    /// The list of warnings that occurred, if any.
    pub warnings: Option<Vec<Warning>>,
}

impl<T> MaybeWarnings<T> {
    /// Extracts the successful result value, returning [`Error::UnhandledWarnings`] if any warnings occurred.
    pub fn into_result(self) -> Result<T> {
        match self.warnings {
            Some(warnings) => Err(Error::UnhandledWarnings(warnings)),
            None => Ok(self.value),
        }
    }

    /// Calls a function with a reference to the warnings, if any.
    pub fn inspect_warnings<F: FnOnce(&Vec<Warning>)>(self, f: F) -> Self {
        if let Some(warnings) = &self.warnings {
            f(warnings);
        }
        self
    }
}

/// Warnings will be dropped.
///
/// See also: [`WarningHandling`]
#[derive(Debug, Default, Clone)]
pub struct IgnoreWarnings;

impl WarningHandling for IgnoreWarnings {
    type OkType<T> = T;

    fn ok<T>(val: T) -> WarningResult<T, Self> {
        Ok(val)
    }

    fn set_result<T>(result: &mut WarningResult<T, Self>, value: T) {
        if let Ok(r) = result {
            *r = value;
        }
    }

    fn add_warning<T>(_result: &mut WarningResult<T, Self>, _warning: Warning) {}

    fn merge<T, U, V>(
        a: WarningResult<T, Self>,
        b: WarningResult<U, Self>,
        f: impl FnOnce(T, U) -> V,
    ) -> WarningResult<V, Self> {
        Ok(f(a?, b?))
    }
}

/// Player colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Player {
    /// The black player.
    #[serde(rename = "B")]
    Black,
    /// The white player.
    #[serde(rename = "W")]
    White,
}

#[cfg(feature = "sgf-parse")]
impl From<sgf_parse::Color> for Player {
    fn from(value: sgf_parse::Color) -> Self {
        match value {
            sgf_parse::Color::Black => Player::Black,
            sgf_parse::Color::White => Player::White,
        }
    }
}

/// A board location in (x, y) format, where (0, 0) is the top-left corner of the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord(pub u8, pub u8);

impl Coord {
    /// Converts a coordinate from GTP format.
    pub fn from_gtp(s: &str, height: u8) -> Option<Self> {
        let (x_part, y_part) = s
            .chars()
            .partition::<String, _>(|c| c.is_ascii_alphabetic());

        let mut x = 0;
        for c in x_part.chars().map(|c| c.to_ascii_uppercase()) {
            x *= 25;
            x += (c as u8) - b'A' + 1;
            if c == 'I' {
                return None;
            } else if c > 'I' {
                x -= 1;
            }
        }
        x = x.checked_sub(1)?;
        let y = height.checked_sub(y_part.parse::<u8>().ok()?)?;
        Some(Self(x, y))
    }

    /// Converts a coordinate to GTP format.
    pub fn to_gtp(self, height: u8) -> String {
        let Self(mut x, y) = self;
        const LETTERS: &[u8; 25] = b"ABCDEFGHJKLMNOPQRSTUVWXYZ";
        let mut gtp = String::with_capacity(4);
        if x >= 25 {
            gtp.push(LETTERS[(x / 25) as usize - 1] as char);
            x %= 25;
        }
        gtp.push(LETTERS[x as usize] as char);
        gtp.push_str(&(height - y).to_string());
        gtp
    }
}

#[cfg(feature = "sgf-parse")]
impl From<sgf_parse::go::Point> for Coord {
    fn from(c: sgf_parse::go::Point) -> Self {
        Self(c.x, c.y)
    }
}

/// A move in a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Move {
    /// A move placing a stone at the specified coordinate.
    Move(Coord),

    /// A pass.
    Pass,
}

impl Move {
    /// Converts a move from GTP format.
    pub fn from_gtp(s: &str, height: u8) -> Option<Self> {
        if s.to_ascii_lowercase() == "pass" {
            Some(Self::Pass)
        } else {
            Coord::from_gtp(s, height).map(Self::Move)
        }
    }

    /// Converts a move to GTP format.
    pub fn to_gtp(self, height: u8) -> String {
        match self {
            Move::Move(coord) => coord.to_gtp(height),
            Move::Pass => "pass".to_string(),
        }
    }
}

#[cfg(feature = "sgf-parse")]
impl From<sgf_parse::go::Move> for Move {
    fn from(m: sgf_parse::go::Move) -> Self {
        match m {
            sgf_parse::go::Move::Move(p) => Move::Move(p.into()),
            sgf_parse::go::Move::Pass => Move::Pass,
        }
    }
}

/// KataGo's version information.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// A string indicating the most recent KataGo release version that this version is a descendant of,
    /// such as `"1.6.1"`.
    pub version: String,

    /// The precise git hash this KataGo version was compiled from, or the string `"<omitted>"` if KataGo was
    /// compiled separately from its repo or without Git support.
    pub git_hash: String,
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
    pub max_batch_size: u32,

    /// Whether it uses a humanSL profile.
    #[serde(rename = "usesHumanSLProfile")]
    pub uses_humansl_profile: bool,

    /// The model version.
    pub version: u32,

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

#[cfg(test)]
mod tests {
    use crate::Coord;

    #[test]
    fn coord_from_gtp() {
        assert_eq!(Coord::from_gtp("A1", 19), Some(Coord(0, 18)));
        assert_eq!(Coord::from_gtp("T19", 19), Some(Coord(18, 0)));
        assert_eq!(Coord::from_gtp("I9", 19), None);
        assert_eq!(Coord::from_gtp("1", 19), None);
        assert_eq!(Coord::from_gtp("A", 19), None);
        assert_eq!(Coord::from_gtp("A20", 19), None);
        assert_eq!(Coord::from_gtp("Z1", 255), Some(Coord(24, 254)));
        assert_eq!(Coord::from_gtp("AA1", 255), Some(Coord(25, 254)));
        assert_eq!(Coord::from_gtp("BB1", 255), Some(Coord(51, 254)));
        assert_eq!(Coord::from_gtp("JJ1", 255), Some(Coord(233, 254)));
    }

    #[test]
    fn coord_to_gtp() {
        assert_eq!(Coord(0, 18).to_gtp(19), "A1");
        assert_eq!(Coord(18, 0).to_gtp(19), "T19");
        assert_eq!(Coord(24, 254).to_gtp(255), "Z1");
        assert_eq!(Coord(25, 254).to_gtp(255), "AA1");
        assert_eq!(Coord(51, 254).to_gtp(255), "BB1");
        assert_eq!(Coord(233, 254).to_gtp(255), "JJ1");
    }
}
