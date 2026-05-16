//! KataGo's analysis protocol, implemented as a low-level stream of unsynchronized messages.
//!
//! See [KataGo Parallel Analysis Engine](https://github.com/lightvector/KataGo/blob/master/docs/Analysis_Engine.md)
//! for official documentation of the analysis engine.
//!
//! You probably want to use the higher-level API in the [crate root](crate) instead of this module. This module is intended
//! for use cases that require lower-level control over the messages sent to and received from the engine.
//!
//! Note: The asynchronous methods in this library must be called from within a Tokio runtime.
//!
//! # Example
//!
//! ```
//! use katago_analysis::{
//!     Player, Result, Rules,
//!     engine::{AnalysisRequest, AnalysisResponse, Engine, LaunchOptions, Request, Response},
//! };
//! use tokio_stream::StreamExt;
//!
//! async fn example(
//!     katago_path: String,
//!     analysis_config_path: String,
//!     model_path: String,
//! ) -> Result<()> {
//!     let options = LaunchOptions::new(katago_path, analysis_config_path, model_path);
//!     let mut engine = Engine::launch(&options)?;
//!
//!     let request = AnalysisRequest::new(
//!         "1".to_string(),
//!         Rules::chinese(),
//!         19,
//!         19,
//!         vec![
//!             (Player::Black, "Q16".to_string()),
//!             (Player::White, "D4".to_string()),
//!         ],
//!     );
//!     engine.stdin.send(&Request::Analyze(request)).await?;
//!     match engine.stdout.try_next().await? {
//!         Some(Response::Analyze(AnalysisResponse { move_infos, .. })) => {
//!             println!(
//!                 "Best move: {} ({:.1}%)",
//!                 move_infos[0].mv,
//!                 move_infos[0].winrate * 100.0
//!             );
//!             println!("{:?}", move_infos[0]);
//!         }
//!         _ => println!("Something went wrong"),
//!     };
//!     Ok(())
//! }
//! ```

use std::{io, process::Stdio};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};
use tokio_stream::{StreamExt, wrappers::LinesStream};

use crate::{Config, Error, Result};

mod request;
pub use request::*;

mod response;
pub use response::*;

/// Command line options for launching KataGo.
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// The path to the KataGo executable.
    pub katago_path: String,

    /// The path to the config file.
    pub config_path: String,

    /// The path to the model file.
    pub model_path: String,

    /// If true, KataGo's stderr output will be produced on the current process's stderr.
    /// Otherwise, it will be made available as [`Engine::stderr`].
    pub inherit_stderr: bool,

    /// The path to the humanSL model file.
    pub human_model_path: Option<String>,

    /// Overrides to pass via `-override-config`.
    pub override_config: Option<Config>,

    /// If true, the engine will stop immediately when stdin is closed, instead of responding to pending requests.
    pub quit_without_waiting: bool,
}

impl LaunchOptions {
    /// Creates a new [`LaunchOptions`].
    pub fn new(katago_path: String, config_path: String, model_path: String) -> Self {
        Self {
            katago_path,
            config_path,
            model_path,
            inherit_stderr: false,
            human_model_path: None,
            override_config: None,
            quit_without_waiting: false,
        }
    }

    /// Causes KataGo's stderr output to be produced on the current process's stderr.
    pub fn with_inherit_stderr(mut self) -> Self {
        self.inherit_stderr = true;
        self
    }

    /// Loads the humanSL model from the given path.
    pub fn with_human_model(mut self, human_model_path: String) -> Self {
        self.human_model_path = Some(human_model_path);
        self
    }

    /// Passes the given options via `-override-config`.
    pub fn with_override_config(mut self, config: Config) -> Self {
        self.override_config = Some(config);
        self
    }

    /// Stop immediately when stdin is closed, instead of responding to pending requests.
    pub fn with_quit_without_waiting(mut self) -> Self {
        self.quit_without_waiting = true;
        self
    }
}

/// An instance of the KataGo analysis engine, launched as a child process.
#[derive(Debug)]
pub struct Engine {
    /// Sends requests to the analysis engine.
    ///
    /// Drop this to close the engine's stdin and request KataGo to exit.
    pub stdin: EngineStdin,

    /// A [`Stream`][futures_core::stream::Stream] of [`Response`]s from the analysis engine.
    pub stdout: EngineStdout,

    /// The analysis engine's stderr output, if available.
    pub stderr: Option<ChildStderr>,

    /// The engine process.
    pub child_process: Child,
}

impl Engine {
    /// Launches the KataGo analysis engine with the given options.
    pub fn launch(config: &LaunchOptions) -> Result<Engine> {
        let mut cmd = Command::new(&config.katago_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(if config.inherit_stderr {
                Stdio::inherit()
            } else {
                Stdio::piped()
            })
            .arg("analysis")
            .arg("-config")
            .arg(&config.config_path)
            .arg("-model")
            .arg(&config.model_path);

        if let Some(human_model_path) = &config.human_model_path {
            cmd.arg("-human-model").arg(human_model_path);
        }

        if let Some(override_config) = &config.override_config {
            cmd.arg("-override-config").arg(
                override_config
                    .to_command_line_arg()
                    .map_err(Error::UnserializableConfig)?,
            );
        }

        if config.quit_without_waiting {
            cmd.arg("-quit-without-waiting");
        }

        let mut child_process = cmd.spawn()?;
        let stdin = child_process.stdin.take().ok_or(Error::StdinUnavailable)?;
        let stdout = child_process
            .stdout
            .take()
            .ok_or(Error::StdoutUnavailable)?;
        let stdout_stream: EngineStdout = LinesStream::new(BufReader::new(stdout).lines())
            .map(|line| Ok(serde_json::from_str::<Response>(&line?)?));

        Ok(Engine {
            stdin: EngineStdin(stdin),
            stdout: stdout_stream,
            stderr: child_process.stderr.take(),
            child_process,
        })
    }
}

/// Sends requests to the analysis engine.
///
/// When dropped, this will close the engine's stdin and request KataGo to exit.
#[derive(Debug)]
pub struct EngineStdin(ChildStdin);

impl EngineStdin {
    /// Sends a [`Request`] to the analysis engine.
    pub async fn send(&mut self, request: &Request) -> Result<()> {
        let json = serde_json::to_string(request)?;
        self.send_raw(&json).await
    }

    /// Sends a raw string to the analysis engine.
    pub async fn send_raw(&mut self, request: &str) -> Result<()> {
        self.0.write_all(request.as_bytes()).await?;
        self.0.write_all(b"\n").await?;
        Ok(())
    }
}

/// A [`Stream`][futures_core::stream::Stream] of [`Response`]s from the analysis engine.
pub type EngineStdout = tokio_stream::adapters::Map<
    LinesStream<BufReader<ChildStdout>>,
    fn(std::result::Result<String, io::Error>) -> Result<Response>,
>;
