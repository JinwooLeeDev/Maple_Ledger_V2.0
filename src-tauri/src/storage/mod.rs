use std::{path::PathBuf, time::Duration};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
pub mod bosses;

use crate::{
    domain::{
        managed::{reconcile_main, validate_managed, ManagedSelection},
        scope_changed, MainCharacter, MainIdentity,
    },
    nexon::{ApiError, CharacterList},
};

pub struct Configuration {
    pub main: Option<MainCharacter>,
    pub managed: ManagedSelection,
}

pub struct MainRepository {
    path: PathBuf,
}

impl MainRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn open(&self) -> Result<Connection, ApiError> {
        let parent = self.path.parent().ok_or_else(storage_error)?;
        std::fs::create_dir_all(parent).map_err(|_| storage_error())?;
        let mut connection = Connection::open(&self.path).map_err(|_| storage_error())?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|_| storage_error())?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|_| storage_error())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| storage_error())?;
        let version: u32 = transaction
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| storage_error())?;
        match version {
            0..=4 => {
                if version == 0 {
                    transaction
                        .execute_batch(include_str!("../../migrations/001_main_character.sql"))
                        .map_err(|_| storage_error())?;
                }
                if version < 2 {
                    transaction
                        .execute_batch(include_str!("../../migrations/002_managed_characters.sql"))
                        .map_err(|_| storage_error())?;
                }
                if version < 3 {
                    transaction
                        .execute_batch(include_str!("../../migrations/003_bosses.sql"))
                        .map_err(|_| storage_error())?;
                }
                if version < 4 {
                    transaction
                        .execute_batch(include_str!(
                            "../../migrations/004_boss_prices_observations.sql"
                        ))
                        .map_err(|_| storage_error())?;
                }
            }
            _ => return Err(ApiError::new("settings_version_unsupported")),
        }
        transaction.commit().map_err(|_| storage_error())?;
        Ok(connection)
    }

    #[cfg(test)]
    pub fn load(&self) -> Result<Option<MainCharacter>, ApiError> {
        read_main(&self.open()?)
    }

    pub fn load_configuration(&self) -> Result<Configuration, ApiError> {
        let mut connection = self.open()?;
        let transaction = connection.transaction().map_err(|_| storage_error())?;
        let configuration = read_configuration(&transaction)?;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(configuration)
    }

    pub fn save(
        &self,
        next: &MainCharacter,
        expected: Option<&MainIdentity>,
        confirmed: bool,
        list: &CharacterList,
        expected_revision: i64,
    ) -> Result<Configuration, ApiError> {
        let mut connection = self.open()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| storage_error())?;
        let configuration = read_configuration(&transaction)?;
        if configuration.managed.revision != expected_revision {
            return Err(ApiError::new("selection_changed"));
        }
        let previous = configuration.main;
        if previous.as_ref().map(|main| &main.identity) != expected {
            return Err(ApiError::new("main_changed"));
        }
        if previous
            .as_ref()
            .is_some_and(|main| scope_changed(&main.identity, &next.identity))
            && !confirmed
        {
            return Err(ApiError::new("confirmation_required"));
        }
        transaction.execute(
            "INSERT INTO main_character (singleton, account_id, ocid, world_name, character_name, character_class, character_level)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(singleton) DO UPDATE SET account_id=excluded.account_id, ocid=excluded.ocid,
             world_name=excluded.world_name, character_name=excluded.character_name,
             character_class=excluded.character_class, character_level=excluded.character_level",
            params![next.identity.account_id, next.identity.ocid, next.identity.world_name, next.character_name, next.character_class, next.character_level],
        ).map_err(|_| storage_error())?;
        let choices = reconcile_main(previous.as_ref(), &configuration.managed, next, list);
        write_managed(&transaction, &choices)?;
        let result = read_configuration(&transaction)?;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(result)
    }

    pub fn save_managed(
        &self,
        requested: &[String],
        expected_main: &MainIdentity,
        expected_revision: i64,
        list: Option<&CharacterList>,
    ) -> Result<Configuration, ApiError> {
        let mut connection = self.open()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| storage_error())?;
        let configuration = read_configuration(&transaction)?;
        let main = configuration
            .main
            .as_ref()
            .ok_or_else(|| ApiError::new("main_required"))?;
        if &main.identity != expected_main {
            return Err(ApiError::new("main_changed"));
        }
        if configuration.managed.revision != expected_revision {
            return Err(ApiError::new("selection_changed"));
        }
        let choices = validate_managed(main, &configuration.managed, requested, list)?;
        write_managed(&transaction, &choices)?;
        let result = read_configuration(&transaction)?;
        transaction.commit().map_err(|_| storage_error())?;
        Ok(result)
    }
}

fn storage_error() -> ApiError {
    ApiError::new("settings_unavailable")
}

fn read_main(connection: &Connection) -> Result<Option<MainCharacter>, ApiError> {
    connection.query_row(
        "SELECT account_id, ocid, world_name, character_name, character_class, character_level FROM main_character WHERE singleton=1", [],
        |row| Ok(MainCharacter {
            identity: MainIdentity { account_id: row.get(0)?, ocid: row.get(1)?, world_name: row.get(2)? },
            character_name: row.get(3)?, character_class: row.get(4)?, character_level: row.get(5)?,
        }),
    ).optional().map_err(|_| storage_error())
}

fn read_configuration(connection: &Connection) -> Result<Configuration, ApiError> {
    let main = read_main(connection)?;
    let revision = connection
        .query_row(
            "SELECT revision FROM selection_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| storage_error())?;
    let mut statement = connection.prepare("SELECT position, account_id, ocid, world_name, character_name, character_class, character_level FROM managed_character ORDER BY position").map_err(|_| storage_error())?;
    let records = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                MainCharacter {
                    identity: MainIdentity {
                        account_id: row.get(1)?,
                        ocid: row.get(2)?,
                        world_name: row.get(3)?,
                    },
                    character_name: row.get(4)?,
                    character_class: row.get(5)?,
                    character_level: row.get(6)?,
                },
            ))
        })
        .map_err(|_| storage_error())?;
    let mut characters = Vec::new();
    for record in records {
        let (position, character) = record.map_err(|_| storage_error())?;
        if position != characters.len() as i64 {
            return Err(storage_error());
        }
        characters.push(character);
    }
    match &main {
        Some(main)
            if characters.is_empty()
                || characters.len() > 6
                || characters[0].identity != main.identity
                || characters
                    .iter()
                    .any(|c| scope_changed(&main.identity, &c.identity)) =>
        {
            return Err(storage_error())
        }
        None if !characters.is_empty() => return Err(storage_error()),
        _ => (),
    }
    Ok(Configuration {
        main,
        managed: ManagedSelection {
            revision,
            characters,
        },
    })
}

fn write_managed(connection: &Connection, characters: &[MainCharacter]) -> Result<(), ApiError> {
    // Called only inside the caller's transaction. No history tables are touched.
    connection
        .execute("DELETE FROM managed_character", [])
        .map_err(|_| storage_error())?;
    for (position, character) in characters.iter().enumerate() {
        connection.execute("INSERT INTO managed_character (position, account_id, ocid, world_name, is_main, character_name, character_class, character_level) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![position as i64, character.identity.account_id, character.identity.ocid, character.identity.world_name, position == 0, character.character_name, character.character_class, character.character_level]).map_err(|_| storage_error())?;
    }
    connection
        .execute(
            "UPDATE selection_state SET revision=revision+1 WHERE singleton=1",
            [],
        )
        .map_err(|_| storage_error())?;
    Ok(())
}

#[cfg(test)]
mod managed_tests;
#[cfg(test)]
mod tests;
