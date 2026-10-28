use super::*;
use crate::nexon::scheduler::{map, ObservedBoss, SchedulerState};

#[derive(Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: i64,
    pub query_date: String,
    pub observed_at: i64,
    pub entries: Vec<ObservedBoss>,
}
pub(super) fn read_observation(
    c: &Connection,
    ocid: &str,
    week: i64,
) -> Result<Option<Observation>, ApiError> {
    let row: Option<(i64,String,i64,String)> = c.query_row("SELECT id,query_date,observed_at,entries_json FROM boss_api_observations WHERE ocid=?1 AND start_day=?2 ORDER BY id DESC LIMIT 1",params![ocid,week],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|storage_error())?;
    row.map(|(id, query_date, observed_at, json)| {
        Ok(Observation {
            id,
            query_date,
            observed_at,
            entries: serde_json::from_str(&json).map_err(|_| storage_error())?,
        })
    })
    .transpose()
}
// Registration cache is independent of the latest completion observation.
pub(super) fn read_scheduler(
    c: &Connection,
    ocid: &str,
    week: i64,
) -> Result<Option<Observation>, ApiError> {
    let mut q = c.prepare("SELECT id,query_date,observed_at,entries_json FROM boss_api_observations WHERE ocid=?1 AND start_day<=?2 ORDER BY id DESC").map_err(|_| storage_error())?;
    let rows = q
        .query_map(params![ocid, week], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| storage_error())?;
    for row in rows {
        let (id, query_date, observed_at, json) = row.map_err(|_| storage_error())?;
        let entries: Vec<ObservedBoss> =
            serde_json::from_str(&json).map_err(|_| storage_error())?;
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|b| b.cycle == "bossWeekly" && b.registered == Some(true))
            .collect();
        if !entries.is_empty() {
            return Ok(Some(Observation {
                id,
                query_date,
                observed_at,
                entries,
            }));
        }
    }
    Ok(None)
}
impl MainRepository {
    pub fn store_boss_observation(
        &self,
        ocid: &str,
        selection_revision: i64,
        query_date: &str,
        state: SchedulerState,
        timestamp: i64,
    ) -> Result<Observation, ApiError> {
        let mut c = self.open()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| storage_error())?;
        let config = read_configuration(&tx)?;
        if config.managed.revision != selection_revision {
            return Err(ApiError::new("selection_changed"));
        }
        let character = config
            .managed
            .characters
            .iter()
            .find(|c| c.identity.ocid == ocid)
            .ok_or_else(|| ApiError::new("boss_not_managed"))?;
        if character.identity.world_name != state.world_name
            || character.character_name.as_deref() != Some(state.character_name.as_str())
        {
            return Err(ApiError::new("identity_changed"));
        }
        let week = week_start(timestamp);
        if state.date != query_date || parse_day(query_date)? != kst_day(timestamp) {
            return Err(ApiError::new("boss_api_stale"));
        }
        let entries = map(&state);
        tx.execute("INSERT INTO boss_api_observations(ocid,start_day,query_date,observed_at,selection_revision,character_json,entries_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![ocid,week,query_date,timestamp,selection_revision,serde_json::to_string(character).map_err(|_|storage_error())?,serde_json::to_string(&entries).map_err(|_|storage_error())?]).map_err(|_|storage_error())?;
        let id = tx.last_insert_rowid();
        tx.commit().map_err(|_| storage_error())?;
        Ok(Observation {
            id,
            query_date: query_date.into(),
            observed_at: timestamp,
            entries,
        })
    }
}
pub(super) fn validate_acceptance(
    c: &Connection,
    input: &ClearInput,
    week: &Week,
    config: &super::super::Configuration,
    timestamp: i64,
) -> Result<(), ApiError> {
    if week.start_day != week.current_start || !input.completed || input.clear_date.is_none() {
        return Err(ApiError::new("boss_api_stale"));
    }
    let row: Option<(String,i64,String,String,i64)> = c.query_row("SELECT ocid,start_day,query_date,entries_json,selection_revision FROM boss_api_observations WHERE id=?1",[input.observation_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(|_|storage_error())?;
    let (ocid, start, query, json, selection) =
        row.ok_or_else(|| ApiError::new("boss_api_stale"))?;
    if ocid != input.ocid
        || start != week.start_day
        || selection != config.managed.revision
        || parse_day(&query)? != kst_day(timestamp)
        || input.clear_date.as_ref().is_some_and(|d| d > &query)
        || !config
            .managed
            .characters
            .iter()
            .any(|c| c.identity.ocid == ocid)
    {
        return Err(ApiError::new("boss_api_stale"));
    }
    let entries: Vec<ObservedBoss> = serde_json::from_str(&json).map_err(|_| storage_error())?;
    if !entries
        .iter()
        .any(|b| b.completed && b.variant_id.as_deref() == Some(input.variant_id.as_str()))
    {
        return Err(ApiError::new("boss_api_stale"));
    }
    let group = variant(&input.variant_id)?.group();
    let accepted: Option<String> = c.query_row("SELECT request_id FROM boss_api_acceptances WHERE ocid=?1 AND start_day=?2 AND clear_group=?3",params![ocid,start,group],|r|r.get(0)).optional().map_err(|_|storage_error())?;
    if accepted.is_some_and(|r| r != input.request_id) {
        return Err(ApiError::new("boss_api_conflict"));
    }
    Ok(())
}
