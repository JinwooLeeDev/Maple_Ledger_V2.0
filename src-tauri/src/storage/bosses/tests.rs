use super::*;
use crate::storage::tests::{list_of, selection, TestDatabase};
fn timestamp() -> i64 {
    parse_day("2026-10-28").unwrap() * 86400
}

#[test]
fn scheduler_cache_survives_empty_failure_reopen_and_week_change() {
    use crate::nexon::scheduler::{BossContent, SchedulerState};
    let db = fixture();
    let repo = db.repo();
    let character = repo.load_configuration().unwrap().managed.characters[0].clone();
    let state = |entries| SchedulerState {
        date: "2026-10-28".into(),
        character_name: character.character_name.clone().unwrap(),
        world_name: "w".into(),
        boss_contents: entries,
    };
    assert!(repo
        .boss_view(None, None, timestamp())
        .unwrap()
        .scheduler
        .is_none());
    let entry = |name: &str, difficulty: &str, registered: &str| BossContent {
        content_name: name.into(),
        difficulty: difficulty.into(),
        cycle: "bossWeekly".into(),
        registration_flag: Some(registered.into()),
        complete_flag: "true".into(),
    };
    repo.store_boss_observation(
        "one",
        1,
        "2026-10-28",
        state(vec![
            entry("피에르", "chaos", "true"),
            entry("미확인", "new", "true"),
            entry("자쿰", "chaos", "false"),
        ]),
        timestamp(),
    )
    .unwrap();
    let view = repo.boss_view(None, None, timestamp()).unwrap();
    let cached = view.scheduler.unwrap();
    assert_eq!(cached.entries.len(), 2);
    assert_eq!(
        cached.entries[0].variant_id.as_deref(),
        Some("pierre-chaos")
    );
    assert!(cached.entries[1].variant_id.is_none());
    assert!(view.records.is_empty());
    let mut request = input(&repo, "scheduler-clear");
    request.variant_id = "pierre-chaos".into();
    repo.save_boss_clear(request, timestamp()).unwrap();
    repo.store_boss_observation("one", 1, "2026-10-28", state(vec![]), timestamp())
        .unwrap();
    let mut invalid = state(vec![]);
    invalid.world_name = "wrong".into();
    assert!(repo
        .store_boss_observation("one", 1, "2026-10-28", invalid, timestamp())
        .is_err());
    let restored = db.repo().boss_view(None, None, timestamp()).unwrap();
    assert_eq!(restored.scheduler.unwrap().id, cached.id);
    assert!(restored.observation.unwrap().entries.is_empty());
    assert_eq!(restored.records.len(), 1);
    assert_eq!(restored.history.len(), 1);
    assert_eq!(restored.plans.len(), 1);
    let next = db
        .repo()
        .boss_view(None, None, timestamp() + 7 * 86400)
        .unwrap();
    assert_eq!(next.scheduler.unwrap().id, cached.id);
    assert!(next.records.is_empty());
}
fn fixture() -> TestDatabase {
    let db = TestDatabase::new();
    let repo = db.repo();
    let c = selection("one", "a", "w");
    repo.save(&c, None, false, &list_of(&[c.clone()]), 0)
        .unwrap();
    repo.save_boss_plan(PlanInput {
        ocid: "one".into(),
        variant_id: "zakum-chaos".into(),
        enabled: true,
        expected_revision: 0,
        selection_revision: 1,
    })
    .unwrap();
    db
}
fn input(repo: &MainRepository, id: &str) -> ClearInput {
    ClearInput {
        ocid: "one".into(),
        week: week_start(timestamp()),
        variant_id: "zakum-chaos".into(),
        completed: true,
        party: 3,
        clear_date: Some("2026-10-28".into()),
        reason: "테스트".into(),
        expected_revision: 0,
        selection_revision: repo.load_configuration().unwrap().managed.revision,
        request_id: id.into(),
        observation_id: None,
    }
}
#[test]
fn revisions_cancel_reopen_duplicate_requests_and_independent_weeks() {
    let db = fixture();
    let repo = db.repo();
    let first = input(&repo, "first");
    assert_eq!(
        repo.save_boss_clear(first.clone(), timestamp())
            .unwrap()
            .personal_income
            .as_deref(),
        Some("1346666")
    );
    assert_eq!(
        repo.save_boss_clear(first.clone(), timestamp())
            .unwrap()
            .revision,
        1
    );
    let mut conflicting = first.clone();
    conflicting.party = 2;
    assert_eq!(
        repo.save_boss_clear(conflicting, timestamp())
            .err()
            .unwrap()
            .code,
        "boss_conflict"
    );
    let mut next = input(&repo, "edit");
    next.expected_revision = 1;
    next.party = 2;
    repo.save_boss_clear(next.clone(), timestamp()).unwrap();
    let view = db.repo().boss_view(None, None, timestamp()).unwrap();
    assert_eq!(view.total.amount, "2020000");
    assert_eq!(view.history.len(), 2);
    next.request_id = "cancel".into();
    next.expected_revision = 2;
    next.completed = false;
    repo.save_boss_clear(next.clone(), timestamp()).unwrap();
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .amount,
        "0"
    );
    next.request_id = "again".into();
    next.expected_revision = 3;
    next.completed = true;
    repo.save_boss_clear(next, timestamp()).unwrap();
    let mut earlier = input(&repo, "past");
    earlier.week -= 7;
    earlier.clear_date = Some("2026-10-21".into());
    repo.save_boss_clear(earlier, timestamp()).unwrap();
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .amount,
        "2020000"
    );
    assert_eq!(
        repo.boss_view(None, Some(first.week - 7), timestamp())
            .unwrap()
            .total
            .amount,
        "1346666"
    );
    assert_eq!(
        repo.boss_view(None, None, timestamp() + 7 * 86400)
            .unwrap()
            .total
            .amount,
        "0"
    );
}
#[test]
fn shared_difficulties_hidden_plan_and_selection_changes_preserve_history() {
    let db = fixture();
    let repo = db.repo();
    let mut req = input(&repo, "lucid");
    repo.save_boss_plan(PlanInput {
        ocid: "one".into(),
        variant_id: "lucid-easy".into(),
        enabled: true,
        expected_revision: 0,
        selection_revision: 1,
    })
    .unwrap();
    req.variant_id = "lucid-easy".into();
    repo.save_boss_clear(req.clone(), timestamp()).unwrap();
    req.variant_id = "lucid-hard".into();
    req.request_id = "hard".into();
    assert_eq!(
        repo.save_boss_clear(req.clone(), timestamp())
            .err()
            .unwrap()
            .code,
        "boss_conflict"
    );
    req.expected_revision = 1;
    repo.save_boss_clear(req, timestamp()).unwrap();
    repo.save_boss_plan(PlanInput {
        ocid: "one".into(),
        variant_id: "lucid-easy".into(),
        enabled: false,
        expected_revision: 1,
        selection_revision: 1,
    })
    .unwrap();
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .amount,
        "19900000"
    );
    let c = selection("two", "a", "w");
    let prior = selection("one", "a", "w");
    repo.save(
        &c,
        Some(&prior.identity),
        false,
        &list_of(&[c.clone(), prior.clone()]),
        1,
    )
    .unwrap();
    let view = repo.boss_view(Some("one"), None, timestamp()).unwrap();
    assert!(!view.managed);
    assert_eq!(view.total.amount, "0");
    assert_eq!(view.character_total.amount, "19900000");
    assert_eq!(view.records.len(), 1);
    assert_eq!(view.history.len(), 2);
    let back = selection("one", "a", "w");
    repo.save(
        &back,
        Some(&c.identity),
        false,
        &list_of(&[back.clone(), c.clone()]),
        2,
    )
    .unwrap();
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .amount,
        "19900000"
    );
}
#[test]
fn legacy_manual_snapshot_and_transaction_rollback() {
    let db = fixture();
    let repo = db.repo();
    let mut req = input(&repo, "unknown");
    req.variant_id = "limbo-extreme".into();
    repo.save_boss_plan(PlanInput {
        ocid: "one".into(),
        variant_id: req.variant_id.clone(),
        enabled: true,
        expected_revision: 0,
        selection_revision: 1,
    })
    .unwrap();
    repo.save_boss_clear(req.clone(), timestamp()).unwrap();
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .unpriced,
        1
    );
    seed_legacy_price(
        &repo,
        "one",
        req.week,
        &req.variant_id,
        7500000000,
        "manual:legacy",
    );
    req.expected_revision = 1;
    req.party = 2;
    req.request_id = "retain".into();
    assert_eq!(
        repo.save_boss_clear(req.clone(), timestamp())
            .unwrap()
            .solo_price
            .as_deref(),
        Some("7500000000")
    );
    repo.open().unwrap().execute_batch("CREATE TRIGGER fail_revision BEFORE INSERT ON boss_clear_revisions BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    req.expected_revision = 2;
    req.request_id = "fail".into();
    assert!(repo.save_boss_clear(req, timestamp()).is_err());
    let view = db.repo().boss_view(None, None, timestamp()).unwrap();
    assert_eq!(view.records[0].revision, 2);
    assert_eq!(view.history.len(), 2);
    assert_eq!(view.total.amount, "3750000000");
}

#[test]
fn v2_migration_preserves_configuration_and_rolls_back_on_collision() {
    for collision in [false, true] {
        let db = TestDatabase::new();
        let c = Connection::open(&db.0).unwrap();
        c.execute_batch(include_str!("../../../migrations/001_main_character.sql"))
            .unwrap();
        c.execute_batch("INSERT INTO main_character(singleton,ocid,account_id,world_name)VALUES(1,'one','a','w');").unwrap();
        c.execute_batch(include_str!(
            "../../../migrations/002_managed_characters.sql"
        ))
        .unwrap();
        if collision {
            c.execute_batch("CREATE TABLE boss_periods(existing TEXT);")
                .unwrap();
        }
        drop(c);
        let result = db.repo().load_configuration();
        let c = Connection::open(&db.0).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            if collision { 2 } else { 4 }
        );
        assert_eq!(
            c.query_row("SELECT ocid FROM main_character", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "one"
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM managed_character", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        if collision {
            assert!(result.is_err());
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name='record_characters'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        } else {
            assert!(result.is_ok());
            assert_eq!(
                c.query_row("SELECT count(*) FROM record_characters", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                c.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
                0
            );
        }
    }
}
#[test]
fn bad_inputs_stale_selection_and_overflow_leave_totals_unchanged() {
    let db = fixture();
    let repo = db.repo();
    let mut req = input(&repo, "bad");
    for party in [0, -1, i64::MAX] {
        req.party = party;
        assert!(repo.save_boss_clear(req.clone(), timestamp()).is_err());
    }
    req.party = 1;
    req.selection_revision = 0;
    assert_eq!(
        repo.save_boss_clear(req.clone(), timestamp())
            .err()
            .unwrap()
            .code,
        "selection_changed"
    );
    req.selection_revision = 1;
    repo.save_boss_clear(req.clone(), timestamp()).unwrap();
    seed_legacy_price(
        &repo,
        "one",
        req.week,
        &req.variant_id,
        i64::MAX,
        "manual:legacy",
    );
    repo.save_boss_plan(PlanInput {
        ocid: "one".into(),
        variant_id: "pierre-chaos".into(),
        enabled: true,
        expected_revision: 0,
        selection_revision: 1,
    })
    .unwrap();
    req.request_id = "overflow".into();
    req.variant_id = "pierre-chaos".into();
    assert_eq!(
        repo.save_boss_clear(req, timestamp()).err().unwrap().code,
        "boss_amount_overflow"
    );
    let view = repo.boss_view(None, None, timestamp()).unwrap();
    assert_eq!(view.records.len(), 1);
    assert_eq!(view.total.amount, i64::MAX.to_string());
}

#[test]
fn per_character_plans_and_stored_official_price_survive_catalog_changes() {
    let db = fixture();
    let repo = db.repo();
    let one = selection("one", "a", "w");
    let two = selection("two", "a", "w");
    repo.save_managed(
        &["one".into(), "two".into()],
        &one.identity,
        1,
        Some(&list_of(&[one.clone(), two])),
    )
    .unwrap();
    assert!(repo
        .boss_view(Some("two"), None, timestamp())
        .unwrap()
        .plans
        .is_empty());
    let req = input(&repo, "old-price");
    repo.save_boss_clear(req, timestamp()).unwrap();
    // Fixture represents a persisted earlier price version, distinct from today's catalog.
    let c = repo.open().unwrap();
    let mut snapshot = repo
        .boss_view(Some("one"), None, timestamp())
        .unwrap()
        .records
        .remove(0);
    snapshot.solo_price = Some("9000001".into());
    snapshot.personal_income = Some("3000000".into());
    snapshot.price_version = "historical-version".into();
    c.execute(
        "UPDATE boss_clear_revisions SET snapshot_json=?1",
        [serde_json::to_string(&snapshot).unwrap()],
    )
    .unwrap();
    drop(c);
    let mut edit = input(&repo, "old-price-edit");
    edit.expected_revision = 1;
    edit.party = 2;
    let saved = repo.save_boss_clear(edit, timestamp()).unwrap();
    assert_eq!(saved.personal_income.as_deref(), Some("4500000"));
    assert_eq!(saved.price_version, "historical-version");
}

#[test]
#[ignore = "Explicit native UI fixture preparation only; never reads user data"]
fn prepare_native_ui_fixture() {
    let root = std::path::PathBuf::from(
        std::env::var_os("MAPLE_LEDGER_CHECK_DIR").expect("explicit temp check directory"),
    );
    assert!(root
        .canonicalize()
        .unwrap()
        .starts_with(std::env::temp_dir().canonicalize().unwrap()));
    let file = root.join("ledger.sqlite3");
    assert!(!file.exists());
    let c = Connection::open(file).unwrap();
    c.execute_batch(include_str!("../../../migrations/001_main_character.sql"))
        .unwrap();
    c.execute_batch("INSERT INTO main_character(singleton,ocid,account_id,world_name,character_name,character_class,character_level) VALUES(1,'ui-one','synthetic','테스트 월드','테스트 본캐','테스트 직업',280);").unwrap();
    c.execute_batch(include_str!(
        "../../../migrations/002_managed_characters.sql"
    ))
    .unwrap();
    c.execute_batch("INSERT INTO managed_character VALUES(1,'ui-two','synthetic','테스트 월드',0,'테스트 부캐','테스트 직업',260);").unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

// Seed prior-version data only in synthetic SQLite fixtures; no production price writer remains.
fn seed_legacy_price(
    repo: &MainRepository,
    ocid: &str,
    week: i64,
    variant: &str,
    price: i64,
    version: &str,
) {
    let c = repo.open().unwrap();
    let (id,revision,json):(i64,i64,String)=c.query_row("SELECT r.id,r.current_revision,v.snapshot_json FROM boss_clear_records r JOIN boss_clear_revisions v ON v.record_id=r.id AND v.revision=r.current_revision WHERE r.ocid=?1 AND r.start_day=?2 AND r.clear_group=?3",params![ocid,week,crate::domain::bosses::variant(variant).unwrap().group()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    let mut snap: ClearSnapshot = serde_json::from_str(&json).unwrap();
    snap.solo_price = Some(price.to_string());
    snap.personal_income = Some((price / snap.party).to_string());
    snap.price_version = version.into();
    snap.source = "과거 합성 사용자 확인 가격".into();
    c.execute(
        "UPDATE boss_clear_revisions SET snapshot_json=?1 WHERE record_id=?2 AND revision=?3",
        params![serde_json::to_string(&snap).unwrap(), id, revision],
    )
    .unwrap();
}
fn observed(repo: &MainRepository, completed: bool) -> Observation {
    let config = repo.load_configuration().unwrap();
    let character = &config.managed.characters[0];
    repo.store_boss_observation(
        "one",
        config.managed.revision,
        "2026-10-28",
        crate::nexon::scheduler::SchedulerState {
            date: "2026-10-28".into(),
            character_name: character.character_name.clone().unwrap(),
            world_name: "w".into(),
            boss_contents: vec![crate::nexon::scheduler::BossContent {
                content_name: "자쿰".into(),
                difficulty: "chaos".into(),
                cycle: "bossWeekly".into(),
                registration_flag: None,
                complete_flag: completed.to_string(),
            }],
        },
        timestamp(),
    )
    .unwrap()
}
#[test]
fn legacy_settings_are_inert_and_existing_snapshots_survive() {
    let db = fixture();
    let repo = db.repo();
    let c = repo.open().unwrap();
    c.execute(
        "INSERT INTO boss_price_settings VALUES('zakum-chaos','10','legacy',1,?1)",
        [timestamp()],
    )
    .unwrap();
    let saved = repo
        .save_boss_clear(input(&repo, "official-only"), timestamp())
        .unwrap();
    assert_eq!(saved.solo_price.as_deref(), Some("4040000"));
    seed_legacy_price(
        &repo,
        "one",
        week_start(timestamp()),
        "zakum-chaos",
        10,
        "setting:zakum-chaos:1",
    );
    let mut edit = input(&repo, "legacy-party-edit");
    edit.expected_revision = 1;
    edit.party = 2;
    let saved = repo.save_boss_clear(edit.clone(), timestamp()).unwrap();
    assert_eq!(saved.personal_income.as_deref(), Some("5"));
    assert_eq!(saved.price_version, "setting:zakum-chaos:1");
    let view = db.repo().boss_view(None, None, timestamp()).unwrap();
    assert_eq!(view.history.len(), 2);
    assert_eq!(view.total.amount, "5");
    let mut fresh = input(&repo, "new-week");
    fresh.week += 7;
    fresh.clear_date = Some("2026-11-04".into());
    assert_eq!(
        repo.save_boss_clear(fresh, timestamp() + 7 * 86400)
            .unwrap()
            .solo_price
            .as_deref(),
        Some("4040000")
    );
    assert_eq!(
        c.query_row(
            "SELECT price FROM boss_price_settings WHERE variant_id='zakum-chaos'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "10"
    );
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    let mut payload = serde_json::to_value(edit).unwrap();
    payload["manual_price"] = serde_json::json!("123");
    assert!(serde_json::from_value::<ClearInput>(payload).is_err());
}

#[test]
fn api_acceptance_is_atomic_idempotent_and_never_overwrites_manual() {
    let db = fixture();
    let repo = db.repo();
    let observation = observed(&repo, true);
    let mut request = input(&repo, "api-first");
    request.observation_id = Some(observation.id);
    let saved = repo.save_boss_clear(request.clone(), timestamp()).unwrap();
    assert_eq!(saved.personal_income.as_deref(), Some("1346666"));
    assert_eq!(
        repo.save_boss_clear(request.clone(), timestamp())
            .unwrap()
            .revision,
        1
    );
    let next = observed(&repo, true);
    request.request_id = "api-repeat".into();
    request.observation_id = Some(next.id);
    assert_eq!(
        repo.save_boss_clear(request, timestamp())
            .err()
            .unwrap()
            .code,
        "boss_api_conflict"
    );
    let mut cancel = input(&repo, "cancel-api");
    cancel.completed = false;
    cancel.expected_revision = 1;
    repo.save_boss_clear(cancel, timestamp()).unwrap();
    observed(&repo, false);
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .amount,
        "0"
    );
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .history
            .len(),
        2
    );
    let db = fixture();
    let repo = db.repo();
    repo.save_boss_clear(input(&repo, "manual"), timestamp())
        .unwrap();
    let o = observed(&repo, true);
    let mut accept = input(&repo, "must-not-overwrite");
    accept.observation_id = Some(o.id);
    assert_eq!(
        repo.save_boss_clear(accept, timestamp())
            .err()
            .unwrap()
            .code,
        "boss_api_conflict"
    );
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .history
            .len(),
        1
    );
}
#[test]
fn api_invalid_stale_missing_and_rollback_protect_records() {
    let db = fixture();
    let repo = db.repo();
    let o = observed(&repo, false);
    let mut request = input(&repo, "not-completed");
    request.observation_id = Some(o.id);
    assert!(repo.save_boss_clear(request.clone(), timestamp()).is_err());
    request.observation_id = Some(observed(&repo, true).id);
    request.clear_date = None;
    assert!(repo.save_boss_clear(request.clone(), timestamp()).is_err());
    request.clear_date = Some("2026-10-28".into());
    assert!(repo
        .save_boss_clear(request.clone(), timestamp() + 86400)
        .is_err());
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .history
            .len(),
        0
    );
    let c = repo.open().unwrap();
    c.execute_batch("CREATE TRIGGER reject_acceptance BEFORE INSERT ON boss_api_acceptances BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    assert!(repo.save_boss_clear(request, timestamp()).is_err());
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .history
            .len(),
        0
    );
    let last = repo
        .boss_view(None, None, timestamp())
        .unwrap()
        .observation
        .unwrap();
    assert!(last.entries[0].completed);
}
#[test]
fn v3_migration_preserves_revisions_and_failure_is_atomic() {
    for collision in [false, true] {
        let db = fixture();
        let repo = db.repo();
        repo.save_boss_clear(input(&repo, "before-migration"), timestamp())
            .unwrap();
        let c = repo.open().unwrap();
        c.execute_batch("DROP TABLE boss_api_acceptances; DROP TABLE boss_api_observations; DROP TABLE boss_price_settings; PRAGMA user_version=3;").unwrap();
        if collision {
            c.execute_batch("CREATE TABLE boss_price_settings(dummy TEXT);")
                .unwrap();
        }
        let result = repo.boss_view(None, None, timestamp());
        if collision {
            assert!(result.is_err());
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                3
            );
        } else {
            let view = result.unwrap();
            assert_eq!(view.total.amount, "1346666");
            assert_eq!(view.history.len(), 1);
            assert_eq!(view.characters.len(), 1);
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                4
            );
        }
        assert_eq!(
            c.query_row("SELECT count(*) FROM boss_clear_revisions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM managed_character", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
#[ignore = "Explicit isolated 7-D native UI fixture only"]
fn prepare_stage7d_native_ui_fixture() {
    prepare_native_ui_fixture();
    let root = std::path::PathBuf::from(std::env::var_os("MAPLE_LEDGER_CHECK_DIR").unwrap());
    let repo = MainRepository::new(root.join("ledger.sqlite3"));
    let config = repo.load_configuration().unwrap();
    for variant in ["zakum-chaos", "pierre-chaos", "limbo-extreme"] {
        repo.save_boss_plan(PlanInput {
            ocid: "ui-one".into(),
            variant_id: variant.into(),
            enabled: true,
            expected_revision: 0,
            selection_revision: config.managed.revision,
        })
        .unwrap();
    }
    let day = date(kst_day(now()));
    repo.save_boss_clear(
        ClearInput {
            ocid: "ui-one".into(),
            week: week_start(now()),
            variant_id: "zakum-chaos".into(),
            completed: true,
            party: 3,
            clear_date: Some(day.clone()),
            reason: "legacy fixture".into(),
            expected_revision: 0,
            selection_revision: config.managed.revision,
            request_id: "legacy-ui".into(),
            observation_id: None,
        },
        now(),
    )
    .unwrap();
    seed_legacy_price(
        &repo,
        "ui-one",
        week_start(now()),
        "zakum-chaos",
        10,
        "setting:zakum-chaos:1",
    );
    repo.save_boss_plan(PlanInput {
        ocid: "ui-one".into(),
        variant_id: "zakum-chaos".into(),
        enabled: false,
        expected_revision: 1,
        selection_revision: config.managed.revision,
    })
    .unwrap();
    repo.open().unwrap().execute("INSERT INTO boss_price_settings VALUES('pierre-chaos','999','legacy inert setting',1,?1)",[now()]).unwrap();
    repo.store_boss_observation(
        "ui-one",
        config.managed.revision,
        &day,
        crate::nexon::scheduler::SchedulerState {
            date: day.clone(),
            character_name: "테스트 본캐".into(),
            world_name: "테스트 월드".into(),
            boss_contents: vec![
                crate::nexon::scheduler::BossContent {
                    content_name: "피에르".into(),
                    difficulty: "chaos".into(),
                    cycle: "bossWeekly".into(),
                    registration_flag: Some("true".into()),
                    complete_flag: "true".into(),
                },
                crate::nexon::scheduler::BossContent {
                    content_name: "림보".into(),
                    difficulty: "extreme".into(),
                    cycle: "bossWeekly".into(),
                    registration_flag: Some("false".into()),
                    complete_flag: "true".into(),
                },
                crate::nexon::scheduler::BossContent {
                    content_name: "알 수 없는 보스".into(),
                    difficulty: "hard".into(),
                    cycle: "bossWeekly".into(),
                    registration_flag: None,
                    complete_flag: "true".into(),
                },
            ],
        },
        now(),
    )
    .unwrap();
}

#[test]
fn api_identity_mismatch_and_unknown_prices_require_confirmation() {
    let db = fixture();
    let repo = db.repo();
    let config = repo.load_configuration().unwrap();
    let state = crate::nexon::scheduler::SchedulerState {
        date: "2026-10-28".into(),
        character_name: config.managed.characters[0].character_name.clone().unwrap(),
        world_name: "wrong-world".into(),
        boss_contents: vec![],
    };
    assert_eq!(
        repo.store_boss_observation(
            "one",
            config.managed.revision,
            "2026-10-28",
            state,
            timestamp()
        )
        .err()
        .unwrap()
        .code,
        "identity_changed"
    );
    assert!(repo
        .boss_view(None, None, timestamp())
        .unwrap()
        .observation
        .is_none());
    repo.save_boss_plan(PlanInput {
        ocid: "one".into(),
        variant_id: "limbo-extreme".into(),
        enabled: true,
        expected_revision: 0,
        selection_revision: config.managed.revision,
    })
    .unwrap();
    let state = crate::nexon::scheduler::SchedulerState {
        date: "2026-10-28".into(),
        character_name: config.managed.characters[0].character_name.clone().unwrap(),
        world_name: "w".into(),
        boss_contents: vec![crate::nexon::scheduler::BossContent {
            content_name: "림보".into(),
            difficulty: "extreme".into(),
            cycle: "bossWeekly".into(),
            registration_flag: None,
            complete_flag: "true".into(),
        }],
    };
    let o = repo
        .store_boss_observation(
            "one",
            config.managed.revision,
            "2026-10-28",
            state,
            timestamp(),
        )
        .unwrap();
    let mut request = input(&repo, "unknown-price-api");
    request.variant_id = "limbo-extreme".into();
    request.observation_id = Some(o.id);
    assert_eq!(
        repo.save_boss_clear(request.clone(), timestamp())
            .err()
            .unwrap()
            .code,
        "boss_api_price_required"
    );
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .history
            .len(),
        0
    );
    let c = repo.open().unwrap();
    c.execute(
        "INSERT INTO boss_price_settings VALUES('limbo-extreme','11','legacy',1,?1)",
        [timestamp()],
    )
    .unwrap();
    assert_eq!(
        repo.save_boss_clear(request.clone(), timestamp())
            .err()
            .unwrap()
            .code,
        "boss_api_price_required"
    );
    let mut manual = request.clone();
    manual.observation_id = None;
    manual.request_id = "unknown-manual-clear".into();
    assert!(repo
        .save_boss_clear(manual, timestamp())
        .unwrap()
        .personal_income
        .is_none());
    // A changed selection revision invalidates a pending acceptance.
    let c = repo.open().unwrap();
    c.execute("UPDATE selection_state SET revision=revision+1", [])
        .unwrap();
    assert_eq!(
        repo.save_boss_clear(request, timestamp())
            .err()
            .unwrap()
            .code,
        "selection_changed"
    );
    assert_eq!(
        repo.boss_view(None, None, timestamp())
            .unwrap()
            .total
            .amount,
        "0"
    );
}

#[test]
fn old_request_json_and_registration_unknown_survive_without_migration() {
    let db = fixture();
    let repo = db.repo();
    let req = input(&repo, "legacy-request");
    repo.save_boss_clear(req.clone(), timestamp()).unwrap();
    let c = repo.open().unwrap();
    let mut json = serde_json::to_value(&req).unwrap();
    json["manual_price"] = serde_json::Value::Null;
    c.execute(
        "UPDATE boss_clear_revisions SET request_json=?1 WHERE request_id=?2",
        params![json.to_string(), req.request_id],
    )
    .unwrap();
    assert_eq!(repo.save_boss_clear(req, timestamp()).unwrap().revision, 1);
    let before = repo.boss_view(None, None, timestamp()).unwrap();
    let character = &repo.load_configuration().unwrap().managed.characters[0];
    let state = crate::nexon::scheduler::SchedulerState {
        date: "2026-10-28".into(),
        character_name: character.character_name.clone().unwrap(),
        world_name: "w".into(),
        boss_contents: vec![],
    };
    repo.store_boss_observation("one", 1, "2026-10-28", state, timestamp())
        .unwrap();
    let after = repo.boss_view(None, None, timestamp()).unwrap();
    assert_eq!(after.plans.len(), before.plans.len());
    assert_eq!(after.records.len(), before.records.len());
    assert_eq!(after.total.amount, before.total.amount);
    assert_eq!(after.history.len(), before.history.len());
    assert!(after.observation.unwrap().entries.is_empty());
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
}
