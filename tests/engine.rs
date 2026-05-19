use std::{env, sync::LazyLock};

use assert_matches::assert_matches;
use katago_analysis::{Bonus, Config, Ko, Player, Rules, Scoring, Tax, engine::*};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

static ENGINE: LazyLock<Mutex<Engine>> = LazyLock::new(|| {
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
    Mutex::new(Engine::launch(&options).unwrap())
});

fn test_request<T: Into<String>>(id: T) -> AnalysisRequest {
    AnalysisRequest::new(
        id.into(),
        Rules::chinese(),
        19,
        19,
        vec![
            (Player::Black, "Q16".to_string()),
            (Player::White, "D4".to_string()),
        ],
    )
}

fn test_lopsided_request<T: Into<String>>(id: T) -> AnalysisRequest {
    AnalysisRequest::new(id.into(), Rules::chinese(), 19, 19, vec![]).with_initial_stones(vec![
        (Player::Black, "Q16".to_string()),
        (Player::Black, "D4".to_string()),
        (Player::Black, "Q4".to_string()),
        (Player::Black, "D16".to_string()),
    ])
}

mod requests {
    use super::*;

    #[tokio::test]
    async fn analyze() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("analyze");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "analyze");
        assert_eq!(response.turn_number, 2);
    }

    #[tokio::test]
    async fn move_infos() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("move_infos");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "move_infos");
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
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
        assert!(mv.order < response.move_infos.len());
        assert!(mv.play_selection_value > 1.0);
        let symm_move = assert_matches!(
            response
                .move_infos
                .iter()
                .find(|m| m.is_symmetry_of.is_some()),
            Some(m) => m
        );
        let orig_move = symm_move.is_symmetry_of.as_ref().unwrap();
        assert_ne!(symm_move.mv, *orig_move);
        let orig_move = assert_matches!(
            response
                .move_infos
                .iter()
                .find(|m| m.mv == *orig_move),
            Some(m) => m
        );
        assert_eq!(symm_move.winrate, orig_move.winrate);
    }

    #[tokio::test]
    async fn root_info() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("root_info");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "root_info");
        assert!(response.root_info.winrate < 0.9);
        assert!(response.root_info.score_lead.abs() < 5.0);
        assert!(response.root_info.score_selfplay.abs() < 5.0);
        assert!(response.root_info.utility.abs() < 1.0);
        assert!(response.root_info.visits >= 4);
        assert_eq!(response.root_info.current_player, Player::Black);
        assert!(response.root_info.raw_winrate < 0.9);
        assert!(response.root_info.raw_lead.abs() < 5.0);
        assert!(response.root_info.raw_score_selfplay.abs() < 5.0);
        assert!(response.root_info.raw_score_selfplay_stdev > 5.0);
        assert!(response.root_info.raw_no_result_prob < 0.01);
        assert!(response.root_info.raw_st_wr_error < 0.1);
        assert!(response.root_info.raw_st_score_error > 0.1);
        assert!(response.root_info.raw_var_time_left > 0.0);
        assert_matches!(response.root_info.human_winrate, Some(wr) if wr < 0.9);
        assert_matches!(response.root_info.human_score_mean, Some(m) if m.abs() < 5.0);
        assert_matches!(response.root_info.human_score_stdev, Some(s) if s > 5.0);
        assert_matches!(response.root_info.human_st_wr_error, Some(e) if e < 0.1);
        assert_matches!(response.root_info.human_st_score_error, Some(e) if e > 0.1);
    }

    #[tokio::test]
    async fn komi() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("komi").with_komi(0.0);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "komi");
        assert_eq!(response.turn_number, 2);
        assert!(response.root_info.winrate > 0.9);
    }

    #[tokio::test]
    async fn white_handicap_bonus() {
        let mut engine = ENGINE.lock().await;
        let request = test_lopsided_request("white_handicap_bonus")
            .with_white_handicap_bonus(Bonus::NMinusOne);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "white_handicap_bonus");
        assert_eq!(response.turn_number, 0);
    }

    #[tokio::test]
    async fn initial_stones() {
        let mut engine = ENGINE.lock().await;
        let request = test_lopsided_request("initial_stones");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "initial_stones");
        assert_eq!(response.turn_number, 0);
        assert!(response.root_info.winrate < 0.1);
    }

    #[tokio::test]
    async fn initial_player() {
        let mut engine = ENGINE.lock().await;
        let request = test_lopsided_request("initial_player").with_initial_player(Player::Black);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "initial_player");
        assert_eq!(response.turn_number, 0);
        assert!(response.root_info.winrate > 0.9);
    }

    #[tokio::test]
    async fn analyze_turns() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("analyze_turns").with_analyze_turns(vec![1, 2]);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let responses = vec![
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
        ];
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number: 1, .. })
                if id == "analyze_turns"
            )
        ));
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number: 2, .. })
                if id == "analyze_turns"
            )
        ));
    }

    #[tokio::test]
    async fn max_visits() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("max_visits").with_max_visits(10);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "max_visits");
        assert_eq!(response.turn_number, 2);
        assert!(response.root_info.visits >= 10);
    }

    #[tokio::test]
    async fn root_policy_temperature() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("root_policy_temperature").with_root_policy_temperature(10.0);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "root_policy_temperature");
        assert_eq!(response.turn_number, 2);
    }

    #[tokio::test]
    async fn root_fpu_reduction_max() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("root_fpu_reduction_max").with_root_fpu_reduction_max(0.0);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "root_fpu_reduction_max");
        assert_eq!(response.turn_number, 2);
    }

    #[tokio::test]
    async fn pv() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("pv");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "pv");
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        assert!(!mv.pv.is_empty());
        assert!(mv.pv_visits.is_none());
        assert!(mv.pv_edge_visits.is_none());
    }

    #[tokio::test]
    async fn analysis_pv_len() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("analysis_pv_len")
            .with_max_visits(20)
            .with_analysis_pv_len(1)
            .with_pv_visits();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "analysis_pv_len");
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        assert_eq!(mv.pv.len(), 2);
        assert_eq!(mv.pv_visits.as_ref().unwrap().len(), 2);
        assert_eq!(mv.pv_edge_visits.as_ref().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn ownership() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("ownership");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "ownership");
        assert!(response.ownership.is_none());
        assert!(response.ownership_stdev.is_none());
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        assert!(mv.ownership.is_none());
        assert!(mv.ownership_stdev.is_none());
    }

    #[tokio::test]
    async fn include_ownership() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("include_ownership").with_ownership();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "include_ownership");
        let ownership = assert_matches!(response.ownership, Some(o) => o);
        assert!(ownership[3 * 19 + 15] > 0.5);
        assert!(response.ownership_stdev.is_none());
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        assert!(mv.ownership.is_none());
        assert!(mv.ownership_stdev.is_none());
    }

    #[tokio::test]
    async fn include_ownership_stdev() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("include_ownership_stdev").with_ownership_stdev();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "include_ownership_stdev");
        assert!(response.ownership.is_none());
        let ownership_stdev = assert_matches!(response.ownership_stdev, Some(s) => s);
        assert!(ownership_stdev[3 * 19 + 15] < 0.1);
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        assert!(mv.ownership.is_none());
        assert!(mv.ownership_stdev.is_none());
    }

    #[tokio::test]
    async fn include_moves_ownership() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("include_moves_ownership").with_moves_ownership();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "include_moves_ownership");
        assert!(response.ownership.is_none());
        assert!(response.ownership_stdev.is_none());
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        let ownership = assert_matches!(mv.ownership.as_ref(), Some(o) => o);
        assert!(ownership[9 * 19 + 9] < 0.5);
        assert!(mv.ownership_stdev.is_none());
    }

    #[tokio::test]
    async fn include_moves_ownership_stdev() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("include_moves_ownership_stdev").with_moves_ownership_stdev();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "include_moves_ownership_stdev");
        assert!(response.ownership.is_none());
        assert!(response.ownership_stdev.is_none());
        assert!(!response.move_infos.is_empty());
        let mv = &response.move_infos[0];
        assert!(mv.ownership.is_none());
        let ownership_stdev = assert_matches!(mv.ownership_stdev.as_ref(), Some(s) => s);
        assert!(ownership_stdev[9 * 19 + 9] < 0.1);
    }

    #[tokio::test]
    async fn policy() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("policy");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "policy");
        assert!(response.policy.is_none());
        assert!(response.human_policy.is_none());
    }

    #[tokio::test]
    async fn include_policy() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("include_policy").with_policy();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "include_policy");
        let policy = assert_matches!(response.policy.as_ref(), Some(p) => p);
        assert_eq!(policy.len(), 19 * 19 + 1);
        assert!(policy[3 * 19 + 3] > 0.1);
        let human_policy = assert_matches!(response.human_policy.as_ref(), Some(p) => p);
        assert_eq!(human_policy.len(), 19 * 19 + 1);
        assert!(human_policy[3 * 19 + 3] > 0.1);
    }

    #[tokio::test]
    #[ignore] // Ignored: Unreleased feature
    async fn include_no_result_value() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("include_no_result_value").with_no_result_value();
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "include_no_result_value");
        assert!(!response.move_infos.is_empty());
        assert_matches!(response.move_infos[0].no_result_value, Some(v) if v < 0.01);
    }

    #[tokio::test]
    async fn override_settings() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("override_settings")
            .with_override_settings(Config::new().with("maxVisits", 1));
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "override_settings");
        assert_eq!(response.turn_number, 2);
        assert_eq!(response.move_infos.len(), 0);
    }

    #[tokio::test]
    async fn report_during_search_every() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("report_during_search_every")
            .with_max_visits(1000)
            .with_report_during_search_every(0.1);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "report_during_search_every");
        assert!(response.is_during_search);
        assert!(response.root_info.visits < 1000);

        engine
            .stdin
            .send(&Request::Terminate {
                id: "report_during_search_every.terminate".to_string(),
                terminate_id: "report_during_search_every".to_string(),
                turn_numbers: None,
            })
            .await
            .unwrap();

        let responses = vec![
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
        ];
        assert!(responses.iter().any(
            |r| matches!(r, Response::Terminate { id, terminate_id, .. }
                if id == "report_during_search_every.terminate" && terminate_id == "report_during_search_every"))
        );
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, is_during_search: false, .. })
                if id == "report_during_search_every"
            )
        ));
    }

    #[tokio::test]
    async fn priority() {
        let mut engine = ENGINE.lock().await;
        let request1 = test_request("priority.1").with_priority(1);
        let request2 = test_request("priority.2").with_priority(-1);
        let request3 = test_request("priority.3");
        engine
            .stdin
            .send(&Request::Analyze(request1))
            .await
            .unwrap();
        engine
            .stdin
            .send(&Request::Analyze(request2))
            .await
            .unwrap();
        engine
            .stdin
            .send(&Request::Analyze(request3))
            .await
            .unwrap();

        let response1 =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        let response2 =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        let response3 =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response1.id, "priority.1");
        assert_eq!(response2.id, "priority.3");
        assert_eq!(response3.id, "priority.2");
    }

    #[tokio::test]
    async fn priorities() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("priorities")
            .with_analyze_turns(vec![0, 1, 2])
            .with_priorities(vec![1, -1, 0]);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response1 =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        let response2 =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        let response3 =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response1.turn_number, 0);
        assert_eq!(response2.turn_number, 2);
        assert_eq!(response3.turn_number, 1);
    }

    #[tokio::test]
    async fn query_version() {
        let mut engine = ENGINE.lock().await;
        engine
            .stdin
            .send(&Request::QueryVersion {
                id: "query_version".to_string(),
            })
            .await
            .unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::QueryVersion { id, .. })) if id == "query_version"
        );
    }

    #[tokio::test]
    async fn clear_cache() {
        let mut engine = ENGINE.lock().await;
        engine
            .stdin
            .send(&Request::ClearCache {
                id: "clear_cache".to_string(),
            })
            .await
            .unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::ClearCache { id })) if id == "clear_cache"
        );
    }

    #[tokio::test]
    async fn terminate() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("terminate.analyze");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();
        engine
            .stdin
            .send(&Request::Terminate {
                id: "terminate".to_string(),
                terminate_id: "terminate.analyze".to_string(),
                turn_numbers: None,
            })
            .await
            .unwrap();

        let responses = vec![
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
        ];
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::Terminate { id, terminate_id, .. }
                    if id == "terminate" && terminate_id == "terminate.analyze"))
        );
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::NoResults { id, turn_number: 2 }
                    if id == "terminate.analyze"
                ))
        );
    }

    #[tokio::test]
    async fn terminate_turn_numbers() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("terminate_turn_numbers.analyze").with_analyze_turns(vec![1, 2]);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();
        engine
            .stdin
            .send(&Request::Terminate {
                id: "terminate_turn_numbers".to_string(),
                terminate_id: "terminate_turn_numbers.analyze".to_string(),
                turn_numbers: Some(vec![1]),
            })
            .await
            .unwrap();

        let responses = vec![
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
        ];
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::Terminate { id, terminate_id, turn_numbers }
                    if id == "terminate_turn_numbers" && terminate_id == "terminate_turn_numbers.analyze"
                ))
        );
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::NoResults { id, turn_number: 1 }
                    if id == "terminate_turn_numbers.analyze"
                ))
        );
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number: 2, .. })
                if id == "terminate_turn_numbers.analyze"
            )
        ));
    }

    #[tokio::test]
    async fn terminate_all() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("terminate_all.analyze");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();
        engine
            .stdin
            .send(&Request::TerminateAll {
                id: "terminate_all".to_string(),
                turn_numbers: None,
            })
            .await
            .unwrap();

        let responses = vec![
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
        ];
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::TerminateAll { id, .. } if id == "terminate_all"))
        );
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::NoResults { id, turn_number: 2 } if id == "terminate_all.analyze"))
        );
    }

    #[tokio::test]
    async fn terminate_all_turn_numbers() {
        let mut engine = ENGINE.lock().await;
        let request =
            test_request("terminate_all_turn_numbers.analyze").with_analyze_turns(vec![1, 2]);
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();
        engine
            .stdin
            .send(&Request::TerminateAll {
                id: "terminate_all_turn_numbers".to_string(),
                turn_numbers: Some(vec![1]),
            })
            .await
            .unwrap();

        let responses = vec![
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
            engine.stdout.next().await.unwrap().unwrap(),
        ];
        assert!(responses.iter().any(
            |r| matches!(r, Response::TerminateAll { id, .. } if id == "terminate_all_turn_numbers")
        ));
        assert!(
            responses
                .iter()
                .any(|r| matches!(r, Response::NoResults { id, turn_number: 1 }
                    if id == "terminate_all_turn_numbers.analyze"
                ))
        );
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number: 2, .. })
                if id == "terminate_all_turn_numbers.analyze"
            )
        ));
    }

    #[tokio::test]
    async fn query_models() {
        let mut engine = ENGINE.lock().await;
        engine
            .stdin
            .send(&Request::QueryModels {
                id: "query_models".to_string(),
            })
            .await
            .unwrap();

        let (id, models) = assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::QueryModels { id, models })) => (id, models)
        );
        assert_eq!(id, "query_models");
        assert_eq!(models.len(), 2);
        assert!(!models[0].uses_humansl_profile);
        assert!(models[1].uses_humansl_profile);
    }

    #[tokio::test]
    async fn general_error() {
        let mut engine = ENGINE.lock().await;
        engine.stdin.send_raw("{").await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::GeneralError { error }))
                if error == "[json.exception.parse_error.101] parse error at line 1, column 2: syntax error while parsing object key - unexpected end of input; expected string literal - could not parse input line as json request: {"
        );
    }

    #[tokio::test]
    async fn field_error() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            board_x_size: 39,
            ..test_request("field_error")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::FieldError { id, field, error }))
                if id == "field_error" && field == "boardXSize" && error == "Must provide an integer from 2 to 19"
        );
    }

    #[tokio::test]
    async fn field_warning() {
        let mut engine = ENGINE.lock().await;
        let request =
            test_request("field_warning").with_override_settings(Config::new().with("foo", "bar"));
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::FieldWarning { id, field, warning }))
                if id == "field_warning" && field == "overrideSettings" && warning == "Unknown config params: foo"
        );
        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, turn_number: 2, .. })))
                if id == "field_warning"
        );
    }
}

mod rules {
    use super::*;

    #[tokio::test]
    async fn named_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::japanese(),
            ..test_request("named_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "named_rules");
    }

    #[tokio::test]
    async fn explicit_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("explicit_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "explicit_rules");
    }

    #[tokio::test]
    async fn japanese_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::japanese(),
            ..test_request("japanese_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "japanese_rules");
    }

    #[tokio::test]
    async fn chinese_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::chinese(),
            ..test_request("chinese_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "chinese_rules");
    }

    #[tokio::test]
    async fn chinese_ogs_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::chinese_ogs(),
            ..test_request("chinese_ogs_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "chinese_ogs_rules");
    }

    #[tokio::test]
    async fn stone_scoring_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::stone_scoring(),
            ..test_request("stone_scoring_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "stone_scoring_rules");
    }

    #[tokio::test]
    async fn ancient_territory_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::ancient_territory(),
            ..test_request("ancient_territory_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "ancient_territory_rules");
    }

    #[tokio::test]
    async fn aga_button_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::aga_button(),
            ..test_request("aga_button_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "aga_button_rules");
    }

    #[tokio::test]
    async fn aga_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::aga(),
            ..test_request("aga_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "aga_rules");
    }

    #[tokio::test]
    async fn new_zealand_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::new_zealand(),
            ..test_request("new_zealand_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "new_zealand_rules");
    }

    #[tokio::test]
    async fn tromp_taylor_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::tromp_taylor(),
            ..test_request("tromp_taylor_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "tromp_taylor_rules");
    }

    #[tokio::test]
    async fn ing_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::ing(),
            ..test_request("ing_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "ing_rules");
    }

    #[tokio::test]
    async fn simple_ko() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Simple,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("simple_ko")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "simple_ko");
    }

    #[tokio::test]
    async fn positional_superko() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("positional_superko")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "positional_superko");
    }

    #[tokio::test]
    async fn situational_superko() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Situational,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("situational_superko")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "situational_superko");
    }

    #[tokio::test]
    async fn area_scoring() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("area_scoring")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "area_scoring");
    }

    #[tokio::test]
    async fn territory_scoring() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Territory,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("territory_scoring")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "territory_scoring");
    }

    #[tokio::test]
    async fn no_tax() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("no_tax")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "no_tax");
    }

    #[tokio::test]
    async fn seki_tax() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::Seki,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("seki_tax")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "seki_tax");
    }

    #[tokio::test]
    async fn group_tax() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::All,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("group_tax")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "group_tax");
    }

    #[tokio::test]
    async fn suicide_illegal() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: false,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("suicide_illegal")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "suicide_illegal");
    }

    #[tokio::test]
    async fn suicide_legal() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("suicide_legal")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "suicide_legal");
    }

    #[tokio::test]
    async fn has_button() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: true,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("has_button")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "has_button");
    }

    #[tokio::test]
    async fn white_handicap_bonus_zero() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("white_handicap_bonus_zero")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "white_handicap_bonus_zero");
    }

    #[tokio::test]
    async fn white_handicap_bonus_n_minus_one() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::NMinusOne,
                friendly_pass_ok: false,
            },
            ..test_request("white_handicap_bonus_n_minus_one")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "white_handicap_bonus_n_minus_one");
    }

    #[tokio::test]
    async fn white_handicap_bonus_n() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::N,
                friendly_pass_ok: false,
            },
            ..test_request("white_handicap_bonus_n")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "white_handicap_bonus_n");
    }

    #[tokio::test]
    async fn friendly_pass_ok() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: true,
            },
            ..test_request("friendly_pass_ok")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "friendly_pass_ok");
    }

    #[tokio::test]
    async fn friendly_pass_not_ok() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::Explicit {
                ko: Ko::Positional,
                scoring: Scoring::Area,
                tax: Tax::None,
                suicide: true,
                has_button: false,
                white_handicap_bonus: Bonus::Zero,
                friendly_pass_ok: false,
            },
            ..test_request("friendly_pass_not_ok")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        let response =
            assert_matches!(engine.stdout.next().await, Some(Ok(Response::Analyze(r))) => r);
        assert_eq!(response.id, "friendly_pass_not_ok");
    }
}
