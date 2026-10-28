use super::*;

pub(super) struct TestDatabase(pub(super) PathBuf);
impl TestDatabase {
    pub(super) fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "maple-ledger-stage4-{}-{stamp}.sqlite3",
            std::process::id()
        )))
    }
    pub(super) fn repo(&self) -> MainRepository {
        MainRepository::new(self.0.clone())
    }
}
impl Drop for TestDatabase {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub(super) fn selection(ocid: &str, account: &str, world: &str) -> MainCharacter {
    MainCharacter {
        identity: MainIdentity {
            ocid: ocid.into(),
            account_id: account.into(),
            world_name: world.into(),
        },
        character_name: Some("synthetic".into()),
        character_class: None,
        character_level: Some(260),
    }
}

pub(super) fn list_of(characters: &[MainCharacter]) -> CharacterList {
    let mut accounts: Vec<crate::nexon::Account> = vec![];
    for saved in characters {
        let index = accounts
            .iter()
            .position(|a| a.account_id == saved.identity.account_id)
            .unwrap_or_else(|| {
                accounts.push(crate::nexon::Account {
                    account_id: saved.identity.account_id.clone(),
                    character_list: vec![],
                });
                accounts.len() - 1
            });
        accounts[index]
            .character_list
            .push(crate::nexon::Character {
                ocid: saved.identity.ocid.clone(),
                world_name: saved.identity.world_name.clone(),
                character_name: saved.character_name.clone(),
                character_class: saved.character_class.clone(),
                character_level: saved.character_level,
            });
    }
    CharacterList {
        account_list: accounts,
    }
}

fn save(
    repo: &MainRepository,
    next: &MainCharacter,
    expected: Option<&MainIdentity>,
    confirmed: bool,
) -> Result<(), ApiError> {
    let revision = repo.load_configuration()?.managed.revision;
    repo.save(
        next,
        expected,
        confirmed,
        &list_of(&[next.clone()]),
        revision,
    )
    .map(|_| ())
}

#[test]
fn selection_reopens_requires_confirmation_and_preserves_other_tables() {
    let database = TestDatabase::new();
    let repo = database.repo();
    assert!(repo.load().unwrap().is_none());
    let first = selection("one", "a", "w");
    save(&repo, &first, None, false).unwrap();
    drop(repo);
    let repo = database.repo();
    assert!(repo.load().unwrap().unwrap() == first);
    let connection = repo.open().unwrap();
    connection.execute_batch("CREATE TABLE unrelated_history (value TEXT); INSERT INTO unrelated_history VALUES ('keep');").unwrap();
    drop(connection);
    let moved = selection("moved", "b", "w");
    assert_eq!(
        save(&repo, &moved, Some(&first.identity), false)
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    assert!(repo.load().unwrap().unwrap() == first);
    save(&repo, &moved, Some(&first.identity), true).unwrap();
    assert_eq!(
        save(&repo, &first, Some(&first.identity), true)
            .unwrap_err()
            .code,
        "main_changed"
    );
    assert!(repo.load().unwrap().unwrap() == moved);
    let same_scope = selection("another", "b", "w");
    save(&repo, &same_scope, Some(&moved.identity), false).unwrap();
    let connection = repo.open().unwrap();
    let count: i64 = connection
        .query_row("SELECT count(*) FROM main_character", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let kept: String = connection
        .query_row("SELECT value FROM unrelated_history", [], |row| row.get(0))
        .unwrap();
    assert_eq!(kept, "keep");
}

#[test]
fn failed_write_rolls_back_and_newer_schema_is_not_modified() {
    let database = TestDatabase::new();
    let repo = database.repo();
    let first = selection("one", "a", "w");
    save(&repo, &first, None, false).unwrap();
    let connection = repo.open().unwrap();
    connection.execute_batch("CREATE TRIGGER reject_main_update BEFORE UPDATE ON main_character BEGIN SELECT RAISE(ABORT, 'synthetic-failure'); END;").unwrap();
    drop(connection);
    assert_eq!(
        save(
            &repo,
            &selection("two", "a", "w"),
            Some(&first.identity),
            false
        )
        .unwrap_err()
        .code,
        "settings_unavailable"
    );
    assert!(repo.load().unwrap().unwrap() == first);
    let connection = repo.open().unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    drop(connection);
    assert_eq!(
        repo.load().err().unwrap().code,
        "settings_version_unsupported"
    );
    let connection = Connection::open(&database.0).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        99
    );
}

#[test]
fn separate_process_restores_selection_without_any_api() {
    if let Ok(path) = std::env::var("MAPLE_LEDGER_STAGE4_TEST_DB") {
        assert!(std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("maple-ledger-stage4-"));
        assert!(
            MainRepository::new(path.into()).load().unwrap().unwrap()
                == selection("restart", "account", "world")
        );
        return;
    }
    let database = TestDatabase::new();
    save(
        &database.repo(),
        &selection("restart", "account", "world"),
        None,
        false,
    )
    .unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "storage::tests::separate_process_restores_selection_without_any_api",
        ])
        .env("MAPLE_LEDGER_STAGE4_TEST_DB", &database.0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(result.success());
}
