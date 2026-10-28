use std::collections::HashSet;

use serde::Serialize;

use super::{candidate, scope_changed, MainCharacter};
use crate::nexon::{ApiError, CharacterList};

#[derive(Clone, Default)]
pub struct ManagedSelection {
    pub revision: i64,
    // Main first, then the user's additional-character order.
    pub characters: Vec<MainCharacter>,
}

#[derive(Serialize)]
pub struct ManagedEntryView {
    pub character: MainCharacter,
    pub is_main: bool,
    pub status: &'static str,
}

pub fn managed_view(
    selection: &ManagedSelection,
    list: Option<&CharacterList>,
) -> Vec<ManagedEntryView> {
    selection
        .characters
        .iter()
        .enumerate()
        .map(|(position, saved)| {
            let (character, status) = match list {
                None => (saved.clone(), "no_data"),
                Some(list) => match candidate(list, &saved.identity.ocid) {
                    Ok(current) if current.identity == saved.identity => (current, "matched"),
                    Ok(_) => (saved.clone(), "identity_changed"),
                    Err(_) => (saved.clone(), "missing"),
                },
            };
            ManagedEntryView {
                character,
                is_main: position == 0,
                status,
            }
        })
        .collect()
}

pub fn validate_managed(
    main: &MainCharacter,
    saved: &ManagedSelection,
    requested: &[String],
    list: Option<&CharacterList>,
) -> Result<Vec<MainCharacter>, ApiError> {
    if requested.is_empty() || requested.len() > 6 {
        return Err(ApiError::new("selection_limit"));
    }
    let unique: HashSet<_> = requested.iter().collect();
    if unique.len() != requested.len() {
        return Err(ApiError::new("selection_duplicate"));
    }
    if !requested.contains(&main.identity.ocid) {
        return Err(ApiError::new("main_required"));
    }
    let verified_main = list
        .and_then(|list| candidate(list, &main.identity.ocid).ok())
        .filter(|current| current.identity == main.identity);
    let allow_new = verified_main.is_some();
    let mut result = vec![verified_main.unwrap_or_else(|| main.clone())];
    for ocid in requested.iter().filter(|ocid| *ocid != &main.identity.ocid) {
        // An existing unavailable entry may be retained, but never auto-linked.
        let existing = saved
            .characters
            .iter()
            .find(|entry| entry.identity.ocid == *ocid);
        let character = match list.and_then(|list| candidate(list, ocid).ok()) {
            Some(current) if allow_new && !scope_changed(&main.identity, &current.identity) => {
                current
            }
            _ => existing
                .filter(|entry| !scope_changed(&main.identity, &entry.identity))
                .cloned()
                .ok_or_else(|| ApiError::new("selection_scope"))?,
        };
        result.push(character);
    }
    Ok(result)
}

pub fn reconcile_main(
    previous: Option<&MainCharacter>,
    saved: &ManagedSelection,
    next: &MainCharacter,
    list: &CharacterList,
) -> Vec<MainCharacter> {
    let mut result = vec![next.clone()];
    let Some(previous) = previous else {
        return result;
    };
    if scope_changed(&previous.identity, &next.identity) {
        return result;
    }
    for entry in saved.characters.iter().skip(1) {
        if entry.identity.ocid == next.identity.ocid
            || scope_changed(&next.identity, &entry.identity)
        {
            continue;
        }
        match candidate(list, &entry.identity.ocid) {
            Ok(current) if current.identity == entry.identity => result.push(current),
            // Missing is unknown, not proof that the saved choice became invalid.
            Err(_) => result.push(entry.clone()),
            Ok(_) if previous.identity == next.identity => result.push(entry.clone()),
            Ok(_) => continue,
        }
        if result.len() == 6 {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests;
