//! Sanity checks for the hand-written seed catalogue (D-114).

use fa_meals::seed_data::{SeedAisle, SeedIngredient, SingularRule, raw};
use std::collections::HashSet;

fn ingredients() -> Vec<SeedIngredient> {
    serde_json::from_str(raw::INGREDIENTS).expect("ingredients.json parses")
}

fn aisles() -> Vec<SeedAisle> {
    serde_json::from_str(raw::AISLES).expect("aisles.json parses")
}

#[test]
fn every_file_parses() {
    assert!(!aisles().is_empty());
    assert!(!ingredients().is_empty());
    let d: Vec<String> = serde_json::from_str(raw::DESCRIPTORS).expect("descriptors.json parses");
    assert!(d.len() >= 30);
    let s: Vec<SingularRule> =
        serde_json::from_str(raw::SINGULAR).expect("singular_exceptions.json parses");
    assert!(!s.is_empty());
}

#[test]
fn canonical_names_unique_and_lower_case() {
    let mut seen = HashSet::new();
    for i in ingredients() {
        assert_eq!(i.canonical_name, i.canonical_name.to_lowercase());
        assert!(!i.canonical_name.trim().is_empty());
        assert!(
            seen.insert(i.canonical_name.clone()),
            "duplicate {}",
            i.canonical_name
        );
    }
}

#[test]
fn aliases_do_not_collide() {
    let ings = ingredients();
    let canon: HashSet<&str> = ings.iter().map(|i| i.canonical_name.as_str()).collect();
    let mut seen = HashSet::new();
    for i in &ings {
        for a in &i.aliases {
            assert_eq!(a, &a.to_lowercase(), "alias not lower case: {a}");
            assert!(
                !canon.contains(a.as_str()),
                "alias {a} equals a canonical name"
            );
            assert!(seen.insert(a.clone()), "alias {a} appears twice");
        }
    }
}

#[test]
fn aisles_are_valid() {
    let names: HashSet<String> = aisles().into_iter().map(|a| a.name).collect();
    assert!(!names.contains("Other"));
    for i in ingredients() {
        assert!(
            names.contains(&i.aisle),
            "{} has unknown aisle {}",
            i.canonical_name,
            i.aisle
        );
    }
}

#[test]
fn count_units_and_staples() {
    let ings = ingredients();
    assert!((250..=400).contains(&ings.len()), "count {}", ings.len());
    for i in &ings {
        assert!(
            matches!(i.default_unit.as_deref(), None | Some("g" | "ml" | "count")),
            "{} unit {:?}",
            i.canonical_name,
            i.default_unit
        );
        assert!(!i.staple);
    }
}

#[test]
fn no_email_url_or_digit() {
    for i in ingredients() {
        let all = std::iter::once(i.canonical_name.clone()).chain(i.aliases.clone());
        for t in all {
            assert!(
                !t.contains('@') && !t.contains("http") && !t.contains("www."),
                "{t}"
            );
            assert!(!t.chars().any(|c| c.is_ascii_digit()), "digit in {t}");
            assert!(t.is_ascii(), "non-ascii {t}");
        }
    }
}
