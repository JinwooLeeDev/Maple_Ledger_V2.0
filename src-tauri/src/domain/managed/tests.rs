use super::*;
use crate::nexon::{Account, Character};

fn validate_managed(
    main: &MainCharacter,
    saved: &ManagedSelection,
    requested: &[String],
    list: &CharacterList,
) -> Result<Vec<MainCharacter>, ApiError> {
    super::validate_managed(main, saved, requested, Some(list))
}

fn fixture() -> CharacterList {
    let mut list = CharacterList {
        account_list: vec![Account {
            account_id: "a".into(),
            character_list: (0..8)
                .map(|index| Character {
                    ocid: format!("c{index}"),
                    world_name: "w".into(),
                    character_name: Some("same".into()),
                    character_class: None,
                    character_level: Some(260),
                })
                .collect(),
        }],
    };
    list.account_list.push(Account {
        account_id: "b".into(),
        character_list: vec![Character {
            ocid: "different-account".into(),
            world_name: "w".into(),
            character_name: None,
            character_class: None,
            character_level: None,
        }],
    });
    list.account_list[0].character_list.push(Character {
        ocid: "different-world".into(),
        world_name: "x".into(),
        character_name: None,
        character_class: None,
        character_level: None,
    });
    list
}
fn ids(indices: &[usize]) -> Vec<String> {
    indices.iter().map(|i| format!("c{i}")).collect()
}

#[test]
fn enforces_main_count_uniqueness_scope_and_requested_order() {
    let list = fixture();
    let main = candidate(&list, "c0").unwrap();
    let saved = ManagedSelection {
        revision: 0,
        characters: vec![main.clone()],
    };
    for (requested, code) in [
        (vec![], "selection_limit"),
        (ids(&[1]), "main_required"),
        (ids(&[0, 1, 1]), "selection_duplicate"),
        (ids(&[0, 1, 2, 3, 4, 5, 6]), "selection_limit"),
        (
            vec!["c0".into(), "different-account".into()],
            "selection_scope",
        ),
        (
            vec!["c0".into(), "different-world".into()],
            "selection_scope",
        ),
    ] {
        assert_eq!(
            validate_managed(&main, &saved, &requested, &list)
                .err()
                .unwrap()
                .code,
            code
        );
    }
    assert_eq!(
        validate_managed(&main, &saved, &ids(&[0]), &list)
            .unwrap()
            .len(),
        1
    );
    let six = validate_managed(&main, &saved, &ids(&[5, 0, 2, 1, 4, 3]), &list).unwrap();
    assert!(
        six.iter()
            .map(|c| c.identity.ocid.clone())
            .collect::<Vec<_>>()
            == ids(&[0, 5, 2, 1, 4, 3])
    );
}

#[test]
fn main_changes_keep_extras_but_do_not_readd_old_main() {
    let list = fixture();
    let main = candidate(&list, "c0").unwrap();
    let saved = ManagedSelection {
        revision: 1,
        characters: (0..6)
            .map(|i| candidate(&list, &format!("c{i}")).unwrap())
            .collect(),
    };
    let next = candidate(&list, "c3").unwrap();
    let result = reconcile_main(Some(&main), &saved, &next, &list);
    assert!(
        result
            .iter()
            .map(|c| c.identity.ocid.clone())
            .collect::<Vec<_>>()
            == ids(&[3, 1, 2, 4, 5])
    );
    let new_unselected = candidate(&list, "c7").unwrap();
    assert_eq!(
        reconcile_main(Some(&main), &saved, &new_unselected, &list).len(),
        6
    );
    for ocid in ["different-account", "different-world"] {
        let result = reconcile_main(Some(&main), &saved, &candidate(&list, ocid).unwrap(), &list);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].identity.ocid, ocid);
    }
}

#[test]
fn unavailable_choices_survive_refresh_and_are_never_relinked() {
    let list = fixture();
    let main = candidate(&list, "c0").unwrap();
    let saved = ManagedSelection {
        revision: 1,
        characters: vec![
            main.clone(),
            candidate(&list, "c1").unwrap(),
            candidate(&list, "c2").unwrap(),
        ],
    };
    let mut changed = fixture();
    changed.account_list.reverse();
    let account = changed
        .account_list
        .iter_mut()
        .find(|a| a.account_id == "a")
        .unwrap();
    account.character_list.retain(|c| c.ocid != "c1");
    account
        .character_list
        .iter_mut()
        .find(|c| c.ocid == "c2")
        .unwrap()
        .world_name = "moved".into();
    account.character_list.reverse();
    let views = managed_view(&saved, Some(&changed));
    assert_eq!(views[0].status, "matched");
    assert_eq!(views[1].status, "missing");
    assert_eq!(views[2].status, "identity_changed");
    assert!(views[2].character.identity == saved.characters[2].identity);
    let result = validate_managed(&main, &saved, &ids(&[0, 1, 2]), &changed).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(
        validate_managed(&main, &ManagedSelection::default(), &ids(&[0, 1]), &changed)
            .err()
            .unwrap()
            .code,
        "selection_scope"
    );
    assert_eq!(managed_view(&saved, None)[1].status, "no_data");
    assert_eq!(
        super::validate_managed(&main, &saved, &ids(&[0]), None)
            .unwrap()
            .len(),
        1
    );
    assert!(super::validate_managed(&main, &saved, &ids(&[0, 7]), None).is_err());
    let mut no_main = changed.clone();
    for account in &mut no_main.account_list {
        account.character_list.retain(|c| c.ocid != "c0");
    }
    assert_eq!(
        super::validate_managed(&main, &saved, &ids(&[0, 1]), Some(&no_main))
            .unwrap()
            .len(),
        2
    );
    assert!(super::validate_managed(&main, &saved, &ids(&[0, 7]), Some(&no_main)).is_err());
}
