use std::env;

use assert_matches::assert_matches;
use katago_analysis::{Config, Rules, Side, engine::*};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::Mutex,
};
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
async fn no_human_model() {
    let _guard = MUTEX.lock().await;
    let options = launch_options();
    let mut engine = Engine::launch(&options).unwrap();
    engine
        .stdin
        .send(&Request::QueryModels {
            id: "no_human_model".to_string(),
        })
        .await
        .unwrap();

    let (id, models) = assert_matches!(
        engine.stdout.next().await,
        Some(Ok(Response::QueryModels { id, models })) => (id, models)
    );
    assert_eq!(id, "no_human_model");
    assert_eq!(models.len(), 1);
    assert!(!models[0].uses_humansl_profile);

    let request = AnalysisRequest::new(
        "no_human_model.analyze".to_string(),
        Rules::chinese(),
        19,
        19,
        vec![],
    );
    engine.stdin.send(&Request::Analyze(request)).await.unwrap();

    let response = assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
    assert_eq!(response.id, "no_human_model.analyze");
    assert!(!response.move_infos.is_empty());
    assert!(response.move_infos[0].human_prior.is_none());
    assert!(response.root_info.human_winrate.is_none());
    assert!(response.root_info.human_score_mean.is_none());
    assert!(response.root_info.human_score_stdev.is_none());
    assert!(response.root_info.human_st_wr_error.is_none());
    assert!(response.root_info.human_st_score_error.is_none());
}

#[tokio::test]
async fn override_config() {
    let _guard = MUTEX.lock().await;
    let options = launch_options().with_override_config(Config::new().with_max_visits(1));
    let mut engine = Engine::launch(&options).unwrap();
    let request = AnalysisRequest::new(
        "override_config".to_string(),
        Rules::chinese(),
        19,
        19,
        vec![],
    );
    engine.stdin.send(&Request::Analyze(request)).await.unwrap();

    let response = assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
    assert_eq!(response.id, "override_config");
    assert_eq!(response.move_infos.len(), 0);
}

#[tokio::test]
async fn quit_without_waiting() {
    let _guard = MUTEX.lock().await;
    let options = launch_options().with_quit_without_waiting();
    let mut engine = Engine::launch(&options).unwrap();
    let request = AnalysisRequest::new(
        "quit_without_waiting".to_string(),
        Rules::chinese(),
        19,
        19,
        vec![],
    );
    engine.stdin.send(&Request::Analyze(request)).await.unwrap();
    drop(engine.stdin);

    assert_matches!(engine.stdout.next().await, None);
}

#[tokio::test]
async fn config() {
    let _guard = MUTEX.lock().await;
    let config = Config::new()
        .with_analysis_pv_len(50)
        .with_anti_mirror(true)
        .with_human_sl_profile("proyear_2023")
        .with_ignore_pre_root_history(false)
        .with_max_time(1.0)
        .with_max_visits(1)
        .with_num_analysis_threads(1)
        .with_num_search_threads_per_analysis_thread(1)
        .with_playout_doubling_advantage(3.0)
        .with_report_analysis_winrates_as(Side::SideToMove)
        .with_root_num_symmetries_to_sample(8)
        .with_wide_root_noise(1.0);
    let mut engine = Engine::launch(&launch_options().with_override_config(config)).unwrap();
    let mut lines = BufReader::new(engine.stderr.take().unwrap()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        println!("> {line}");
        assert!(!line.contains("Unused key"));
        if line.contains("Started, ready to begin handling requests") {
            return;
        }
    }
    assert!(false, "Engine did not start successfully");
}
