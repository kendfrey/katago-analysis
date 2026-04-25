use std::{env, sync::LazyLock};

use assert_matches::assert_matches;
use katago_analysis::{engine::*, *};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

static ENGINE: LazyLock<Mutex<Engine>> = LazyLock::new(|| {
    _ = dotenv::dotenv();
    let config = LaunchOptions::new(
        env::var("KATAGO_PATH").expect("KATAGO_PATH environment variable not set"),
        "test_analysis.cfg".to_string(),
        env::var("KATAGO_MODEL_PATH").expect("KATAGO_MODEL_PATH environment variable not set"),
    );
    Mutex::new(Engine::launch(&config).unwrap())
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

mod requests {
    use super::*;

    #[tokio::test]
    async fn analyze() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("analyze");
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, turn_number, .. })))
                if id == "analyze" && turn_number == 2
        );
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
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number, .. })
                if id == "analyze_turns" && *turn_number == 1
            )
        ));
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number, .. })
                if id == "analyze_turns" && *turn_number == 2
            )
        ));
    }

    #[tokio::test]
    async fn override_settings() {
        let mut engine = ENGINE.lock().await;
        let request = test_request("override_settings")
            .with_override_settings(Config::new().with("maxVisits", 1));
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, turn_number, move_infos, .. })))
                if id == "override_settings" && turn_number == 2 && move_infos.is_empty()
        );
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
                .any(|r| matches!(r, Response::NoResults { id, turn_number }
                    if id == "terminate.analyze" && *turn_number == 2
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
                .any(|r| matches!(r, Response::NoResults { id, turn_number }
                    if id == "terminate_turn_numbers.analyze" && *turn_number == 1
                ))
        );
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number, .. })
                if id == "terminate_turn_numbers.analyze" && *turn_number == 2
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
                .any(|r| matches!(r, Response::NoResults { id, turn_number }
                    if id == "terminate_all.analyze" && *turn_number == 2
                ))
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
                .any(|r| matches!(r, Response::NoResults { id, turn_number }
                    if id == "terminate_all_turn_numbers.analyze" && *turn_number == 1
                ))
        );
        assert!(responses.iter().any(
            |r| matches!(r, Response::Analyze(AnalysisResponse { id, turn_number, .. })
                if id == "terminate_all_turn_numbers.analyze" && *turn_number == 2
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::QueryModels { id, models }))
                if id == "query_models" && models.iter().any(|m| !m.uses_humansl_profile)
        );
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
            Some(Ok(Response::Analyze(AnalysisResponse { id, turn_number, .. })))
                if id == "field_warning" && turn_number == 2
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "named_rules"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "explicit_rules"
        );
    }

    #[tokio::test]
    async fn japanese_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::japanese(),
            ..test_request("japanese_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "japanese_rules"
        );
    }

    #[tokio::test]
    async fn chinese_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::chinese(),
            ..test_request("chinese_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "chinese_rules"
        );
    }

    #[tokio::test]
    async fn chinese_ogs_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::chinese_ogs(),
            ..test_request("chinese_ogs_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "chinese_ogs_rules"
        );
    }

    #[tokio::test]
    async fn stone_scoring_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::stone_scoring(),
            ..test_request("stone_scoring_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "stone_scoring_rules"
        );
    }

    #[tokio::test]
    async fn ancient_territory_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::ancient_territory(),
            ..test_request("ancient_territory_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "ancient_territory_rules"
        );
    }

    #[tokio::test]
    async fn aga_button_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::aga_button(),
            ..test_request("aga_button_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "aga_button_rules"
        );
    }

    #[tokio::test]
    async fn aga_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::aga(),
            ..test_request("aga_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "aga_rules"
        );
    }

    #[tokio::test]
    async fn new_zealand_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::new_zealand(),
            ..test_request("new_zealand_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "new_zealand_rules"
        );
    }

    #[tokio::test]
    async fn tromp_taylor_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::tromp_taylor(),
            ..test_request("tromp_taylor_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "tromp_taylor_rules"
        );
    }

    #[tokio::test]
    async fn ing_rules() {
        let mut engine = ENGINE.lock().await;
        let request = AnalysisRequest {
            rules: Rules::ing(),
            ..test_request("ing_rules")
        };
        engine.stdin.send(&Request::Analyze(request)).await.unwrap();

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "ing_rules"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "simple_ko"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "positional_superko"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "situational_superko"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "area_scoring"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "territory_scoring"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "no_tax"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "seki_tax"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "group_tax"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "suicide_illegal"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "suicide_legal"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "has_button"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "white_handicap_bonus_zero"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "white_handicap_bonus_n_minus_one"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "white_handicap_bonus_n"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "friendly_pass_ok"
        );
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

        assert_matches!(
            engine.stdout.next().await,
            Some(Ok(Response::Analyze(AnalysisResponse { id, .. }))) if id == "friendly_pass_not_ok"
        );
    }
}
