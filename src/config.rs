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
    pub fn insert<K: Into<String>, V: Into<Value>>(&mut self, key: K, value: V) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }

    /// Removes a configuration setting.
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.0.remove(key);
        self
    }

    /// Adds a configuration setting and returns the new configuration.
    pub fn with<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
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
}
