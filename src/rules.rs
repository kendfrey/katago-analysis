use serde::Serialize;

/// Rules settings for KataGo.
///
/// See [KataGo's Supported Go Rules](https://lightvector.github.io/KataGo/rules.html) for more details.
///
/// ```
/// # use katago_analysis::*;
/// let japanese_rules = Rules::japanese();
/// let bga_rules = Rules::Named("bga".to_string());
/// let custom_rules = Rules::Explicit {
///     ko: Ko::Positional,
///     scoring: Scoring::Territory,
///     tax: Tax::Seki,
///     suicide: false,
///     has_button: false,
///     white_handicap_bonus: Bonus::Zero,
///     friendly_pass_ok: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum Rules {
    /// A ruleset identified by name.
    Named(String),

    /// A ruleset defined by settings for each rule.
    Explicit {
        /// The ko rule.
        ko: Ko,

        /// The scoring method.
        scoring: Scoring,

        /// The group tax rule.
        tax: Tax,

        /// Whether multi-stone suicide is legal.
        suicide: bool,

        /// Whether the button rule is used.
        has_button: bool,

        /// The bonus points white receives in handicap games.
        white_handicap_bonus: Bonus,

        /// Whether it's allowed to pass before removing all dead stones.
        friendly_pass_ok: bool,
    },
}

macro_rules! rules {
    ($(#[$meta:meta])* $name:ident, $value:expr) => {
        $(#[$meta])*
        pub fn $name() -> Self {
            Rules::Named($value.to_string())
        }
    };
}

impl Rules {
    rules!(
        /// Japanese and Korean rules.
        japanese, "japanese"
    );
    rules!(
        /// Chinese rules as implemented over the board (no superko).
        chinese, "chinese"
    );
    rules!(
        /// Chinese rules as implemented online (positional superko).
        chinese_ogs, "chinese-ogs"
    );
    rules!(
        /// Stone scoring (area scoring with group tax).
        stone_scoring, "stone-scoring"
    );
    rules!(
        /// Territory scoring with group tax.
        ancient_territory, "ancient-territory"
    );
    rules!(
        /// AGA rules using the button.
        aga_button, "aga-button"
    );
    rules!(
        /// AGA, BGA, and French rules.
        aga, "aga"
    );
    rules!(
        /// New Zealand rules.
        new_zealand, "new-zealand"
    );
    rules!(
        /// Tromp-Taylor rules.
        tromp_taylor, "tromp-taylor"
    );
    rules!(
        /// Ing rules.
        ing, "ing"
    );
}

/// Ko rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Ko {
    /// The immediately previous position is forbidden.
    Simple,

    /// Any previous position is forbidden.
    Positional,

    /// Any previous position with the same player to move is forbidden.
    Situational,
}

/// Scoring methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Scoring {
    /// Area scoring as used in Chinese rules.
    Area,

    /// Territory scoring as used in Japanese rules.
    Territory,
}

/// Group tax rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Tax {
    /// All surrounded empty points count.
    None,

    /// Empty points surrounded by a group in seki don't count.
    Seki,

    /// All groups are taxed up to 2 of their surrounded empty points.
    All,
}

/// Bonus points white receives in handicap games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Bonus {
    /// White receives no bonus points.
    #[serde(rename = "0")]
    Zero,

    /// White receives bonus points equal to one less than the number of handicap stones.
    #[serde(rename = "N-1")]
    NMinusOne,

    /// White receives bonus points equal to the number of handicap stones.
    N,
}
