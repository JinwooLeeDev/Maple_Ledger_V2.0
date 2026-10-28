use crate::nexon::ApiError;
use serde::{Deserialize, Serialize};

pub const DATA_VERSION: &str = "kms-2026-10-28-v1";
pub const CALC_VERSION: &str = "party_floor_v1";
pub const RESET_VERSION: &str = "kms-thursday-0000-v1";
const DAY: i64 = 86400;

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
pub fn kst_day(timestamp: i64) -> i64 {
    (timestamp + 9 * 3600).div_euclid(DAY)
}
pub fn week_start(timestamp: i64) -> i64 {
    let d = kst_day(timestamp);
    d - d.rem_euclid(7)
}

// Gregorian civil date from Unix day. No locale, OS timezone or ISO-week dependency.
pub fn date(day: i64) -> String {
    let z = day + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}
pub fn parse_day(s: &str) -> Result<i64, ApiError> {
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(ApiError::new("boss_date_invalid"));
    }
    let y = parts[0]
        .parse::<i64>()
        .map_err(|_| ApiError::new("boss_date_invalid"))?;
    let m = parts[1]
        .parse::<i64>()
        .map_err(|_| ApiError::new("boss_date_invalid"))?;
    let d = parts[2]
        .parse::<i64>()
        .map_err(|_| ApiError::new("boss_date_invalid"))?;
    if !(1970..=9999).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(ApiError::new("boss_date_invalid"));
    }
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let day = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    if date(day) != s {
        return Err(ApiError::new("boss_date_invalid"));
    }
    Ok(day)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Week {
    pub start_day: i64,
    pub start_date: String,
    pub end_date: String,
    pub today: String,
    pub current_start: i64,
}
impl Week {
    pub fn new(day: Option<i64>, timestamp: i64) -> Result<Self, ApiError> {
        let current = week_start(timestamp);
        let start = day.unwrap_or(current);
        if start < 0 || start > current || start % 7 != 0 {
            return Err(ApiError::new("boss_week_invalid"));
        }
        Ok(Self {
            start_day: start,
            start_date: date(start),
            end_date: date(start + 7),
            today: date(kst_day(timestamp)),
            current_start: current,
        })
    }
}
pub fn income(price: Option<i64>, party: i64, max: Option<i64>) -> Result<Option<i64>, ApiError> {
    if party <= 0 || party > i32::MAX as i64 || max.is_some_and(|m| party > m) {
        return Err(ApiError::new("boss_party_invalid"));
    }
    match price {
        Some(p) if p <= 0 => Err(ApiError::new("boss_price_invalid")),
        Some(p) => Ok(Some(p / party)),
        None => Ok(None),
    }
}
pub fn sum(mut values: impl Iterator<Item = i64>) -> Result<String, ApiError> {
    values
        .try_fold(0i64, |a, b| {
            a.checked_add(b)
                .ok_or_else(|| ApiError::new("boss_amount_overflow"))
        })
        .map(|n| n.to_string())
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub boss_id: String,
    pub name: String,
    pub difficulty: String,
    pub solo_price: Option<String>,
    pub effective_date: Option<String>,
    pub source: Option<String>,
    pub party_max: Option<i64>,
    pub shared_limit: bool,
}
pub fn catalog() -> Vec<Variant> {
    serde_json::from_str(include_str!("../../resources/bosses.json"))
        .expect("embedded reviewed boss catalog")
}
pub fn variant(id: &str) -> Result<Variant, ApiError> {
    catalog()
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| ApiError::new("boss_unknown"))
}
impl Variant {
    pub fn group(&self) -> String {
        if self.shared_limit {
            self.boss_id.clone()
        } else {
            self.id.clone()
        }
    }
    pub fn price(&self, week: &Week, clear_day: Option<i64>) -> Option<i64> {
        let effective = parse_day(self.effective_date.as_ref()?).ok()?;
        // A maintenance date has no verified exact time. That day remains ambiguous.
        let earliest = clear_day.unwrap_or(week.start_day);
        if earliest <= effective {
            return None;
        }
        self.solo_price.as_ref()?.parse::<i64>().ok()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClearInput {
    pub ocid: String,
    pub week: i64,
    pub variant_id: String,
    pub completed: bool,
    pub party: i64,
    pub clear_date: Option<String>,
    pub reason: String,
    pub expected_revision: i64,
    pub selection_revision: i64,
    pub request_id: String,
    #[serde(default)]
    pub observation_id: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ClearSnapshot {
    #[serde(default)]
    pub observation_id: Option<i64>,
    pub clear_group: String,
    pub variant_id: String,
    pub boss_id: String,
    pub name: String,
    pub difficulty: String,
    pub completed: bool,
    pub party: i64,
    pub clear_date: Option<String>,
    pub solo_price: Option<String>,
    pub personal_income: Option<String>,
    pub source: String,
    pub price_version: String,
    pub calculation_version: String,
    pub character: super::MainCharacter,
    pub recorded_at: i64,
    pub reason: String,
    pub revision: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn division_unknown_and_invalid_inputs() {
        assert_eq!(income(Some(4040000), 1, None).unwrap(), Some(4040000));
        assert_eq!(income(Some(4040000), 3, None).unwrap(), Some(1346666));
        assert_eq!(income(None, 1, None).unwrap(), None);
        for p in [0, -1, i64::MAX] {
            assert!(income(Some(10), p, None).is_err());
        }
        assert!(income(Some(-1), 1, None).is_err());
        assert!(income(Some(0), 1, None).is_err());
        assert!(income(Some(10), 4, Some(3)).is_err());
        assert!(sum([i64::MAX, 1].into_iter()).is_err());
    }
    #[test]
    fn thursday_kst_and_calendar_validation() {
        let day = parse_day("2026-10-22").unwrap();
        let boundary = day * DAY - 9 * 3600;
        assert_eq!(week_start(boundary - 1), day - 7);
        assert_eq!(week_start(boundary), day);
        assert_eq!(week_start(boundary + 7 * DAY), day + 7);
        for s in ["2026-02-29", "2026-13-01", "2026-2-01"] {
            assert!(parse_day(s).is_err());
        }
        assert_eq!(date(parse_day("2024-02-29").unwrap()), "2024-02-29");
        assert!(Week::new(Some(day + 1), boundary).is_err());
    }
    #[test]
    fn catalog_verified_prices_and_maintenance_boundaries() {
        let all = catalog();
        assert_eq!(all.len(), 54);
        assert_eq!(all.iter().filter(|v| v.solo_price.is_some()).count(), 51);
        let week = Week::new(None, parse_day("2026-10-28").unwrap() * DAY).unwrap();
        let v = variant("kaling-extreme").unwrap();
        assert_eq!(v.price(&week, None), None);
        assert_eq!(
            v.price(&week, Some(parse_day("2026-10-23").unwrap())),
            Some(4845000000)
        );
        assert!(variant("limbo-extreme")
            .unwrap()
            .price(&week, None)
            .is_none());
        let mut ids = std::collections::HashSet::new();
        for v in all {
            assert!(ids.insert(v.id));
        }
    }
}
