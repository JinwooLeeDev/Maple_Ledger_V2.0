use serde::{Deserialize, Serialize};

use crate::nexon::{ApiError, Character, CharacterList};
pub mod bosses;
pub mod managed;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MainIdentity {
    pub account_id: String,
    pub ocid: String,
    pub world_name: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct MainCharacter {
    pub identity: MainIdentity,
    pub character_name: Option<String>,
    pub character_class: Option<String>,
    pub character_level: Option<u32>,
}

impl MainCharacter {
    pub fn from_character(account_id: &str, character: &Character) -> Self {
        Self {
            identity: MainIdentity {
                account_id: account_id.into(),
                ocid: character.ocid.clone(),
                world_name: character.world_name.clone(),
            },
            character_name: character.character_name.clone(),
            character_class: character.character_class.clone(),
            character_level: character.character_level,
        }
    }
}

#[derive(Serialize)]
pub struct MainView {
    pub main: Option<MainCharacter>,
    pub status: &'static str,
    pub candidates: Vec<Character>,
    pub snapshot_version: Option<u64>,
    #[serde(flatten)]
    pub managed: ManagedView,
}

#[derive(Default, Serialize)]
pub struct ManagedView {
    pub selection_revision: i64,
    pub managed_characters: Vec<managed::ManagedEntryView>,
}

pub fn attach_managed(
    mut view: MainView,
    selection: &managed::ManagedSelection,
    list: Option<&CharacterList>,
) -> MainView {
    view.managed = ManagedView {
        selection_revision: selection.revision,
        managed_characters: managed::managed_view(selection, list),
    };
    view
}

pub fn candidate(list: &CharacterList, ocid: &str) -> Result<MainCharacter, ApiError> {
    list.account_list
        .iter()
        .find_map(|account| {
            account
                .character_list
                .iter()
                .find(|character| character.ocid == ocid)
                .map(|character| MainCharacter::from_character(&account.account_id, character))
        })
        .ok_or_else(|| ApiError::new("candidate_missing"))
}

pub fn scope_changed(old: &MainIdentity, new: &MainIdentity) -> bool {
    old.account_id != new.account_id || old.world_name != new.world_name
}

pub fn main_view(
    main: Option<MainCharacter>,
    list: Option<&CharacterList>,
    version: Option<u64>,
) -> MainView {
    let Some(saved) = main.as_ref() else {
        return MainView {
            main: None,
            status: "unselected",
            candidates: vec![],
            snapshot_version: version,
            managed: ManagedView::default(),
        };
    };
    let Some(list) = list else {
        return MainView {
            main,
            status: "no_data",
            candidates: vec![],
            snapshot_version: None,
            managed: ManagedView::default(),
        };
    };
    let Ok(current) = candidate(list, &saved.identity.ocid) else {
        return MainView {
            main,
            status: "missing",
            candidates: vec![],
            snapshot_version: version,
            managed: ManagedView::default(),
        };
    };
    if current.identity != saved.identity {
        return MainView {
            main,
            status: "identity_changed",
            candidates: vec![],
            snapshot_version: version,
            managed: ManagedView::default(),
        };
    }
    let candidates = list
        .account_list
        .iter()
        .filter(|account| account.account_id == saved.identity.account_id)
        .flat_map(|account| account.character_list.iter())
        .filter(|character| character.world_name == saved.identity.world_name)
        .cloned()
        .collect();
    MainView {
        main: Some(current),
        status: "matched",
        candidates,
        snapshot_version: version,
        managed: ManagedView::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nexon::parse_character_list;

    pub fn fixture() -> CharacterList {
        parse_character_list(br#"{"account_list":[
          {"account_id":"a","character_list":[
            {"ocid":"one","world_name":"w","character_name":"same"},
            {"ocid":"two","world_name":"w","character_name":"same"},
            {"ocid":"other-world","world_name":"x"}]},
          {"account_id":"b","character_list":[{"ocid":"other-account","world_name":"w","character_name":"same"}]}]}"#).unwrap()
    }

    #[test]
    fn exact_account_world_filter_includes_main_and_uses_ocid() {
        let mut list = fixture();
        let selected = candidate(&list, "two").unwrap();
        list.account_list.reverse();
        for account in &mut list.account_list {
            account.character_list.reverse();
        }
        let view = main_view(Some(selected), Some(&list), Some(1));
        assert_eq!(view.status, "matched");
        let ids: Vec<_> = view.candidates.iter().map(|c| c.ocid.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"one") && ids.contains(&"two"));
        assert_eq!(view.main.unwrap().identity.ocid, "two");
    }

    #[test]
    fn absent_changed_and_empty_data_never_replace_saved_identity() {
        let list = fixture();
        let selected = candidate(&list, "two").unwrap();
        assert_eq!(main_view(None, Some(&list), Some(1)).status, "unselected");
        assert_eq!(
            main_view(Some(selected.clone()), None, None).status,
            "no_data"
        );
        let empty = CharacterList {
            account_list: vec![],
        };
        let view = main_view(Some(selected.clone()), Some(&empty), Some(2));
        assert_eq!(view.status, "missing");
        assert!(view.candidates.is_empty());
        assert!(view.main.unwrap() == selected);
        let mut reidentified = fixture();
        reidentified.account_list[0]
            .character_list
            .iter_mut()
            .find(|c| c.ocid == "two")
            .unwrap()
            .ocid = "new-ocid-same-name".into();
        let view = main_view(Some(selected.clone()), Some(&reidentified), Some(3));
        assert_eq!(view.status, "missing");
        assert!(view.main.unwrap().identity == selected.identity);
        let mut transferred = fixture();
        let position = transferred.account_list[0]
            .character_list
            .iter()
            .position(|c| c.ocid == "two")
            .unwrap();
        let character = transferred.account_list[0].character_list.remove(position);
        transferred.account_list[1].character_list.push(character);
        let view = main_view(Some(selected.clone()), Some(&transferred), Some(3));
        assert_eq!(view.status, "identity_changed");
        assert!(view.candidates.is_empty());
        let mut moved = fixture();
        moved.account_list[0]
            .character_list
            .iter_mut()
            .find(|c| c.ocid == "two")
            .unwrap()
            .world_name = "moved".into();
        let view = main_view(Some(selected.clone()), Some(&moved), Some(3));
        assert_eq!(view.status, "identity_changed");
        assert!(view.candidates.is_empty());
        assert!(view.main.unwrap() == selected);
        let mut renamed = fixture();
        renamed.account_list[0]
            .character_list
            .iter_mut()
            .find(|c| c.ocid == "two")
            .unwrap()
            .character_name = Some("renamed".into());
        let view = main_view(Some(selected.clone()), Some(&renamed), Some(4));
        assert_eq!(view.status, "matched");
        assert!(view.main.unwrap().identity == selected.identity);
        assert!(candidate(&list, "missing-ocid").is_err());
        assert!(!scope_changed(
            &selected.identity,
            &candidate(&list, "one").unwrap().identity
        ));
        assert!(scope_changed(
            &selected.identity,
            &candidate(&list, "other-account").unwrap().identity
        ));
        assert!(scope_changed(
            &selected.identity,
            &candidate(&list, "other-world").unwrap().identity
        ));
    }
}
