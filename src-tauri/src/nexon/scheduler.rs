use super::{ApiError, NexonClient};
use crate::domain::bosses::{catalog, parse_day};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SchedulerState {
    pub date: String,
    pub character_name: String,
    pub world_name: String,
    pub boss_contents: Vec<BossContent>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct BossContent {
    pub content_name: String,
    pub difficulty: String,
    pub cycle: String,
    pub registration_flag: Option<String>,
    pub complete_flag: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct ObservedBoss {
    #[serde(default)]
    pub registered: Option<bool>,
    pub content_name: String,
    pub difficulty: String,
    pub cycle: String,
    pub completed: bool,
    pub variant_id: Option<String>,
}
pub fn parse(body: &[u8]) -> Result<SchedulerState, ApiError> {
    let mut state: SchedulerState =
        serde_json::from_slice(body).map_err(|_| ApiError::new("response_parse"))?;
    if let Some(day) = state.date.strip_suffix("T00:00+09:00") {
        state.date = day.to_owned();
    }
    parse_day(&state.date).map_err(|_| ApiError::new("response_parse"))?;
    if state.character_name.trim().is_empty()
        || state.world_name.trim().is_empty()
        || state.boss_contents.iter().any(|b| {
            !matches!(b.complete_flag.as_str(), "true" | "false")
                || b.registration_flag
                    .as_deref()
                    .is_some_and(|flag| !matches!(flag, "true" | "false"))
                || b.content_name.is_empty()
                || b.difficulty.is_empty()
        })
    {
        return Err(ApiError::new("response_parse"));
    }
    Ok(state)
}
fn difficulty(label: &str) -> Option<&'static str> {
    match label {
        "easy" => Some("이지"),
        "normal" => Some("노멀"),
        "hard" => Some("하드"),
        "chaos" => Some("카오스"),
        "extreme" => Some("익스트림"),
        _ => None,
    }
}
pub fn map(state: &SchedulerState) -> Vec<ObservedBoss> {
    let catalog = catalog();
    state
        .boss_contents
        .iter()
        .map(|b| {
            // Exact labels only; unknown spelling/cycle stays visible and cannot be accepted.
            let name = if b.content_name == "듄켈" {
                "친위대장 듄켈"
            } else {
                &b.content_name
            };
            let candidates: Vec<_> = catalog
                .iter()
                .filter(|v| {
                    v.name == name
                        && Some(v.difficulty.as_str()) == difficulty(&b.difficulty)
                        && b.cycle == "bossWeekly"
                })
                .collect();
            let unique = state
                .boss_contents
                .iter()
                .filter(|other| {
                    other.content_name == b.content_name && other.difficulty == b.difficulty
                })
                .count()
                == 1;
            ObservedBoss {
                registered: b.registration_flag.as_deref().map(|flag| flag == "true"),
                content_name: b.content_name.clone(),
                difficulty: b.difficulty.clone(),
                cycle: b.cycle.clone(),
                completed: b.complete_flag == "true",
                variant_id: if unique && candidates.len() == 1 {
                    Some(candidates[0].id.clone())
                } else {
                    None
                },
            }
        })
        .collect()
}
impl NexonClient {
    pub async fn scheduler(
        &self,
        key: &str,
        ocid: &str,
        _date: &str,
    ) -> Result<SchedulerState, ApiError> {
        let mut url = reqwest::Url::parse(
            "https://open.api.nexon.com/maplestory/v1/scheduler/character-state",
        )
        .map_err(|_| ApiError::new("client_unavailable"))?;
        url.query_pairs_mut().append_pair("ocid", ocid);
        parse(&self.request(url.as_str(), key).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Explicit 7-D investigation: at most three real calls; aggregate output only"]
    fn live_registration_investigation() {
        use crate::domain::bosses::{date, parse_day};
        use rusqlite::{Connection, OpenFlags};
        use std::collections::HashSet;
        let key = zeroize::Zeroizing::new(
            std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../secrets/nexon-api-key.txt"),
            )
            .expect("development key file required"),
        );
        let db = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
            .join("com.mapleledger.app/ledger.sqlite3");
        let c = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut q = c
            .prepare("SELECT ocid FROM managed_character ORDER BY position")
            .unwrap();
        let mut ids: Vec<String> = q
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let target_name = std::env::var("MAPLE_LEDGER_RESEARCH_CHARACTER")
            .expect("explicit user-identified character required");
        let target: String = c
            .query_row(
                "SELECT ocid FROM managed_character WHERE character_name=?1",
                [target_name],
                |r| r.get(0),
            )
            .expect("user-identified character must already be managed");
        ids.retain(|id| id != &target);
        ids.insert(0, target);
        ids.truncate(2);
        let expected: HashSet<&str> = [
            "스우",
            "더스크",
            "친위대장 듄켈",
            "진 힐라",
            "선택받은 세렌",
            "감시자 칼로스",
            "최초의 대적자",
            "카링",
            "찬란한 흉성",
            "림보",
            "벨로나",
            "발드릭스",
        ]
        .into_iter()
        .collect();
        tauri::async_runtime::block_on(async {
            let client = NexonClient::new().unwrap();
            let mut current = None;
            for (index, id) in ids.iter().enumerate() {
                match client.scheduler(key.trim(), id, "").await {
                    Ok(state) => {
                        let mapped = map(&state);
                        let registered: Vec<_> = mapped
                            .iter()
                            .filter(|b| b.cycle == "bossWeekly" && b.registered == Some(true))
                            .collect();
                        let names: HashSet<_> = registered
                            .iter()
                            .filter_map(|b| b.variant_id.as_ref())
                            .filter_map(|id| {
                                catalog().into_iter().find(|v| &v.id == id).map(|v| v.name)
                            })
                            .collect();
                        let catalog_ids: HashSet<_> = mapped
                            .iter()
                            .filter_map(|b| b.variant_id.as_deref())
                            .collect();
                        let missing = catalog()
                            .iter()
                            .filter(|v| !catalog_ids.contains(v.id.as_str()))
                            .count();
                        println!("current_sample={} status=200 entries={} weekly={} registration_true={} registration_false={} registration_unknown={} registered_completed={} registered_incomplete={} registered_unmapped={} catalog_missing={}",index+1,mapped.len(),mapped.iter().filter(|b|b.cycle=="bossWeekly").count(),registered.len(),mapped.iter().filter(|b|b.cycle=="bossWeekly" && b.registered==Some(false)).count(),mapped.iter().filter(|b|b.cycle=="bossWeekly" && b.registered.is_none()).count(),registered.iter().filter(|b|b.completed).count(),registered.iter().filter(|b|!b.completed).count(),registered.iter().filter(|b|b.variant_id.is_none()).count(),missing);
                        if index == 0 {
                            println!("user_boss_name_comparison expected={} observed={} missing={} unexpected={}",expected.len(),names.len(),expected.iter().filter(|name|!names.contains(**name)).count(),names.iter().filter(|name|!expected.contains(name.as_str())).count());
                            current = Some(state);
                        }
                    }
                    Err(e) => {
                        println!("current_sample={} error_code={}", index + 1, e.code);
                        if matches!(e.code, "rate_limited" | "authentication") {
                            return;
                        }
                    }
                }
            }
            if let Some(current) = current {
                let previous = date(parse_day(&current.date).unwrap() - 7);
                let mut url = reqwest::Url::parse(
                    "https://open.api.nexon.com/maplestory/v1/scheduler/character-state",
                )
                .unwrap();
                url.query_pairs_mut()
                    .append_pair("ocid", &ids[0])
                    .append_pair("date", &previous);
                match client
                    .request(url.as_str(), key.trim())
                    .await
                    .and_then(|body| parse(&body))
                {
                    Ok(old) => {
                        let registrations = |s: &SchedulerState| {
                            s.boss_contents
                                .iter()
                                .filter(|b| {
                                    b.cycle == "bossWeekly"
                                        && b.registration_flag.as_deref() == Some("true")
                                })
                                .map(|b| (b.content_name.clone(), b.difficulty.clone()))
                                .collect::<HashSet<_>>()
                        };
                        println!("previous_week status=200 requested_date_matches={} registered={} same_registered_variants_as_current={}",old.date==previous,registrations(&old).len(),registrations(&old)==registrations(&current));
                    }
                    Err(e) => println!("previous_week error_code={}", e.code),
                }
            }
        });
    }
    #[test]
    fn parsing_mapping_and_unknown_labels() {
        let mut state = parse(br#"{"date":"2026-10-28","character_name":"test","world_name":"world","boss_contents":[]}"#).unwrap();
        state.boss_contents.push(BossContent {
            content_name: "자쿰".into(),
            difficulty: "chaos".into(),
            cycle: "bossWeekly".into(),
            registration_flag: None,
            complete_flag: "true".into(),
        });
        assert_eq!(map(&state)[0].variant_id.as_deref(), Some("zakum-chaos"));
        state.boss_contents[0].difficulty = "UNKNOWN".into();
        assert!(map(&state)[0].variant_id.is_none());
        state.boss_contents[0].difficulty = "chaos".into();
        state.boss_contents[0].cycle = "bossDaily".into();
        assert!(map(&state)[0].variant_id.is_none());
        state.boss_contents[0].cycle = "bossWeekly".into();
        state.boss_contents.push(state.boss_contents[0].clone());
        assert!(map(&state).iter().all(|b| b.variant_id.is_none()));
        for body in [
            b"null".as_slice(),
            b"{}",
            br#"{"date":"2026-10-28","character_name":"t","world_name":"w","boss_contents":null}"#,
        ] {
            assert!(parse(body).is_err());
        }
    }
    #[test]
    fn observed_wire_date_flags_and_all_catalog_variants() {
        let state = parse(br#"{"date":"2026-10-28T00:00+09:00","character_name":"synthetic","world_name":"world","boss_contents":[]}"#).unwrap();
        assert_eq!(state.date, "2026-10-28");
        for v in catalog() {
            let label = match v.difficulty.as_str() {
                "이지" => "easy",
                "노멀" => "normal",
                "하드" => "hard",
                "카오스" => "chaos",
                "익스트림" => "extreme",
                _ => panic!("unmapped reviewed difficulty"),
            };
            let mut state = SchedulerState {
                date: state.date.clone(),
                character_name: "synthetic".into(),
                world_name: "world".into(),
                boss_contents: vec![BossContent {
                    content_name: if v.name == "친위대장 듄켈" {
                        "듄켈".into()
                    } else {
                        v.name.clone()
                    },
                    difficulty: label.into(),
                    cycle: "bossWeekly".into(),
                    registration_flag: None,
                    complete_flag: "false".into(),
                }],
            };
            let entry = map(&state).remove(0);
            assert_eq!(entry.variant_id, Some(v.id));
            assert!(!entry.completed);
            state.boss_contents[0].cycle = "bossMonthly".into();
            assert!(map(&state)[0].variant_id.is_none());
        }
        for body in [br#"{"date":"2026-10-28T00:00Z","character_name":"t","world_name":"w","boss_contents":[]}"#.as_slice(),br#"{"date":"2026-10-28","character_name":"t","world_name":"w","boss_contents":[{"content_name":"b","difficulty":"hard","cycle":"bossWeekly","complete_flag":"1"}]}"#] { assert!(parse(body).is_err()); }
    }
    #[test]
    fn registration_is_independent_of_completion_and_legacy_observations_are_unknown() {
        for registered in ["true", "false"] {
            for completed in ["true", "false"] {
                let body = serde_json::json!({"date":"2026-10-28","character_name":"synthetic","world_name":"synthetic","boss_contents":[{"content_name":"자쿰","difficulty":"chaos","cycle":"bossWeekly","registration_flag":registered,"complete_flag":completed}]});
                let state = parse(&serde_json::to_vec(&body).unwrap()).unwrap();
                let entry = map(&state).remove(0);
                assert_eq!(entry.registered, Some(registered == "true"));
                assert_eq!(entry.completed, completed == "true");
                let mut invalid = body.clone();
                invalid["boss_contents"][0]["registration_flag"] = serde_json::json!("unknown");
                assert!(parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
            }
        }
        let legacy: ObservedBoss=serde_json::from_str(r#"{"content_name":"자쿰","difficulty":"chaos","cycle":"bossWeekly","completed":true,"variant_id":"zakum-chaos"}"#).unwrap();
        assert!(legacy.registered.is_none());
    }
}
