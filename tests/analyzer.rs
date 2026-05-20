use std::{
    env,
    ops::{ControlFlow, DerefMut},
    sync::{Arc, Mutex},
};

use assert_matches::assert_matches;
use katago_analysis::{
    engine::{Engine, LaunchOptions},
    *,
};
use libtest_mimic::{Arguments, Trial};
use tokio::task::JoinSet;

macro_rules! test {
    ($name:ident, $input:expr) => {
        Trial::test(stringify!($name), {
            let input = $input.clone();
            move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on($name(
                        input.lock().unwrap_or_else(|e| e.into_inner()).deref_mut(),
                    ));
                Ok(())
            }
        })
    };
}

#[tokio::main]
async fn main() {
    _ = dotenv::dotenv();
    let options = LaunchOptions::new(
        env::var("KATAGO_PATH").expect("KATAGO_PATH environment variable not set"),
        "test_analysis.cfg".to_string(),
        env::var("KATAGO_MODEL_PATH").expect("KATAGO_MODEL_PATH environment variable not set"),
    )
    .with_human_model(
        env::var("KATAGO_HUMAN_MODEL_PATH")
            .expect("KATAGO_HUMAN_MODEL_PATH environment variable not set"),
    );
    let analyzer = Arc::new(Mutex::new(Analyzer::from(
        Engine::launch(&options).unwrap(),
    )));

    let tests = vec![
        test!(analyze, analyzer),
        test!(analyze_position, analyzer),
        test!(analyze_game, analyzer),
        test!(analyze_positions, analyzer),
        test!(move_infos, analyzer),
        test!(root_info, analyzer),
        test!(komi, analyzer),
        test!(white_handicap_bonus, analyzer),
        test!(initial_stones, analyzer),
        test!(initial_player, analyzer),
        test!(max_visits, analyzer),
        test!(root_policy_temperature, analyzer),
        test!(root_fpu_reduction_max, analyzer),
        test!(pv, analyzer),
        test!(analysis_pv_len, analyzer),
        test!(ownership, analyzer),
        test!(include_ownership, analyzer),
        test!(include_ownership_stdev, analyzer),
        test!(include_moves_ownership, analyzer),
        test!(include_moves_ownership_stdev, analyzer),
        test!(policy, analyzer),
        test!(include_policy, analyzer),
        test!(include_no_result_value, analyzer).with_ignored_flag(true), // Ignored: Unreleased feature
        test!(avoid_moves, analyzer),
        test!(allow_moves, analyzer),
        test!(override_settings, analyzer),
        test!(report_during_search_every, analyzer),
        test!(priority, analyzer),
        test!(priorities, analyzer),
        test!(pass, analyzer),
        test!(query_version, analyzer),
        test!(clear_cache, analyzer),
        test!(terminate, analyzer),
        test!(terminate_game, analyzer),
        test!(terminate_positions, analyzer),
        test!(terminate_all, analyzer),
        test!(terminate_all_positions, analyzer),
        test!(query_models, analyzer),
        test!(field_error, analyzer),
        test!(field_warning, analyzer),
    ];
    libtest_mimic::run(&Arguments::from_args(), tests).exit_if_failed();
}

fn test_request() -> AnalysisRequest {
    AnalysisRequest::new(
        Rules::chinese(),
        19,
        19,
        vec![
            (Player::Black, Move::Move(Coord(15, 3))),
            (Player::White, Move::Move(Coord(3, 15))),
        ],
    )
}

fn test_lopsided_request() -> AnalysisRequest {
    AnalysisRequest::new(Rules::chinese(), 19, 19, vec![]).with_initial_stones(vec![
        (Player::Black, Coord(15, 3)),
        (Player::Black, Coord(3, 15)),
        (Player::Black, Coord(15, 15)),
        (Player::Black, Coord(3, 3)),
    ])
}

async fn analyze(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert_eq!(result.turn_number, 2);
    assert_eq!(result.is_during_search, false);
    assert_eq!(result.root_info.visits, 4);
    assert!(!result.move_infos.is_empty());
    assert!(result.move_infos[0].visits < result.root_info.visits);
}

async fn analyze_position(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(
        analyzer.analyze_position(request, 1).await,
        Ok(Some(r)) => r
    );
    assert_eq!(result.turn_number, 1);
    assert_eq!(result.is_during_search, false);
    assert_eq!(result.root_info.visits, 4);
}

async fn analyze_game(analyzer: &mut Analyzer) {
    let request = test_request();

    let mut results = assert_matches!(
        analyzer.analyze_game(request).await,
        Ok(r) => r
    );
    let pos0 = assert_matches!(results.remove(&0), Some(r) => r);
    let pos1 = assert_matches!(results.remove(&1), Some(r) => r);
    let pos2 = assert_matches!(results.remove(&2), Some(r) => r);
    assert!(results.is_empty());
    assert_eq!(pos0.turn_number, 0);
    assert_eq!(pos1.turn_number, 1);
    assert_eq!(pos2.turn_number, 2);
}

async fn analyze_positions(analyzer: &mut Analyzer) {
    let request = test_request();

    let mut results = assert_matches!(
        analyzer.analyze_positions(request, vec![0, 1]).await,
        Ok(r) => r
    );
    let pos0 = assert_matches!(results.remove(&0), Some(r) => r);
    let pos1 = assert_matches!(results.remove(&1), Some(r) => r);
    assert!(results.is_empty());
    assert_eq!(pos0.turn_number, 0);
    assert_eq!(pos1.turn_number, 1);
}

async fn move_infos(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert!(mv.visits > 0);
    assert!(mv.edge_visits > 0);
    assert!(mv.winrate < 0.9);
    assert!(mv.score_lead.abs() < 5.0);
    assert!(mv.score_stdev > 5.0);
    assert!(mv.score_selfplay.abs() < 5.0);
    assert!(mv.prior > 0.1);
    assert_matches!(mv.human_prior, Some(p) if p > 0.1);
    assert!(mv.utility.abs() < 1.0);
    assert!(mv.lcb < mv.winrate);
    assert!(mv.utility_lcb < mv.utility);
    assert!(mv.weight > 0.0);
    assert!(mv.edge_weight > 0.0);
    assert!(mv.order < result.move_infos.len());
    assert!(mv.play_selection_value > 1.0);
    let symm_move = assert_matches!(
        result
            .move_infos
            .iter()
            .find(|m| m.is_symmetry_of.is_some()),
        Some(m) => m
    );
    let orig_move = Move::Move(symm_move.is_symmetry_of.unwrap());
    assert_ne!(symm_move.mv, orig_move);
    let orig_move = assert_matches!(
        result
            .move_infos
            .iter()
            .find(|m| m.mv == orig_move),
        Some(m) => m
    );
    assert_eq!(symm_move.winrate, orig_move.winrate);
}

async fn root_info(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.root_info.winrate < 0.9);
    assert!(result.root_info.score_lead.abs() < 5.0);
    assert!(result.root_info.score_selfplay.abs() < 5.0);
    assert!(result.root_info.utility.abs() < 1.0);
    assert!(result.root_info.visits >= 4);
    assert_eq!(result.root_info.current_player, Player::Black);
    assert!(result.root_info.raw_winrate < 0.9);
    assert!(result.root_info.raw_lead.abs() < 5.0);
    assert!(result.root_info.raw_score_selfplay.abs() < 5.0);
    assert!(result.root_info.raw_score_selfplay_stdev > 5.0);
    assert!(result.root_info.raw_no_result_prob < 0.01);
    assert!(result.root_info.raw_st_wr_error < 0.1);
    assert!(result.root_info.raw_st_score_error > 0.1);
    assert!(result.root_info.raw_var_time_left > 0.0);
    assert_matches!(result.root_info.human_winrate, Some(wr) if wr < 0.9);
    assert_matches!(result.root_info.human_score_mean, Some(m) if m.abs() < 5.0);
    assert_matches!(result.root_info.human_score_stdev, Some(s) if s > 5.0);
    assert_matches!(result.root_info.human_st_wr_error, Some(e) if e < 0.1);
    assert_matches!(result.root_info.human_st_score_error, Some(e) if e > 0.1);
}

async fn komi(analyzer: &mut Analyzer) {
    let request = test_request().with_komi(0.0);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.root_info.winrate > 0.9);
}

async fn white_handicap_bonus(analyzer: &mut Analyzer) {
    let request = test_lopsided_request().with_white_handicap_bonus(Bonus::NMinusOne);

    assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
}

async fn initial_stones(analyzer: &mut Analyzer) {
    let request = test_lopsided_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.root_info.winrate < 0.1);
}

async fn initial_player(analyzer: &mut Analyzer) {
    let request = test_lopsided_request().with_initial_player(Player::Black);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.root_info.winrate > 0.9);
}

async fn max_visits(analyzer: &mut Analyzer) {
    let request = test_request().with_max_visits(10);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert_eq!(result.root_info.visits, 10);
}

async fn root_policy_temperature(analyzer: &mut Analyzer) {
    let request = test_request().with_root_policy_temperature(10.0);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert_eq!(result.turn_number, 2);
}

async fn root_fpu_reduction_max(analyzer: &mut Analyzer) {
    let request = test_request().with_root_fpu_reduction_max(0.0);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert_eq!(result.turn_number, 2);
}

async fn pv(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert!(!mv.pv.is_empty());
    assert!(mv.pv_visits.is_none());
    assert!(mv.pv_edge_visits.is_none());
}

async fn analysis_pv_len(analyzer: &mut Analyzer) {
    let request = test_request()
        .with_max_visits(20)
        .with_analysis_pv_len(1)
        .with_pv_visits();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert_eq!(mv.pv.len(), 2);
    assert_eq!(mv.pv_visits.as_ref().unwrap().len(), 2);
    assert_eq!(mv.pv_edge_visits.as_ref().unwrap().len(), 2);
}

async fn ownership(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.ownership.is_none());
    assert!(result.ownership_stdev.is_none());
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert!(mv.ownership.is_none());
    assert!(mv.ownership_stdev.is_none());
}

async fn include_ownership(analyzer: &mut Analyzer) {
    let request = test_request().with_ownership();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    let ownership = assert_matches!(result.ownership.as_ref(), Some(m) => m);
    assert!(*ownership.get(15, 3) > 0.5);
    assert!(result.ownership_stdev.is_none());
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert!(mv.ownership.is_none());
    assert!(mv.ownership_stdev.is_none());
}

async fn include_ownership_stdev(analyzer: &mut Analyzer) {
    let request = test_request().with_ownership_stdev();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.ownership.is_none());
    let ownership_stdev = assert_matches!(result.ownership_stdev.as_ref(), Some(m) => m);
    assert!(*ownership_stdev.get(15, 3) < 0.1);
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert!(mv.ownership.is_none());
    assert!(mv.ownership_stdev.is_none());
}

async fn include_moves_ownership(analyzer: &mut Analyzer) {
    let request = test_request().with_moves_ownership();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.ownership.is_none());
    assert!(result.ownership_stdev.is_none());
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    let ownership = assert_matches!(mv.ownership.as_ref(), Some(m) => m);
    assert!(*ownership.get(9, 9) < 0.5);
    assert!(mv.ownership_stdev.is_none());
}

async fn include_moves_ownership_stdev(analyzer: &mut Analyzer) {
    let request = test_request().with_moves_ownership_stdev();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.ownership.is_none());
    assert!(result.ownership_stdev.is_none());
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert!(mv.ownership.is_none());
    let ownership_stdev = assert_matches!(mv.ownership_stdev.as_ref(), Some(m) => m);
    assert!(*ownership_stdev.get(9, 9) < 0.1);
}

async fn policy(analyzer: &mut Analyzer) {
    let request = test_request();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.policy.is_none());
    assert!(result.policy_pass.is_none());
    assert!(result.human_policy.is_none());
    assert!(result.human_policy_pass.is_none());
}

async fn include_policy(analyzer: &mut Analyzer) {
    let request = test_request().with_policy();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    let policy = assert_matches!(result.policy.as_ref(), Some(p) => p);
    assert!(*policy.get(3, 3) > 0.1);
    assert_matches!(result.policy_pass, Some(p) if p < 0.01);
    let human_policy = assert_matches!(result.human_policy.as_ref(), Some(p) => p);
    assert!(*human_policy.get(3, 3) > 0.1);
    assert_matches!(result.human_policy_pass, Some(p) if p < 0.01);
}

async fn include_no_result_value(analyzer: &mut Analyzer) {
    let request = test_request().with_no_result_value();

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(!result.move_infos.is_empty());
    let mv = &result.move_infos[0];
    assert_matches!(mv.no_result_value, Some(v) if v < 0.01);
}

async fn avoid_moves(analyzer: &mut Analyzer) {
    let request = test_request().with_avoid_moves(vec![RestrictedMoves {
        player: Player::Black,
        moves: vec![Move::Move(Coord(3, 3))],
        until_depth: 1,
    }]);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(!result.move_infos.is_empty());
    assert!(
        !result
            .move_infos
            .iter()
            .any(|mv| mv.mv == Move::Move(Coord(3, 3)))
    );
}

async fn allow_moves(analyzer: &mut Analyzer) {
    let request = test_request().with_allow_moves(vec![RestrictedMoves {
        player: Player::Black,
        moves: vec![Move::Move(Coord(3, 3))],
        until_depth: 1,
    }]);

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert_eq!(result.move_infos.len(), 1);
    assert_eq!(result.move_infos[0].mv, Move::Move(Coord(3, 3)));
}

async fn override_settings(analyzer: &mut Analyzer) {
    let request = test_request().with_override_settings(Config::new().with("maxVisits", 1));

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert!(result.move_infos.is_empty());
}

async fn report_during_search_every(analyzer: &mut Analyzer) {
    let request = test_request()
        .with_report_during_search_every(0.01)
        .with_max_visits(100);
    let progress = analyzer.start_analyze(request).await.unwrap();

    let progress = assert_matches!(progress.poll().await, ControlFlow::Continue(p) => p);
    {
        let guard = progress.read().await;
        let result = assert_matches!(guard.as_ref(), Ok(Some(r)) => r);
        assert!(result.is_during_search);
        assert!(result.root_info.visits < 100);
    }
    analyzer.terminate(&progress).await.unwrap();
    let result = assert_matches!(progress.finish().await, Ok(Some(r)) => r);
    assert!(!result.is_during_search);
    assert!(result.root_info.visits < 100);
}

async fn priority(analyzer: &mut Analyzer) {
    let progress1 = analyzer
        .start_analyze(test_request().with_priority(1))
        .await
        .unwrap();
    let progress2 = analyzer
        .start_analyze(test_request().with_priority(-1))
        .await
        .unwrap();
    let progress3 = analyzer.start_analyze(test_request()).await.unwrap();

    let mut set = JoinSet::new();
    set.spawn(async {
        let _ = progress1.finish().await;
        1
    });
    set.spawn(async {
        let _ = progress2.finish().await;
        2
    });
    set.spawn(async {
        let _ = progress3.finish().await;
        3
    });
    let result1 = set.join_next().await.unwrap().unwrap();
    let result2 = set.join_next().await.unwrap().unwrap();
    let result3 = set.join_next().await.unwrap().unwrap();
    assert_eq!(result1, 1);
    assert_eq!(result2, 3);
    assert_eq!(result3, 2);
}

async fn priorities(analyzer: &mut Analyzer) {
    let mut positions = analyzer
        .start_analyze_game_prioritized(test_request(), vec![1, -1, 0])
        .await
        .unwrap()
        .into_positions();
    let progress1 = positions.remove(&0).unwrap();
    let progress2 = positions.remove(&1).unwrap();
    let progress3 = positions.remove(&2).unwrap();

    let mut set = JoinSet::new();
    set.spawn(async {
        let _ = progress1.finish().await;
        1
    });
    set.spawn(async {
        let _ = progress2.finish().await;
        2
    });
    set.spawn(async {
        let _ = progress3.finish().await;
        3
    });
    let result1 = set.join_next().await.unwrap().unwrap();
    let result2 = set.join_next().await.unwrap().unwrap();
    let result3 = set.join_next().await.unwrap().unwrap();
    assert_eq!(result1, 1);
    assert_eq!(result2, 3);
    assert_eq!(result3, 2);
}

async fn pass(analyzer: &mut Analyzer) {
    let mut request = test_request();
    request.moves.push((Player::Black, Move::Pass));

    let result = assert_matches!(analyzer.analyze(request).await, Ok(Some(r)) => r);
    assert_eq!(result.turn_number, 3);
    assert!(result.root_info.winrate > 0.9);
}

async fn query_version(analyzer: &mut Analyzer) {
    let version_info = assert_matches!(analyzer.query_version().await, Ok(v) => v);
    assert!(!version_info.version.is_empty());
    assert!(!version_info.git_hash.is_empty());
}

async fn clear_cache(analyzer: &mut Analyzer) {
    assert_matches!(analyzer.clear_cache().await, Ok(()));
}

async fn terminate(analyzer: &mut Analyzer) {
    let request = test_request();
    let progress = analyzer.start_analyze(request).await.unwrap();

    assert_matches!(analyzer.terminate(&progress).await, Ok(()));
    assert_matches!(progress.finish().await, Ok(None));
}

async fn terminate_game(analyzer: &mut Analyzer) {
    let request = test_request();
    let progress = analyzer.start_analyze_game(request).await.unwrap();

    assert_matches!(analyzer.terminate_game(&progress).await, Ok(()));
    let results = assert_matches!(progress.finish().await, Ok(r) => r);
    assert!(results.is_empty());
}

async fn terminate_positions(analyzer: &mut Analyzer) {
    let request = test_request();
    let progress = analyzer.start_analyze_game(request).await.unwrap();

    assert_matches!(
        analyzer.terminate_positions(&progress, vec![0, 1]).await,
        Ok(())
    );
    let results = assert_matches!(progress.finish().await, Ok(r) => r);
    assert_eq!(results.len(), 1);
    let result = assert_matches!(results.get(&2), Some(r) => r);
    assert!(result.root_info.visits >= 4);
}

async fn terminate_all(analyzer: &mut Analyzer) {
    let request1 = test_request();
    let progress1 = analyzer.start_analyze(request1).await.unwrap();
    let request2 = test_request();
    let progress2 = analyzer.start_analyze_position(request2, 0).await.unwrap();

    assert_matches!(analyzer.terminate_all().await, Ok(()));
    assert_matches!(progress1.finish().await, Ok(None));
    assert_matches!(progress2.finish().await, Ok(None));
}

async fn terminate_all_positions(analyzer: &mut Analyzer) {
    let request1 = test_request();
    let progress1 = analyzer
        .start_analyze_positions(request1, vec![1, 2])
        .await
        .unwrap();
    let request2 = test_request();
    let progress2 = analyzer
        .start_analyze_positions(request2, vec![0, 1])
        .await
        .unwrap();

    assert_matches!(analyzer.terminate_all_positions(vec![1]).await, Ok(()));
    let result1 = assert_matches!(progress1.finish().await, Ok(r) => r);
    let result2 = assert_matches!(progress2.finish().await, Ok(r) => r);
    assert_eq!(result1.len(), 1);
    assert!(result1.contains_key(&2));
    assert_eq!(result2.len(), 1);
    assert!(result2.contains_key(&0));
}

async fn query_models(analyzer: &mut Analyzer) {
    let models = assert_matches!(analyzer.query_models().await, Ok(m) => m);
    assert_eq!(models.len(), 2);
    assert!(!models[0].uses_humansl_profile);
    assert!(models[1].uses_humansl_profile);
}

async fn field_error(analyzer: &mut Analyzer) {
    let request = test_request().with_komi(361.0);

    let (error, field) = assert_matches!(
        analyzer.analyze(request).await,
        Err(Error::KataGoFieldError { error, field }) => (error, field)
    );
    assert_eq!(
        error,
        "Must be a integer or half-integer from -150.0 to 150.0"
    );
    assert_eq!(field, "komi");
}

async fn field_warning(analyzer: &mut Analyzer) {
    let request = test_request().with_override_settings(Config::new().with("foo", "bar"));

    let warnings = assert_matches!(
        analyzer.analyze(request).await,
        Err(Error::UnhandledWarnings(warnings)) => warnings
    );
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].field, "overrideSettings");
    assert_eq!(warnings[0].warning, "Unknown config params: foo");
}
