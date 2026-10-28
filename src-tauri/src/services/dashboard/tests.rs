use super::*;
use crate::domain::MainIdentity;

fn saved(index: usize) -> MainCharacter {
    MainCharacter {
        identity: MainIdentity {
            account_id: "a".into(),
            ocid: format!("c{index}"),
            world_name: "w".into(),
        },
        character_name: Some(format!("synthetic-{index}")),
        character_class: Some("job".into()),
        character_level: Some(260),
    }
}

#[test]
fn six_cards_preserve_order_main_and_individual_errors_without_data_loss() {
    let selection = ManagedSelection {
        revision: 7,
        characters: [0, 5, 3, 1, 4, 2].into_iter().map(saved).collect(),
    };
    let mut cache = DashboardCache::default();
    let first = cache.snapshot(&selection);
    assert_eq!(first.cards.len(), 6);
    assert!(first.cards[0].is_main);
    assert!(!first.cards[1].is_main);
    assert_eq!(first.cards[1].character.identity.ocid, "c5");
    cache.failure(&selection.characters[2], false, ApiError::new("timeout"));
    let cards = cache.snapshot(&selection).cards;
    assert_eq!(cards[2].error.as_ref().unwrap().code, "timeout");
    assert!(cards[1].error.is_none());
    assert!(cards[2].character == selection.characters[2]);
    assert!(cache.needs_refresh(&selection.characters[0], false));
    assert!(!cache.needs_refresh(&selection.characters[2], false));
    let mut moved = saved(1);
    moved.identity.world_name = "different".into();
    cache.snapshot(&ManagedSelection {
        revision: 8,
        characters: vec![moved],
    });
    assert!(cache.entries.is_empty());
}

#[test]
fn basic_identity_mismatch_keeps_saved_data_and_cache_avoids_rerender_requests() {
    let value = saved(0);
    let mut cache = DashboardCache::default();
    let mut card = cache.card(&value, true);
    let mut info = BasicInfo {
        world_name: "other".into(),
        character_name: Some("renamed".into()),
        character_class: None,
        character_level: Some(270),
        character_image: None,
    };
    assert!(apply_basic(&mut card, &info).is_err());
    assert!(card.character == value);
    info.world_name = "w".into();
    apply_basic(&mut card, &info).unwrap();
    assert_eq!(card.character.character_level, Some(270));
    assert_eq!(card.character.character_class.as_deref(), Some("job"));
    card.image_src = Some("data:image/png;base64,synthetic".into());
    cache.entries.insert(
        value.identity.ocid.clone(),
        CachedCard {
            card,
            image_url: None,
            attempted_at: Instant::now(),
        },
    );
    assert!(!cache.needs_refresh(&value, false));
    assert!(!cache.needs_refresh(&value, true));
    let failed = cache.failure(&value, true, ApiError::new("network"));
    assert_eq!(failed.character.character_level, Some(270));
    assert!(failed.image_src.is_some());
    let limited = cache.failure(
        &value,
        true,
        ApiError {
            code: "rate_limited",
            retry_after_seconds: Some(60),
        },
    );
    assert_eq!(limited.error.unwrap().code, "rate_limited");
    assert!(cache.blocked_until.unwrap() > Instant::now());
}

#[test]
fn credential_reset_clears_dashboard_memory() {
    let value = saved(0);
    let mut session = CharacterSession::default();
    session
        .dashboard
        .failure(&value, true, ApiError::new("network"));
    assert!(!session.dashboard.needs_refresh(&value, false));
    session.reset();
    assert!(session.dashboard.needs_refresh(&value, false));
    assert!(session.dashboard.card(&value, true).error.is_none());
}
