use super::tests::{list_of, selection, TestDatabase};
use super::*;

#[test]
fn version_one_migration_preserves_main_history_and_is_atomic() {
    let database = TestDatabase::new();
    let connection = Connection::open(&database.0).unwrap();
    connection
        .execute_batch(include_str!("../../migrations/001_main_character.sql"))
        .unwrap();
    connection.execute_batch("INSERT INTO main_character (singleton,account_id,ocid,world_name) VALUES(1,'a','migrated-main','w'); CREATE TABLE history(value INTEGER); INSERT INTO history VALUES(42);").unwrap();
    drop(connection);
    let config = database.repo().load_configuration().unwrap();
    assert_eq!(config.main.unwrap().identity.ocid, "migrated-main");
    assert_eq!(config.managed.characters.len(), 1);
    assert_eq!(config.managed.characters[0].identity.ocid, "migrated-main");
    let connection = database.repo().open().unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        connection
            .query_row("SELECT value FROM history", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        42
    );
    drop(connection);

    let failed = TestDatabase::new();
    let connection = Connection::open(&failed.0).unwrap();
    connection
        .execute_batch(include_str!("../../migrations/001_main_character.sql"))
        .unwrap();
    connection.execute_batch("CREATE TABLE selection_state(existing INTEGER); INSERT INTO main_character (singleton,account_id,ocid,world_name) VALUES(1,'a','preserved','w');").unwrap();
    drop(connection);
    assert!(failed.repo().load_configuration().is_err());
    let connection = Connection::open(&failed.0).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='managed_character'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT ocid FROM main_character", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "preserved"
    );
}

#[test]
fn managed_save_main_transition_revision_and_failure_are_atomic() {
    let database = TestDatabase::new();
    let repo = database.repo();
    let characters: Vec<_> = (0..8)
        .map(|i| selection(&format!("c{i}"), "a", "w"))
        .chain([selection("outside", "b", "w")])
        .collect();
    let list = list_of(&characters);
    let first = &characters[0];
    let config = repo.save(first, None, false, &list, 0).unwrap();
    let requested: Vec<_> = characters[..6]
        .iter()
        .map(|c| c.identity.ocid.clone())
        .collect();
    let six = repo
        .save_managed(
            &requested,
            &first.identity,
            config.managed.revision,
            Some(&list),
        )
        .unwrap();
    assert_eq!(six.managed.characters.len(), 6);
    assert_eq!(
        repo.save_managed(
            &requested,
            &first.identity,
            config.managed.revision,
            Some(&list)
        )
        .err()
        .unwrap()
        .code,
        "selection_changed"
    );
    let connection = repo.open().unwrap();
    connection.execute_batch("CREATE TABLE history(value INTEGER); INSERT INTO history VALUES(42); CREATE TRIGGER fail_managed_insert BEFORE INSERT ON managed_character WHEN new.position=1 BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    drop(connection);
    assert!(repo
        .save(
            &characters[7],
            Some(&first.identity),
            false,
            &list,
            six.managed.revision
        )
        .is_err());
    let preserved = repo.load_configuration().unwrap();
    assert!(preserved.main.unwrap().identity == first.identity);
    assert!(preserved.managed.characters == six.managed.characters);
    assert_eq!(preserved.managed.revision, six.managed.revision);
    assert!(repo
        .save_managed(
            &requested[..5],
            &first.identity,
            six.managed.revision,
            Some(&list)
        )
        .is_err());
    assert!(repo.load_configuration().unwrap().managed.characters == six.managed.characters);
    let connection = repo.open().unwrap();
    connection
        .execute_batch("DROP TRIGGER fail_managed_insert")
        .unwrap();
    drop(connection);
    let next = repo
        .save(
            &characters[3],
            Some(&first.identity),
            false,
            &list,
            six.managed.revision,
        )
        .unwrap();
    assert!(
        next.managed
            .characters
            .iter()
            .map(|c| c.identity.ocid.as_str())
            .collect::<Vec<_>>()
            == vec!["c3", "c1", "c2", "c4", "c5"]
    );
    assert_eq!(
        repo.save(
            &characters[8],
            Some(&characters[3].identity),
            false,
            &list,
            next.managed.revision
        )
        .err()
        .unwrap()
        .code,
        "confirmation_required"
    );
    let moved = repo
        .save(
            &characters[8],
            Some(&characters[3].identity),
            true,
            &list,
            next.managed.revision,
        )
        .unwrap();
    assert_eq!(moved.managed.characters.len(), 1);
    let connection = repo.open().unwrap();
    assert_eq!(
        connection
            .query_row("SELECT value FROM history", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        42
    );
}

#[test]
fn missing_entries_survive_reopen_and_explicit_offline_removal() {
    let database = TestDatabase::new();
    let repo = database.repo();
    let main = selection("main", "a", "w");
    let extra = selection("extra", "a", "w");
    let list = list_of(&[main.clone(), extra.clone()]);
    let config = repo.save(&main, None, false, &list, 0).unwrap();
    let saved = repo
        .save_managed(
            &["main".into(), "extra".into()],
            &main.identity,
            config.managed.revision,
            Some(&list),
        )
        .unwrap();
    let missing = list_of(&[main.clone()]);
    let kept = repo
        .save_managed(
            &["main".into(), "extra".into()],
            &main.identity,
            saved.managed.revision,
            Some(&missing),
        )
        .unwrap();
    assert_eq!(kept.managed.characters.len(), 2);
    assert_eq!(
        database
            .repo()
            .load_configuration()
            .unwrap()
            .managed
            .characters
            .len(),
        2
    );
    let removed = repo
        .save_managed(
            &["main".into()],
            &main.identity,
            kept.managed.revision,
            None,
        )
        .unwrap();
    assert_eq!(removed.managed.characters.len(), 1);
    assert!(repo
        .save_managed(
            &["main".into(), "extra".into()],
            &main.identity,
            removed.managed.revision,
            None
        )
        .is_err());
}

#[test]
fn managed_order_restores_in_another_process() {
    if let Ok(path) = std::env::var("MAPLE_LEDGER_STAGE5_TEST_DB") {
        assert!(std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("maple-ledger-stage4-"));
        let config = MainRepository::new(path.into())
            .load_configuration()
            .unwrap();
        assert!(
            config
                .managed
                .characters
                .iter()
                .map(|c| c.identity.ocid.as_str())
                .collect::<Vec<_>>()
                == vec!["main", "two", "one"]
        );
        return;
    }
    let database = TestDatabase::new();
    let repo = database.repo();
    let main = selection("main", "a", "w");
    let list = list_of(&[
        main.clone(),
        selection("one", "a", "w"),
        selection("two", "a", "w"),
    ]);
    let config = repo.save(&main, None, false, &list, 0).unwrap();
    repo.save_managed(
        &["main".into(), "two".into(), "one".into()],
        &main.identity,
        config.managed.revision,
        Some(&list),
    )
    .unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "storage::managed_tests::managed_order_restores_in_another_process",
        ])
        .env("MAPLE_LEDGER_STAGE5_TEST_DB", &database.0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(result.success());
}
