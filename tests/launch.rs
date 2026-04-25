use std::env;

use assert_matches::assert_matches;
use katago_analysis::{engine::*, *};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

static MUTEX: Mutex<()> = Mutex::const_new(());

fn launch_options() -> LaunchOptions {
    _ = dotenv::dotenv();
    LaunchOptions::new(
        env::var("KATAGO_PATH").expect("KATAGO_PATH environment variable not set"),
        "test_analysis.cfg".to_string(),
        env::var("KATAGO_MODEL_PATH").expect("KATAGO_MODEL_PATH environment variable not set"),
    )
}

#[tokio::test]
async fn pipe_stderr() {
    let _guard = MUTEX.lock().await;
    let mut engine = Engine::launch(&launch_options()).unwrap();
    assert_matches!(engine.stderr, Some(_));
    engine.child_process.kill().await.unwrap();
}

#[tokio::test]
async fn inherit_stderr() {
    let _guard = MUTEX.lock().await;
    let mut engine = Engine::launch(&launch_options().with_inherit_stderr()).unwrap();
    assert_matches!(engine.stderr, None);
    engine.child_process.kill().await.unwrap();
}

#[tokio::test]
async fn human_model() {
    let _guard = MUTEX.lock().await;
    let options = launch_options().with_human_model(
        env::var("KATAGO_HUMAN_MODEL_PATH")
            .expect("KATAGO_HUMAN_MODEL_PATH environment variable not set"),
    );
    let mut engine = Engine::launch(&options).unwrap();
    engine
        .stdin
        .send(&Request::QueryModels {
            id: "human_model".to_string(),
        })
        .await
        .unwrap();

    assert_matches!(
        engine.stdout.next().await,
        Some(Ok(Response::QueryModels { id, models }))
            if id == "human_model" && !models[0].uses_humansl_profile && models[1].uses_humansl_profile
    );
}

#[tokio::test]
async fn override_config() {
    let _guard = MUTEX.lock().await;
    let options = launch_options().with_override_config(Config::new().with("maxVisits", 1));
    let mut engine = Engine::launch(&options).unwrap();
    let request = AnalysisRequest::new(
        "override_config".to_string(),
        Rules::chinese(),
        19,
        19,
        vec![],
    );
    engine.stdin.send(&Request::Analyze(request)).await.unwrap();

    assert_matches!(
        engine.stdout.next().await,
        Some(Ok(Response::Analyze(AnalysisResponse { id, move_infos, .. })))
            if id == "override_config" && move_infos.is_empty()
    );
}
