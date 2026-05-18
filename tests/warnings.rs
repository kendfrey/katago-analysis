use std::env;

use assert_matches::assert_matches;
use katago_analysis::{
    engine::{Engine, LaunchOptions},
    *,
};
use tokio::sync::Mutex;

static MUTEX: Mutex<()> = Mutex::const_new(());

fn launch_options() -> LaunchOptions {
    _ = dotenv::dotenv();
    LaunchOptions::new(
        env::var("KATAGO_PATH").expect("KATAGO_PATH environment variable not set"),
        "test_analysis.cfg".to_string(),
        env::var("KATAGO_MODEL_PATH").expect("KATAGO_MODEL_PATH environment variable not set"),
    )
    .with_human_model(
        env::var("KATAGO_HUMAN_MODEL_PATH")
            .expect("KATAGO_HUMAN_MODEL_PATH environment variable not set"),
    )
}

fn warning_request() -> AnalysisRequest {
    AnalysisRequest::new(
        Rules::chinese(),
        19,
        19,
        vec![
            (Player::Black, Move::Move(Coord(15, 3))),
            (Player::White, Move::Move(Coord(3, 15))),
        ],
    )
    .with_override_settings(Config::new().with("foo", "bar"))
}

#[tokio::test]
async fn return_warnings() {
    let _guard = MUTEX.lock().await;
    let mut analyzer: Analyzer<ReturnWarnings> = Engine::launch(&launch_options()).unwrap().into();

    let (result, warnings) = assert_matches!(
        analyzer.analyze(warning_request()).await,
        Ok(MaybeWarnings { value: Some(result), warnings: Some(warnings) }) => (result, warnings)
    );
    assert_eq!(result.turn_number, 2);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].field, "overrideSettings");
    assert_eq!(warnings[0].warning, "Unknown config params: foo");
}

#[tokio::test]
async fn ignore_warnings() {
    let _guard = MUTEX.lock().await;
    let mut analyzer: Analyzer<IgnoreWarnings> = Engine::launch(&launch_options()).unwrap().into();

    let result =
        assert_matches!(analyzer.analyze(warning_request()).await, Ok(Some(result)) => result);
    assert_eq!(result.turn_number, 2);
}
