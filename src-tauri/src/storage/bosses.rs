mod observations;
use super::{read_configuration, storage_error, MainRepository};
use crate::{
    domain::{bosses::*, MainCharacter},
    nexon::ApiError,
};
pub use observations::*;
use observations::{read_observation, read_scheduler, validate_acceptance};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CharacterOption {
    pub character: MainCharacter,
    pub managed: bool,
}
#[derive(Serialize)]
pub struct Plan {
    pub boss_id: String,
    pub variant_id: String,
    pub enabled: bool,
    pub revision: i64,
}
#[derive(Serialize)]
pub struct Total {
    pub amount: String,
    pub unpriced: usize,
}
#[derive(Serialize)]
pub struct BossView {
    pub week: Week,
    pub selection_revision: i64,
    pub catalog: Vec<Variant>,
    pub observation: Option<Observation>,
    pub scheduler: Option<Observation>,
    pub characters: Vec<CharacterOption>,
    pub character: Option<MainCharacter>,
    pub managed: bool,
    pub plans: Vec<Plan>,
    pub records: Vec<ClearSnapshot>,
    pub history: Vec<ClearSnapshot>,
    pub character_total: Total,
    pub total: Total,
}
#[derive(Deserialize)]
pub struct PlanInput {
    pub ocid: String,
    pub variant_id: String,
    pub enabled: bool,
    pub expected_revision: i64,
    pub selection_revision: i64,
}

fn stored_character(c: &Connection, ocid: &str) -> Result<Option<MainCharacter>, ApiError> {
    let json: Option<String> = c
        .query_row(
            "SELECT character_json FROM record_characters WHERE ocid=?1",
            [ocid],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| storage_error())?;
    json.map(|j| serde_json::from_str(&j).map_err(|_| storage_error()))
        .transpose()
}
fn ensure_character(c: &Connection, character: &MainCharacter) -> Result<(), ApiError> {
    if stored_character(c, &character.identity.ocid)?
        .is_some_and(|old| old.identity != character.identity)
    {
        return Err(ApiError::new("identity_changed"));
    }
    c.execute("INSERT INTO record_characters (ocid,account_id,world_name,character_json) VALUES (?1,?2,?3,?4) ON CONFLICT(ocid) DO UPDATE SET character_json=excluded.character_json",
        params![character.identity.ocid,character.identity.account_id,character.identity.world_name,serde_json::to_string(character).map_err(|_|storage_error())?]).map_err(|_|storage_error())?;
    Ok(())
}
fn snapshots(
    c: &Connection,
    week: i64,
    ocid: Option<&str>,
    history: bool,
) -> Result<Vec<ClearSnapshot>, ApiError> {
    let sql = if history {
        "SELECT v.snapshot_json FROM boss_clear_records r JOIN boss_clear_revisions v ON v.record_id=r.id WHERE r.start_day=?1 AND (?2 IS NULL OR r.ocid=?2) ORDER BY r.id,v.revision DESC"
    } else {
        "SELECT v.snapshot_json FROM boss_clear_records r JOIN boss_clear_revisions v ON v.record_id=r.id AND v.revision=r.current_revision WHERE r.start_day=?1 AND (?2 IS NULL OR r.ocid=?2) ORDER BY r.id"
    };
    let mut q = c.prepare(sql).map_err(|_| storage_error())?;
    let rows = q
        .query_map(params![week, ocid], |r| r.get::<_, String>(0))
        .map_err(|_| storage_error())?;
    rows.map(|r| {
        serde_json::from_str(&r.map_err(|_| storage_error())?).map_err(|_| storage_error())
    })
    .collect()
}
fn total<'a>(items: impl Iterator<Item = &'a ClearSnapshot>) -> Result<Total, ApiError> {
    let mut values = Vec::new();
    let mut missing = 0;
    for r in items.filter(|r| r.completed) {
        match &r.personal_income {
            Some(v) => values.push(v.parse::<i64>().map_err(|_| storage_error())?),
            None => missing += 1,
        }
    }
    Ok(Total {
        amount: sum(values.into_iter())?,
        unpriced: missing,
    })
}

impl MainRepository {
    pub fn boss_view(
        &self,
        ocid: Option<&str>,
        week: Option<i64>,
        timestamp: i64,
    ) -> Result<BossView, ApiError> {
        let mut connection = self.open()?;
        let tx = connection.transaction().map_err(|_| storage_error())?;
        let config = read_configuration(&tx)?;
        let week = Week::new(week, timestamp)?;
        let all = snapshots(&tx, week.start_day, None, false)?;
        let total = total(all.iter().filter(|r| {
            config
                .managed
                .characters
                .iter()
                .any(|c| c.identity.ocid == r.character.identity.ocid)
        }))?;
        let mut characters: Vec<_> = config
            .managed
            .characters
            .iter()
            .map(|c| CharacterOption {
                character: c.clone(),
                managed: true,
            })
            .collect();
        {
            let mut q = tx
                .prepare("SELECT character_json FROM record_characters ORDER BY ocid")
                .map_err(|_| storage_error())?;
            let rows = q
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|_| storage_error())?;
            for row in rows {
                let c: MainCharacter = serde_json::from_str(&row.map_err(|_| storage_error())?)
                    .map_err(|_| storage_error())?;
                if !characters
                    .iter()
                    .any(|v| v.character.identity.ocid == c.identity.ocid)
                {
                    characters.push(CharacterOption {
                        character: c,
                        managed: false,
                    });
                }
            }
        }
        let character = match ocid {
            Some(id) => Some(
                characters
                    .iter()
                    .find(|v| v.character.identity.ocid == id)
                    .ok_or_else(|| ApiError::new("candidate_missing"))?
                    .character
                    .clone(),
            ),
            None => characters.first().map(|v| v.character.clone()),
        };
        let selected = character.as_ref().map(|c| c.identity.ocid.as_str());
        let managed = config
            .managed
            .characters
            .iter()
            .any(|c| Some(c.identity.ocid.as_str()) == selected);
        let mut plans = Vec::new();
        if let Some(id) = selected {
            let mut q=tx.prepare("SELECT boss_id,variant_id,enabled,revision FROM character_boss_plans WHERE ocid=?1 ORDER BY rowid").map_err(|_|storage_error())?;
            let rows = q
                .query_map([id], |r| {
                    Ok(Plan {
                        boss_id: r.get(0)?,
                        variant_id: r.get(1)?,
                        enabled: r.get(2)?,
                        revision: r.get(3)?,
                    })
                })
                .map_err(|_| storage_error())?;
            for row in rows {
                plans.push(row.map_err(|_| storage_error())?);
            }
        }
        let records: Vec<_> = all
            .into_iter()
            .filter(|r| Some(r.character.identity.ocid.as_str()) == selected)
            .collect();
        let character_total = total_for_records(&records)?;
        let history = if selected.is_some() {
            snapshots(&tx, week.start_day, selected, true)?
        } else {
            Vec::new()
        };
        let observation = selected
            .map(|id| read_observation(&tx, id, week.start_day))
            .transpose()?
            .flatten();
        let scheduler = selected
            .map(|id| read_scheduler(&tx, id, week.start_day))
            .transpose()?
            .flatten();
        tx.commit().map_err(|_| storage_error())?;
        Ok(BossView {
            week,
            selection_revision: config.managed.revision,
            catalog: catalog(),
            observation,
            scheduler,
            characters,
            character,
            managed,
            plans,
            records,
            history,
            character_total,
            total,
        })
    }

    pub fn save_boss_plan(&self, input: PlanInput) -> Result<(), ApiError> {
        let v = variant(&input.variant_id)?;
        let mut connection = self.open()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| storage_error())?;
        let config = read_configuration(&tx)?;
        if config.managed.revision != input.selection_revision {
            return Err(ApiError::new("selection_changed"));
        }
        let c = config
            .managed
            .characters
            .iter()
            .find(|c| c.identity.ocid == input.ocid)
            .ok_or_else(|| ApiError::new("boss_not_managed"))?;
        ensure_character(&tx, c)?;
        let revision: i64 = tx
            .query_row(
                "SELECT revision FROM character_boss_plans WHERE ocid=?1 AND boss_id=?2",
                params![input.ocid, v.boss_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| storage_error())?
            .unwrap_or(0);
        if revision != input.expected_revision {
            return Err(ApiError::new("boss_conflict"));
        }
        tx.execute("INSERT INTO character_boss_plans(ocid,boss_id,variant_id,enabled,revision) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(ocid,boss_id) DO UPDATE SET variant_id=excluded.variant_id,enabled=excluded.enabled,revision=excluded.revision",
            params![input.ocid,v.boss_id,v.id,input.enabled,revision+1]).map_err(|_|storage_error())?;
        tx.commit().map_err(|_| storage_error())
    }

    pub fn save_boss_clear(
        &self,
        input: ClearInput,
        timestamp: i64,
    ) -> Result<ClearSnapshot, ApiError> {
        let v = variant(&input.variant_id)?;
        let week = Week::new(Some(input.week), timestamp)?;
        if input.request_id.is_empty()
            || input.request_id.len() > 128
            || input.reason.chars().count() > 300
        {
            return Err(ApiError::new("boss_input_invalid"));
        }
        let clear_day = input.clear_date.as_deref().map(parse_day).transpose()?;
        if clear_day.is_some_and(|d| {
            d < week.start_day || d >= week.start_day + 7 || d > kst_day(timestamp)
        }) {
            return Err(ApiError::new("boss_date_invalid"));
        }
        income(None, input.party, v.party_max)?;
        let mut connection = self.open()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| storage_error())?;
        let config = read_configuration(&tx)?;
        if config.managed.revision != input.selection_revision {
            return Err(ApiError::new("selection_changed"));
        }
        if input.observation_id.is_some() {
            validate_acceptance(&tx, &input, &week, &config, timestamp)?;
        }
        let request_json = serde_json::to_string(&input).map_err(|_| storage_error())?;
        let duplicate: Option<(String, String)> = tx
            .query_row(
                "SELECT request_json,snapshot_json FROM boss_clear_revisions WHERE request_id=?1",
                [&input.request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| storage_error())?;
        if let Some((prior, snapshot)) = duplicate {
            // Compare historical requests without mutating their stored audit JSON.
            let mut prior: serde_json::Value =
                serde_json::from_str(&prior).map_err(|_| storage_error())?;
            if let Some(fields) = prior.as_object_mut() {
                if fields.get("manual_price") == Some(&serde_json::Value::Null) {
                    fields.remove("manual_price");
                }
                fields
                    .entry("observation_id")
                    .or_insert(serde_json::Value::Null);
            }
            if prior != serde_json::to_value(&input).map_err(|_| storage_error())? {
                return Err(ApiError::new("boss_conflict"));
            }
            return serde_json::from_str(&snapshot).map_err(|_| storage_error());
        }
        let old = snapshots(&tx, week.start_day, Some(&input.ocid), false)?
            .into_iter()
            .find(|r| r.clear_group == v.group());
        let old_revision = old.as_ref().map(|r| r.revision).unwrap_or(0);
        if input.observation_id.is_some() && old.is_some() {
            return Err(ApiError::new("boss_api_conflict"));
        }
        if old_revision != input.expected_revision {
            return Err(ApiError::new("boss_conflict"));
        }
        let character = if let Some(c) = config
            .managed
            .characters
            .iter()
            .find(|c| c.identity.ocid == input.ocid)
        {
            ensure_character(&tx, c)?;
            c.clone()
        } else if let Some(r) = &old {
            r.character.clone()
        } else {
            return Err(ApiError::new("boss_not_managed"));
        };
        let plan_enabled: bool = tx
            .query_row(
                "SELECT enabled FROM character_boss_plans WHERE ocid=?1 AND boss_id=?2",
                params![input.ocid, v.boss_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| storage_error())?
            .unwrap_or(false);
        let registered = read_scheduler(&tx, &input.ocid, week.start_day)?.is_some_and(|o| {
            o.entries
                .iter()
                .any(|b| b.variant_id.as_deref() == Some(input.variant_id.as_str()))
        });
        if old.is_none() && (!input.completed || (!plan_enabled && !registered)) {
            return Err(ApiError::new("boss_plan_required"));
        }
        // Legacy user-confirmed prices survive only inside their original snapshots.
        let retained = old
            .as_ref()
            .filter(|r| r.variant_id == v.id && r.clear_date == input.clear_date);
        let price = if let Some(r) = retained {
            r.solo_price
                .as_ref()
                .map(|p| p.parse::<i64>().map_err(|_| storage_error()))
                .transpose()?
        } else {
            v.price(&week, clear_day)
        };
        let (source, version) = if let Some(r) = retained {
            (r.source.clone(), r.price_version.clone())
        } else {
            (
                v.source
                    .clone()
                    .unwrap_or_else(|| "공식 가격 미확인".into()),
                format!("{}:{}", DATA_VERSION, v.id),
            )
        };
        let personal = income(price, input.party, v.party_max)?;
        if input.observation_id.is_some() && price.is_none() {
            return Err(ApiError::new("boss_api_price_required"));
        }
        let snapshot = ClearSnapshot {
            observation_id: input
                .observation_id
                .or_else(|| old.as_ref().and_then(|r| r.observation_id)),
            clear_group: v.group(),
            variant_id: v.id.clone(),
            boss_id: v.boss_id.clone(),
            name: v.name.clone(),
            difficulty: v.difficulty.clone(),
            completed: input.completed,
            party: input.party,
            clear_date: input.clear_date.clone(),
            solo_price: price.map(|p| p.to_string()),
            personal_income: personal.map(|p| p.to_string()),
            source,
            price_version: version,
            calculation_version: CALC_VERSION.into(),
            character,
            recorded_at: timestamp,
            reason: input.reason.clone(),
            revision: old_revision + 1,
        };
        // Validate both character and managed aggregate BEFORE committing an amount.
        let mut all = snapshots(&tx, week.start_day, None, false)?;
        all.retain(|r| !(r.character.identity.ocid == input.ocid && r.clear_group == v.group()));
        all.push(snapshot.clone());
        total(
            all.iter()
                .filter(|r| r.character.identity.ocid == input.ocid),
        )?;
        total(all.iter().filter(|r| {
            config
                .managed
                .characters
                .iter()
                .any(|c| c.identity.ocid == r.character.identity.ocid)
        }))?;
        tx.execute(
            "INSERT OR IGNORE INTO boss_periods VALUES(?1,?2,'Asia/Seoul',?3)",
            params![week.start_day, week.start_day + 7, RESET_VERSION],
        )
        .map_err(|_| storage_error())?;
        tx.execute("INSERT INTO boss_clear_records(ocid,start_day,clear_group,current_revision) VALUES(?1,?2,?3,?4) ON CONFLICT(ocid,start_day,clear_group) DO UPDATE SET current_revision=excluded.current_revision",params![input.ocid,week.start_day,v.group(),snapshot.revision]).map_err(|_|storage_error())?;
        let record_id:i64=tx.query_row("SELECT id FROM boss_clear_records WHERE ocid=?1 AND start_day=?2 AND clear_group=?3",params![input.ocid,week.start_day,v.group()],|r|r.get(0)).map_err(|_|storage_error())?;
        tx.execute(
            "INSERT INTO boss_clear_revisions VALUES(?1,?2,?3,?4,?5)",
            params![
                record_id,
                snapshot.revision,
                input.request_id,
                request_json,
                serde_json::to_string(&snapshot).map_err(|_| storage_error())?
            ],
        )
        .map_err(|_| storage_error())?;
        if let Some(id) = input.observation_id {
            tx.execute(
                "INSERT INTO boss_api_acceptances VALUES(?1,?2,?3,?4,?5)",
                params![input.ocid, week.start_day, v.group(), id, input.request_id],
            )
            .map_err(|_| storage_error())?;
        }
        tx.commit().map_err(|_| storage_error())?;
        Ok(snapshot)
    }
}
fn total_for_records(records: &[ClearSnapshot]) -> Result<Total, ApiError> {
    total(records.iter())
}

#[cfg(test)]
mod tests;
