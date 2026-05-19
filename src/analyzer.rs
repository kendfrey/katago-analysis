use std::{
    collections::HashMap,
    ops::{ControlFlow, Deref},
    sync::Arc,
};

use tokio::{
    process::{Child, ChildStderr},
    sync::{Notify, RwLock, RwLockReadGuard},
};
use tokio_stream::StreamExt;

use crate::{
    engine::{Engine, EngineStdin, EngineStdout},
    *,
};

/// An instance of the KataGo analysis engine, launched as a child process.
///
/// Drop this to close the engine's stdin and request KataGo to exit.
/// Responses will continue to be processed until the engine actually exits.
pub struct Analyzer<W: WarningHandling = WarningsAsErrors> {
    stdin: EngineStdin,

    /// The analysis engine's stderr output, if available.
    pub stderr: Option<ChildStderr>,

    /// The engine process.
    pub child_process: Child,

    next_id: u32,
    pending_requests: PendingRequests<W>,
}

impl<W: WarningHandling> Analyzer<W> {
    /// Analyzes the final position in the game and returns a single result.
    pub async fn analyze(
        &mut self,
        request: AnalysisRequest,
    ) -> WarningResult<Option<AnalysisResult>, W> {
        self.start_analyze(request).await?.finish().await
    }

    /// Analyzes a specific position in the game and returns a single result.
    pub async fn analyze_position(
        &mut self,
        request: AnalysisRequest,
        position: usize,
    ) -> WarningResult<Option<AnalysisResult>, W> {
        self.start_analyze_position(request, position)
            .await?
            .finish()
            .await
    }

    /// Analyzes all moves in the game and returns a collection of results, one for each position.
    pub async fn analyze_game(
        &mut self,
        request: AnalysisRequest,
    ) -> WarningResult<HashMap<usize, AnalysisResult>, W> {
        self.start_analyze_game(request).await?.finish().await
    }

    /// Analyzes the specified positions in the game and returns a collection of results, one for each position.
    pub async fn analyze_positions(
        &mut self,
        request: AnalysisRequest,
        analyze_turns: Vec<usize>,
    ) -> WarningResult<HashMap<usize, AnalysisResult>, W> {
        self.start_analyze_positions(request, analyze_turns)
            .await?
            .finish()
            .await
    }

    /// Starts analyzing the final position in the game and returns a progress object which can be polled for updates.
    pub async fn start_analyze(&mut self, request: AnalysisRequest) -> Result<AnalysisProgress<W>> {
        let position = request.moves.len();
        self.start_analyze_position(request, position).await
    }

    /// Starts analyzing a specific position in the game and returns a progress object which can be polled for updates.
    pub async fn start_analyze_position(
        &mut self,
        request: AnalysisRequest,
        position: usize,
    ) -> Result<AnalysisProgress<W>> {
        Ok(self
            .start_analyze_positions(request, vec![position])
            .await?
            .into_positions()
            .remove(&position)
            .expect("position analysis should be available"))
    }

    /// Starts analyzing all moves in the game and returns a collection of progress objects.
    pub async fn start_analyze_game(
        &mut self,
        request: AnalysisRequest,
    ) -> Result<GameAnalysisProgress<W>> {
        let positions = (0..=request.moves.len()).collect();
        self.start_analyze_positions(request, positions).await
    }

    /// Starts analyzing the specified positions in the game and returns a collection of progress objects.
    pub async fn start_analyze_positions(
        &mut self,
        request: AnalysisRequest,
        analyze_turns: Vec<usize>,
    ) -> Result<GameAnalysisProgress<W>> {
        self.start_analyze_positions_impl(request, analyze_turns, None)
            .await
    }

    /// Analyzes all moves in the game with the given priorities and returns a collection of results, one for each
    /// position.
    ///
    /// `priorities` must have length equal to one more than the number of moves in the game.
    pub async fn analyze_game_prioritized(
        &mut self,
        request: AnalysisRequest,
        priorities: Vec<i32>,
    ) -> WarningResult<HashMap<usize, AnalysisResult>, W> {
        self.start_analyze_game_prioritized(request, priorities)
            .await?
            .finish()
            .await
    }

    /// Analyzes the specified positions in the game with the given priorities and returns a collection of results,
    /// one for each position.
    ///
    /// `priorities` must have the same length as `analyze_turns`.
    pub async fn analyze_positions_prioritized(
        &mut self,
        request: AnalysisRequest,
        analyze_turns: Vec<usize>,
        priorities: Vec<i32>,
    ) -> WarningResult<HashMap<usize, AnalysisResult>, W> {
        self.start_analyze_positions_prioritized(request, analyze_turns, priorities)
            .await?
            .finish()
            .await
    }

    /// Starts analyzing all moves in the game with the given priorities and returns a collection of progress objects.
    ///
    /// `priorities` must have length equal to one more than the number of moves in the game.
    pub async fn start_analyze_game_prioritized(
        &mut self,
        request: AnalysisRequest,
        priorities: Vec<i32>,
    ) -> Result<GameAnalysisProgress<W>> {
        let positions = (0..=request.moves.len()).collect();
        self.start_analyze_positions_prioritized(request, positions, priorities)
            .await
    }

    /// Starts analyzing the specified positions in the game with the given priorities and returns a collection of
    /// progress objects.
    ///
    /// `priorities` must have the same length as `analyze_turns`.
    pub async fn start_analyze_positions_prioritized(
        &mut self,
        request: AnalysisRequest,
        analyze_turns: Vec<usize>,
        priorities: Vec<i32>,
    ) -> Result<GameAnalysisProgress<W>> {
        self.start_analyze_positions_impl(request, analyze_turns, Some(priorities))
            .await
    }

    async fn start_analyze_positions_impl(
        &mut self,
        request: AnalysisRequest,
        analyze_turns: Vec<usize>,
        priorities: Option<Vec<i32>>,
    ) -> Result<GameAnalysisProgress<W>> {
        let id = self.generate_id();
        let mut senders = HashMap::new();
        let mut positions = HashMap::new();
        for position in &analyze_turns {
            let (sender, receiver) = channel(W::ok(None));
            senders.insert(*position, sender);
            positions.insert(
                *position,
                AnalysisProgress::<W> {
                    receiver,
                    id: id.clone(),
                    turn_number: *position,
                },
            );
        }

        let pending_request = PendingRequest::<W> {
            positions: senders,
            width: request.board_x_size,
            height: request.board_y_size,
        };

        let mut requests = self.pending_requests.requests.write().await;
        self.stdin
            .send(&engine::Request::Analyze(request.into_engine_request(
                id.clone(),
                analyze_turns,
                priorities,
            )))
            .await?;
        requests.insert(id.clone(), pending_request);
        Ok(GameAnalysisProgress::<W> { id, positions })
    }

    /// Requests KataGo's version information.
    pub async fn query_version(&mut self) -> WarningResult<VersionInfo, W> {
        let id = self.generate_id();
        let (sender, receiver) = channel(W::ok(VersionInfo {
            version: String::new(),
            git_hash: String::new(),
        }));

        let mut requests = self.pending_requests.query_version_requests.write().await;
        self.stdin
            .send(&engine::Request::QueryVersion { id: id.clone() })
            .await?;
        requests.insert(id, sender);
        drop(requests);

        receiver.finish().await
    }

    /// Clears the neural network cache.
    pub async fn clear_cache(&mut self) -> WarningResult<(), W> {
        let id = self.generate_id();
        let (sender, receiver) = channel(W::ok(()));

        let mut requests = self.pending_requests.clear_cache_requests.write().await;
        self.stdin
            .send(&engine::Request::ClearCache { id: id.clone() })
            .await?;
        requests.insert(id, sender);
        drop(requests);

        receiver.finish().await
    }

    /// Terminates the analysis for a single position.
    ///
    /// `progress` may still be used to wait for the final result.
    pub async fn terminate(&mut self, progress: &AnalysisProgress) -> WarningResult<(), W> {
        self.terminate_impl(progress.id.clone(), Some(vec![progress.turn_number]))
            .await
    }

    /// Terminates the analysis for all positions in a game.
    ///
    /// `progress` may still be used to wait for the final results.
    pub async fn terminate_game(
        &mut self,
        progress: &GameAnalysisProgress,
    ) -> WarningResult<(), W> {
        self.terminate_impl(progress.id.clone(), None).await
    }

    /// Terminates the analysis for the specified positions in a game.
    ///
    /// `progress` may still be used to wait for the final results.
    pub async fn terminate_positions(
        &mut self,
        progress: &GameAnalysisProgress,
        turn_numbers: Vec<usize>,
    ) -> WarningResult<(), W> {
        self.terminate_impl(progress.id.clone(), Some(turn_numbers))
            .await
    }

    async fn terminate_impl(
        &mut self,
        terminate_id: String,
        turn_numbers: Option<Vec<usize>>,
    ) -> WarningResult<(), W> {
        let id = self.generate_id();
        let (sender, receiver) = channel(W::ok(()));

        let mut requests = self.pending_requests.terminate_requests.write().await;
        self.stdin
            .send(&engine::Request::Terminate {
                id: id.clone(),
                terminate_id,
                turn_numbers,
            })
            .await?;
        requests.insert(id, sender);
        drop(requests);

        receiver.finish().await
    }

    /// Terminates all pending analysis requests.
    pub async fn terminate_all(&mut self) -> WarningResult<(), W> {
        self.terminate_all_impl(None).await
    }

    /// Terminates all pending analysis requests for the specified positions.
    pub async fn terminate_all_positions(
        &mut self,
        turn_numbers: Vec<usize>,
    ) -> WarningResult<(), W> {
        self.terminate_all_impl(Some(turn_numbers)).await
    }

    async fn terminate_all_impl(
        &mut self,
        turn_numbers: Option<Vec<usize>>,
    ) -> WarningResult<(), W> {
        let id = self.generate_id();
        let (sender, receiver) = channel(W::ok(()));

        let mut requests = self.pending_requests.terminate_all_requests.write().await;
        self.stdin
            .send(&engine::Request::TerminateAll {
                id: id.clone(),
                turn_numbers,
            })
            .await?;
        requests.insert(id, sender);
        drop(requests);

        receiver.finish().await
    }

    /// Requests information about the available neural network models.
    pub async fn query_models(&mut self) -> WarningResult<Vec<Model>, W> {
        let id = self.generate_id();
        let (sender, receiver) = channel(W::ok(vec![]));

        let mut requests = self.pending_requests.query_models_requests.write().await;
        self.stdin
            .send(&engine::Request::QueryModels { id: id.clone() })
            .await?;
        requests.insert(id, sender);
        drop(requests);

        receiver.finish().await
    }

    fn generate_id(&mut self) -> String {
        let id = self.next_id.to_string();
        self.next_id += 1;
        id
    }
}

impl<W: WarningHandling + Default + Clone + 'static> From<Engine> for Analyzer<W>
where
    W::OkType<Option<AnalysisResult>>: Send + Sync,
    W::OkType<VersionInfo>: Send + Sync,
    W::OkType<()>: Send + Sync,
    W::OkType<Vec<Model>>: Send + Sync,
{
    fn from(engine: Engine) -> Self {
        let client = Self {
            stdin: engine.stdin,
            stderr: engine.stderr,
            child_process: engine.child_process,
            next_id: 1,
            pending_requests: PendingRequests::<W>::default(),
        };

        tokio::spawn(handle_responses(
            engine.stdout,
            client.pending_requests.clone(),
        ));

        client
    }
}

async fn handle_responses<W: WarningHandling>(
    mut stdout: EngineStdout,
    mut pending: PendingRequests<W>,
) {
    while let Some(response) = stdout.next().await {
        let response = match response {
            Ok(response) => response,
            Err(e) => {
                pending.poison_all(e).await;
                continue;
            }
        };
        match response {
            engine::Response::Analyze(response) => {
                let id = response.id.clone();
                let turn_number = response.turn_number;
                let is_during_search = response.is_during_search;
                let mut requests = pending.requests.write().await;
                if let Some(request) = requests.get_mut(&id) {
                    if let Some(sender) = request.positions.get(&turn_number) {
                        let result = Some(AnalysisResult::from_engine_response(
                            response,
                            request.width,
                            request.height,
                        ));
                        sender.send_modify(|r| W::set_result(r, result)).await;

                        if !is_during_search {
                            request.positions.remove(&turn_number);
                        }
                    }
                    if request.positions.is_empty() {
                        requests.remove(&id);
                    }
                }
            }
            engine::Response::NoResults { id, turn_number } => {
                let mut requests = pending.requests.write().await;
                if let Some(request) = requests.get_mut(&id) {
                    request.positions.remove(&turn_number);
                    if request.positions.is_empty() {
                        requests.remove(&id);
                    }
                }
            }
            engine::Response::QueryVersion {
                id,
                version,
                git_hash,
            } => {
                let mut requests = pending.query_version_requests.write().await;
                if let Some(sender) = requests.remove(&id) {
                    sender
                        .send_modify(|r| W::set_result(r, VersionInfo { version, git_hash }))
                        .await;
                }
            }
            engine::Response::ClearCache { id } => {
                let mut requests = pending.clear_cache_requests.write().await;
                if let Some(sender) = requests.remove(&id) {
                    sender.send_modify(|r| W::set_result(r, ())).await;
                }
            }
            engine::Response::Terminate { id, .. } => {
                let mut requests = pending.terminate_requests.write().await;
                if let Some(sender) = requests.remove(&id) {
                    sender.send_modify(|r| W::set_result(r, ())).await;
                }
            }
            engine::Response::TerminateAll { id, .. } => {
                let mut requests = pending.terminate_all_requests.write().await;
                if let Some(sender) = requests.remove(&id) {
                    sender.send_modify(|r| W::set_result(r, ())).await;
                }
            }
            engine::Response::QueryModels { id, models } => {
                let mut requests = pending.query_models_requests.write().await;
                if let Some(sender) = requests.remove(&id) {
                    sender.send_modify(|r| W::set_result(r, models)).await;
                }
            }
            engine::Response::GeneralError { error } => {
                pending
                    .poison_all(Error::KataGoGeneralError { error })
                    .await;
            }
            engine::Response::FieldError { id, error, field } => {
                pending
                    .poison(&id, Error::KataGoFieldError { error, field })
                    .await;
            }
            engine::Response::FieldWarning { id, warning, field } => {
                pending.add_warning(&id, Warning { warning, field }).await;
            }
        };
    }
}

impl<W: WarningHandling> std::fmt::Debug for Analyzer<W>
where
    W::OkType<Option<AnalysisResult>>: std::fmt::Debug,
    W::OkType<VersionInfo>: std::fmt::Debug,
    W::OkType<()>: std::fmt::Debug,
    W::OkType<Vec<Model>>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Analyzer")
            .field("stdin", &self.stdin)
            .field("stderr", &self.stderr)
            .field("child_process", &self.child_process)
            .field("next_id", &self.next_id)
            .field("pending_requests", &self.pending_requests)
            .finish()
    }
}

#[derive(Default, Clone)]
struct PendingRequests<W: WarningHandling = WarningsAsErrors> {
    requests: Arc<RwLock<HashMap<String, PendingRequest<W>>>>,
    query_version_requests: Arc<RwLock<HashMap<String, Sender<WarningResult<VersionInfo, W>>>>>,
    clear_cache_requests: Arc<RwLock<HashMap<String, Sender<WarningResult<(), W>>>>>,
    terminate_requests: Arc<RwLock<HashMap<String, Sender<WarningResult<(), W>>>>>,
    terminate_all_requests: Arc<RwLock<HashMap<String, Sender<WarningResult<(), W>>>>>,
    query_models_requests: Arc<RwLock<HashMap<String, Sender<WarningResult<Vec<Model>, W>>>>>,
}

impl<W: WarningHandling> PendingRequests<W> {
    async fn poison_all(&mut self, error: Error) {
        for (_, request) in self.requests.write().await.drain() {
            for sender in request.positions.values() {
                sender.send_err(error.clone()).await;
            }
        }

        for (_, sender) in self.query_version_requests.write().await.drain() {
            sender.send_err(error.clone()).await;
        }

        for (_, sender) in self.clear_cache_requests.write().await.drain() {
            sender.send_err(error.clone()).await;
        }

        for (_, sender) in self.terminate_requests.write().await.drain() {
            sender.send_err(error.clone()).await;
        }

        for (_, sender) in self.terminate_all_requests.write().await.drain() {
            sender.send_err(error.clone()).await;
        }

        for (_, sender) in self.query_models_requests.write().await.drain() {
            sender.send_err(error.clone()).await;
        }
    }

    async fn poison(&mut self, id: &str, error: Error) {
        if let Some(request) = self.requests.write().await.remove(id) {
            for sender in request.positions.values() {
                sender.send_err(error.clone()).await;
            }
        }

        if let Some(sender) = self.query_version_requests.write().await.remove(id) {
            sender.send_err(error.clone()).await;
        }

        if let Some(sender) = self.clear_cache_requests.write().await.remove(id) {
            sender.send_err(error.clone()).await;
        }

        if let Some(sender) = self.terminate_requests.write().await.remove(id) {
            sender.send_err(error.clone()).await;
        }

        if let Some(sender) = self.terminate_all_requests.write().await.remove(id) {
            sender.send_err(error.clone()).await;
        }

        if let Some(sender) = self.query_models_requests.write().await.remove(id) {
            sender.send_err(error.clone()).await;
        }
    }

    async fn add_warning(&mut self, id: &str, warning: Warning) {
        if let Some(request) = self.requests.write().await.get(id) {
            for sender in request.positions.values() {
                sender
                    .send_modify(|r| W::add_warning(r, warning.clone()))
                    .await;
            }
        }

        if let Some(sender) = self.query_version_requests.write().await.get(id) {
            sender
                .send_modify(|r| W::add_warning(r, warning.clone()))
                .await;
        }

        if let Some(sender) = self.clear_cache_requests.write().await.get(id) {
            sender
                .send_modify(|r| W::add_warning(r, warning.clone()))
                .await;
        }

        if let Some(sender) = self.terminate_requests.write().await.get(id) {
            sender
                .send_modify(|r| W::add_warning(r, warning.clone()))
                .await;
        }

        if let Some(sender) = self.terminate_all_requests.write().await.get(id) {
            sender
                .send_modify(|r| W::add_warning(r, warning.clone()))
                .await;
        }

        if let Some(sender) = self.query_models_requests.write().await.get(id) {
            sender
                .send_modify(|r| W::add_warning(r, warning.clone()))
                .await;
        }
    }
}

impl<W: WarningHandling> std::fmt::Debug for PendingRequests<W>
where
    W::OkType<Option<AnalysisResult>>: std::fmt::Debug,
    W::OkType<VersionInfo>: std::fmt::Debug,
    W::OkType<()>: std::fmt::Debug,
    W::OkType<Vec<Model>>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequests")
            .field("requests", &self.requests)
            .field("query_version_requests", &self.query_version_requests)
            .field("clear_cache_requests", &self.clear_cache_requests)
            .field("terminate_requests", &self.terminate_requests)
            .field("terminate_all_requests", &self.terminate_all_requests)
            .field("query_models_requests", &self.query_models_requests)
            .finish()
    }
}

struct PendingRequest<W: WarningHandling = WarningsAsErrors> {
    positions: HashMap<usize, Sender<WarningResult<Option<AnalysisResult>, W>>>,
    width: u8,
    height: u8,
}

impl<W: WarningHandling> std::fmt::Debug for PendingRequest<W>
where
    W::OkType<Option<AnalysisResult>>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequest")
            .field("positions", &self.positions)
            .field("height", &self.height)
            .finish()
    }
}

#[derive(Debug)]
struct NotifyOnDrop(Arc<Notify>);

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

impl Deref for NotifyOnDrop {
    type Target = Arc<Notify>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The sender half of a single-producer single-consumer watch channel.
///
/// When dropped, is guaranteed to notify the receiver after the last value is sent.
#[derive(Debug)]
struct Sender<T> {
    value: Arc<RwLock<T>>,
    notify: NotifyOnDrop,
}

impl<T> Sender<T> {
    async fn send_modify(&self, f: impl FnOnce(&mut T)) {
        f(&mut *self.value.write().await);
        self.notify.notify_one();
    }
}

impl<T, E> Sender<std::result::Result<T, E>> {
    async fn send_err(&self, value: E) {
        self.send_modify(|r| *r = Err(value)).await;
    }
}

/// The receiver half of a single-producer single-consumer watch channel.
#[derive(Debug)]
struct Receiver<T> {
    value: Arc<RwLock<T>>,
    notify: Arc<Notify>,
}

impl<T> Receiver<T> {
    async fn finish(mut self) -> T {
        loop {
            match self.poll().await {
                ControlFlow::Break(value) => return value,
                ControlFlow::Continue(s) => self = s,
            };
        }
    }

    async fn poll(self) -> ControlFlow<T, Self> {
        self.notify.notified().await;
        match Arc::try_unwrap(self.value) {
            Ok(value) => ControlFlow::Break(value.into_inner()),
            Err(arc) => ControlFlow::Continue(Self { value: arc, ..self }),
        }
    }

    async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read().await
    }
}

/// Creates a single-producer single-consumer watch channel with the given initial value.
fn channel<T>(value: T) -> (Sender<T>, Receiver<T>) {
    let receiver = Receiver {
        value: Arc::new(RwLock::new(value)),
        notify: Arc::new(Notify::new()),
    };
    let sender = Sender {
        value: receiver.value.clone(),
        notify: NotifyOnDrop(receiver.notify.clone()),
    };
    (sender, receiver)
}

/// A collection of in-progress analysis operations for multiple positions in a single game.
pub struct GameAnalysisProgress<W: WarningHandling = WarningsAsErrors> {
    id: String,
    positions: HashMap<usize, AnalysisProgress<W>>,
}

impl<W: WarningHandling> GameAnalysisProgress<W> {
    /// Waits for all positions to finish analyzing and returns the results.
    ///
    /// Positions that were terminated before any search was performed will not be included in the results.
    pub async fn finish(self) -> WarningResult<HashMap<usize, AnalysisResult>, W> {
        let mut results = W::ok(HashMap::new());
        for (position, progress) in self.into_positions().into_iter() {
            let result = progress.finish().await;
            results = W::merge(results, result, |mut results, result| {
                if let Some(result) = result {
                    results.insert(position, result);
                }
                results
            });
        }
        results
    }

    /// Returns a reference to the raw collection of in-progress analysis operations for each position.
    pub fn positions(&self) -> &HashMap<usize, AnalysisProgress<W>> {
        &self.positions
    }

    /// Returns a mutable reference to the raw collection of in-progress analysis operations for each position.
    pub fn positions_mut(&mut self) -> &mut HashMap<usize, AnalysisProgress<W>> {
        &mut self.positions
    }

    /// Extracts the collection of in-progress analysis operations for each position and consumes this object.
    pub fn into_positions(self) -> HashMap<usize, AnalysisProgress<W>> {
        self.positions
    }
}

impl<W: WarningHandling> std::fmt::Debug for GameAnalysisProgress<W>
where
    W::OkType<Option<AnalysisResult>>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameAnalysisProgress")
            .field("id", &self.id)
            .field("positions", &self.positions)
            .finish()
    }
}

/// An in-progress analysis operation for a single position.
pub struct AnalysisProgress<W: WarningHandling = WarningsAsErrors> {
    receiver: Receiver<WarningResult<Option<AnalysisResult>, W>>,
    id: String,
    turn_number: usize,
}

impl<W: WarningHandling> AnalysisProgress<W> {
    /// Waits for the analysis to finish and returns the result.
    ///
    /// If the analysis was terminated before any search was performed, returns `Ok(None)`.
    pub async fn finish(self) -> WarningResult<Option<AnalysisResult>, W> {
        self.receiver.finish().await
    }

    /// Waits for an analysis update.
    ///
    /// This is mainly useful when using [`AnalysisRequest::report_during_search_every`]. Otherwise, it's simpler to
    /// just call [`finish`](Self::finish) to wait for the final result.
    ///
    /// If the analysis is finished, it consumes this object and returns [`ControlFlow::Break`] containing the final
    /// result. If a partial result is available, it returns [`ControlFlow::Continue`] containing this object again,
    /// which can be read using [`read`](Self::read) or polled again for the next update.
    ///
    /// This method (in combination with [`read`](Self::read)) is conceptually similar to Tokio's
    /// [`watch`](tokio::sync::watch) channel. It provides a way to follow the latest information as it becomes
    /// available without any danger of falling behind.
    ///
    /// # Example
    ///
    /// ```
    /// # use katago_analysis::*;
    /// # use std::ops::ControlFlow;
    /// # async fn example(mut progress: AnalysisProgress) {
    /// loop {
    ///     match progress.poll().await {
    ///         ControlFlow::Break(result) => {
    ///             match result {
    ///                 Ok(Some(result)) => {
    ///                     println!("Winrate: {:.1}%", result.root_info.winrate * 100.0);
    ///                 }
    ///                 Ok(None) => println!("No results"),
    ///                 Err(e) => println!("Error: {e}"),
    ///             }
    ///             break;
    ///         }
    ///         ControlFlow::Continue(p) => {
    ///             progress = p;
    ///             if let Ok(Some(result)) = progress.read().await.as_ref() {
    ///                 println!(
    ///                     "Winrate: {:.1}% Visits: {}",
    ///                     result.root_info.winrate * 100.0,
    ///                     result.root_info.visits
    ///                 );
    ///             }
    ///         }
    ///     };
    /// }
    /// # }
    /// ```
    pub async fn poll(self) -> ControlFlow<WarningResult<Option<AnalysisResult>, W>, Self> {
        self.receiver.poll().await.map_continue(|r| Self {
            receiver: r,
            ..self
        })
    }

    /// Reads the latest analysis result available.
    ///
    /// See also: [`poll`](Self::poll)
    pub async fn read(&self) -> RwLockReadGuard<'_, WarningResult<Option<AnalysisResult>, W>> {
        self.receiver.read().await
    }
}

impl<W: WarningHandling> std::fmt::Debug for AnalysisProgress<W>
where
    W::OkType<Option<AnalysisResult>>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisProgress")
            .field("receiver", &self.receiver)
            .field("id", &self.id)
            .field("turn_number", &self.turn_number)
            .finish()
    }
}
