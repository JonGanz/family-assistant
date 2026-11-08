use super::*;

fn n() -> Normaliser {
    Normaliser::standard()
}

type Row = (
    &'static str,
    Option<f64>,
    Option<&'static str>,
    Option<&'static str>,
);

#[test]
fn lines_parse_and_clean() {
    let rows: &[Row] = &[
        // plain, fractions, ranges
        ("2 cups flour", Some(2.0), Some("cup"), Some("flour")),
        ("1/2 cup sugar", Some(0.5), Some("cup"), Some("sugar")),
        ("1 1/2 cups milk", Some(1.5), Some("cup"), Some("milk")),
        ("½ tsp salt", Some(0.5), Some("tsp"), Some("salt")),
        (
            "1½ tbsp olive oil",
            Some(1.5),
            Some("tbsp"),
            Some("olive oil"),
        ),
        ("2-3 tbsp butter", Some(2.0), Some("tbsp"), Some("butter")),
        (
            "3 to 4 cloves garlic",
            Some(3.0),
            Some("clove"),
            Some("garlic"),
        ),
        ("250 g pasta", Some(250.0), Some("g"), Some("pasta")),
        ("1 kg potatoes", Some(1.0), Some("kg"), Some("potato")),
        (
            "2 lb ground beef",
            Some(2.0),
            Some("lb"),
            Some("ground beef"),
        ),
        (
            "8 oz cream cheese",
            Some(8.0),
            Some("oz"),
            Some("cream cheese"),
        ),
        ("500 ml stock", Some(500.0), Some("ml"), Some("stock")),
        // the can form
        (
            "1 (14 oz) can tomatoes",
            Some(1.0),
            Some("can"),
            Some("tomato"),
        ),
        (
            "2 (15-ounce) cans black beans",
            Some(2.0),
            Some("can"),
            Some("black bean"),
        ),
        // plurals and exceptions
        ("3 tomatoes", Some(3.0), None, Some("tomato")),
        ("2 cherry tomatoes", Some(2.0), None, Some("cherry tomato")),
        ("4 eggs", Some(4.0), None, Some("egg")),
        ("2 cups chickpeas", Some(2.0), Some("cup"), Some("chickpea")),
        ("1 cup lentils", Some(1.0), Some("cup"), Some("lentil")),
        ("1 cup oats", Some(1.0), Some("cup"), Some("oat")),
        ("1 cup hummus", Some(1.0), Some("cup"), Some("hummus")),
        (
            "1 bunch asparagus",
            Some(1.0),
            Some("bunch"),
            Some("asparagus"),
        ),
        ("2 loaves", Some(2.0), None, Some("loaf")),
        (
            "2 cups brussels sprouts",
            Some(2.0),
            Some("cup"),
            Some("brussels sprout"),
        ),
        ("3 green onions", Some(3.0), None, Some("green onion")),
        ("2 peaches", Some(2.0), None, Some("peach")),
        // descriptors
        ("1 tsp kosher salt", Some(1.0), Some("tsp"), Some("salt")),
        ("2 large eggs", Some(2.0), None, Some("egg")),
        (
            "1 cup fresh basil leaves",
            Some(1.0),
            Some("cup"),
            Some("basil leaf"),
        ),
        (
            "1 lb boneless skinless chicken breasts",
            Some(1.0),
            Some("lb"),
            Some("chicken breast"),
        ),
        ("2 medium ripe bananas", Some(2.0), None, Some("banana")),
        // preparation and parentheticals
        ("1 onion, diced", Some(1.0), None, Some("onion")),
        (
            "2 carrots, peeled and sliced",
            Some(2.0),
            None,
            Some("carrot"),
        ),
        (
            "1 cup cheddar (shredded)",
            Some(1.0),
            Some("cup"),
            Some("cheddar"),
        ),
        (
            "1 cup finely chopped parsley",
            Some(1.0),
            Some("cup"),
            Some("parsley"),
        ),
        // no quantity
        ("Salt", None, None, Some("salt")),
        ("salt to taste", None, None, Some("salt")),
        (
            "Freshly ground black pepper",
            None,
            None,
            Some("ground black pepper"),
        ),
        // folding
        (
            "2 cups jalapeño peppers",
            Some(2.0),
            Some("cup"),
            Some("jalapeno pepper"),
        ),
        (
            "1 cup crème fraîche",
            Some(1.0),
            Some("cup"),
            Some("creme fraiche"),
        ),
        // units spelled out
        (
            "2 tablespoons soy sauce",
            Some(2.0),
            Some("tbsp"),
            Some("soy sauce"),
        ),
        (
            "1 teaspoon vanilla extract",
            Some(1.0),
            Some("tsp"),
            Some("vanilla extract"),
        ),
        ("3 slices bacon", Some(3.0), Some("slice"), Some("bacon")),
    ];
    let n = n();
    let mut bad = Vec::new();
    for (raw, qty, unit, item) in rows {
        let got = n.normalise(raw);
        let ok = got.qty == *qty && got.unit.as_deref() == *unit && got.item.as_deref() == *item;
        if !ok {
            bad.push(format!("{raw:?} -> {got:?}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert!(rows.len() >= 40, "{} rows", rows.len());
}

#[test]
fn notes_collect_preparation_and_parentheticals() {
    let n = n();
    let l = n.normalise("1 onion, diced");
    assert_eq!(l.note.as_deref(), Some("diced"));
    let l = n.normalise("salt to taste");
    assert_eq!(l.note.as_deref(), Some("to taste"));
    let l = n.normalise("1 cup cheddar (shredded)");
    assert_eq!(l.note.as_deref(), Some("shredded"));
    let l = n.normalise("1 cup finely chopped parsley");
    assert_eq!(l.note.as_deref(), Some("finely, chopped"));
    let (item, note) = n.clean_item("tomatoes (15-oz), drained");
    assert_eq!(item, "tomato");
    assert_eq!(note.as_deref(), Some("15-oz, drained"));
    let l = n.normalise("2 eggs");
    assert_eq!(l.note, None);
}

#[test]
fn clean_item_never_ends_empty_and_takes_lists() {
    let n = n();
    assert_eq!(n.clean_item("large").0, "large");
    let (item, _) = clean_item("  Fresh  Mint ", &["fresh".to_string()], &[]);
    assert_eq!(item, "mint");
    // A household list replaces the seed list.
    let custom = Normaliser::with_descriptors(&["smoked".to_string()]);
    assert_eq!(custom.clean_item("smoked paprika").0, "paprika");
    assert_eq!(custom.clean_item("fresh basil").0, "fresh basil");
    assert_eq!(
        Normaliser::with_descriptors(&[])
            .clean_item("fresh basil")
            .0,
        "basil"
    );
}

#[test]
fn singular_rules() {
    let n = n();
    for (plural, singular) in [
        ("tomatoes", "tomato"),
        ("potatoes", "potato"),
        ("berries", "berry"),
        ("cloves", "clove"),
        ("leaves", "leaf"),
        ("radishes", "radish"),
        ("molasses", "molasses"),
        ("couscous", "couscous"),
        ("brussels sprouts", "brussels sprout"),
        ("red onions", "red onion"),
        ("rice", "rice"),
    ] {
        assert_eq!(n.singular(plural), singular, "{plural}");
    }
}

fn base(q: f64, u: Option<&str>) -> Option<(f64, String)> {
    to_base(q, u)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6 * b.abs().max(1.0)
}

#[test]
fn conversion_stays_in_its_dimension() {
    let (q, u) = base(2.0, Some("cup")).unwrap();
    assert!(close(q, 480.0) && u == "ml");
    let (q, u) = base(1.0, Some("tbsp")).unwrap();
    assert!(close(q, 14.7868) && u == "ml");
    let (q, u) = base(3.0, Some("tsp")).unwrap();
    assert!(close(q, 14.78676) && u == "ml");
    let (q, u) = base(1.0, Some("fl oz")).unwrap();
    assert!(close(q, 29.5735) && u == "ml");
    let (q, u) = base(1.0, Some("l")).unwrap();
    assert!(close(q, 1000.0) && u == "ml");
    let (q, u) = base(1.0, Some("gallon")).unwrap();
    assert!(close(q, 3785.41) && u == "ml");
    let (q, u) = base(1.0, Some("lb")).unwrap();
    assert!(close(q, 453.592) && u == "g");
    let (q, u) = base(2.0, Some("oz")).unwrap();
    assert!(close(q, 56.699) && u == "g");
    let (q, u) = base(1.5, Some("kg")).unwrap();
    assert!(close(q, 1500.0) && u == "g");
    assert_eq!(base(3.0, Some("clove")), Some((3.0, "clove".to_string())));
    assert_eq!(base(2.0, None), Some((2.0, "count".to_string())));
    assert_eq!(base(1.0, Some("handful")), None);
    // Volume and weight are different dimensions and never convert to each other.
    assert_ne!(
        dimension_of(&base(1.0, Some("cup")).unwrap().1),
        dimension_of(&base(1.0, Some("lb")).unwrap().1)
    );
    assert_eq!(dimension_of("clove"), Dimension::Count);
    assert_eq!(canonical_unit("Tablespoons"), Some("tbsp"));
    assert_eq!(canonical_unit("lbs"), Some("lb"));
    assert_eq!(canonical_unit("handful"), None);
}

#[test]
fn display_metric() {
    use fa_domain::Units::Metric as M;
    let rows: &[(f64, &str, &str)] = &[
        (1.2, "ml", "1 ml"),
        (7.4, "ml", "7 ml"),
        (14.7868, "ml", "15 ml"),
        (73.0, "ml", "75 ml"),
        (240.0, "ml", "250 ml"),
        (480.0, "ml", "475 ml"),
        (990.0, "ml", "1 l"),
        (1500.0, "ml", "1.5 l"),
        (2349.0, "ml", "2.3 l"),
        (3.0, "g", "3 g"),
        (28.3495, "g", "30 g"),
        (250.0, "g", "250 g"),
        (761.0, "g", "750 g"),
        (1000.0, "g", "1 kg"),
        (1500.0, "g", "1.5 kg"),
        (3.0, "count", "3"),
        (0.5, "count", "½"),
        (2.5, "count", "2 ½"),
        (1.0 / 3.0, "count", "⅓"),
        (2.0, "clove", "2 cloves"),
        (1.0, "can", "1 can"),
        (2.0, "bunch", "2 bunches"),
        (1.0, "pinch", "1 pinch"),
    ];
    for (q, u, want) in rows {
        assert_eq!(display(*q, u, M), *want, "{q} {u}");
    }
    assert_eq!(display(0.0, "g", M), "");
    assert_eq!(display(f64::NAN, "g", M), "");
    assert_eq!(display(2.0, "handful", M), "2 handful");
}

#[test]
fn display_imperial() {
    use fa_domain::Units::Imperial as I;
    let rows: &[(f64, &str, &str)] = &[
        (1.0, "ml", "¼ tsp"),
        (2.46446, "ml", "½ tsp"),
        (4.92892, "ml", "1 tsp"),
        (7.4, "ml", "1 ½ tsp"),
        (12.3, "ml", "2 ½ tsp"),
        (14.7868, "ml", "1 tbsp"),
        (44.36, "ml", "3 tbsp"),
        (22.0, "ml", "1 ½ tbsp"),
        (60.0, "ml", "¼ cup"),
        (80.0, "ml", "⅓ cup"),
        (120.0, "ml", "½ cup"),
        (160.0, "ml", "⅔ cup"),
        (180.0, "ml", "¾ cup"),
        (240.0, "ml", "1 cup"),
        (600.0, "ml", "2 ½ cups"),
        (960.0, "ml", "4 cups"),
        (10.0, "g", "½ oz"),
        (28.3495, "g", "1 oz"),
        (100.0, "g", "3 ½ oz"),
        (200.0, "g", "7 oz"),
        (453.592, "g", "1 lb"),
        (680.0, "g", "1 ½ lb"),
        (900.0, "g", "2 lb"),
        (1250.0, "g", "2 ¾ lb"),
        (3.0, "count", "3"),
        (2.0, "clove", "2 cloves"),
    ];
    for (q, u, want) in rows {
        assert_eq!(display(*q, u, I), *want, "{q} {u}");
    }
}

#[test]
fn display_pair_joins_two_dimensions() {
    use fa_domain::Units::Metric as M;
    assert_eq!(
        display_pair((Some(480.0), Some("ml")), (Some(200.0), Some("g")), M),
        "475 ml + 200 g"
    );
    assert_eq!(
        display_pair((Some(3.0), Some("count")), (None, None), M),
        "3"
    );
    assert_eq!(display_pair((None, None), (None, None), M), "");
}

#[test]
fn parse_line_wraps_the_recipe_parser() {
    let p = Normaliser::parse_line("  1 1/2 cups flour, sifted ");
    assert_eq!(p.qty, Some(1.5));
    assert_eq!(p.unit.as_deref(), Some("cup"));
    assert_eq!(p.note.as_deref(), Some("sifted"));
    let p = Normaliser::parse_line("Pepper");
    assert_eq!((p.qty, p.item.as_str()), (None, "Pepper"));
}
