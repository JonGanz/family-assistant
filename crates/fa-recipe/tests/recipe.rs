use axum::{Json, Router, routing::post};
use fa_connectors::http::Http;
use fa_llm::{Dialect, EndpointConfig, LlmClient, default_http};
use fa_recipe::*;
use serde_json::json;
use std::time::Duration;

const PLAIN: &str = include_str!("fixtures/plain.html");
const GRAPH: &str = include_str!("fixtures/graph.html");
const NO_LD: &str = include_str!("fixtures/nojsonld.html");

#[test]
fn plain_object() {
    let u = url::Url::parse("https://example.com/r").unwrap();
    let d = from_html(PLAIN, Some(&u)).unwrap();
    assert_eq!(d.name, "Lemon Pasta");
    assert_eq!(d.origin, RecipeOrigin::Jsonld);
    assert_eq!(d.source_url.as_deref(), Some("https://example.com/r"));
    assert_eq!(d.yield_text.as_deref(), Some("4 servings"));
    assert_eq!(d.servings, Some(4));
    assert_eq!(
        (d.prep_minutes, d.cook_minutes, d.total_minutes),
        (Some(15), Some(90), Some(105))
    );
    assert_eq!(d.ingredients.len(), 3);
    assert_eq!(d.steps, ["Boil water.", "Add pasta."]);
    assert_eq!(d.tags, ["easy", "Pasta", "Dinner", "Italian"]);
    assert_eq!(d.image_url.as_deref(), Some("https://example.com/p.jpg"));
}

#[test]
fn graph_array_type_sections() {
    let d = from_html(GRAPH, None).unwrap();
    assert_eq!(d.name, "Fictional Stew");
    assert_eq!(d.yield_text.as_deref(), Some("6"));
    assert_eq!(d.servings, Some(6));
    assert_eq!(d.steps, ["Serve."]);
    assert_eq!(d.sections.len(), 1);
    assert_eq!(d.sections[0].name, "Prep");
    assert_eq!(d.sections[0].steps, ["Chop.", "Stir."]);
    assert!(d.sections[0].ingredients.is_empty());
    assert_eq!(d.ingredients.len(), 2);
    assert_eq!(d.image_url.as_deref(), Some("https://example.com/a.jpg"));
    assert_eq!(d.source_url, None);
}

#[test]
fn no_jsonld_is_none() {
    assert!(from_html(NO_LD, None).is_none());
    let t = visible_text(NO_LD);
    assert_eq!(t, "Grandma Soup Boil 2 cups water with 1 tsp salt.");
}

#[test]
fn durations() {
    assert_eq!(parse_iso_duration("PT1H30M"), Some(90));
    assert_eq!(parse_iso_duration("PT45M"), Some(45));
    assert_eq!(parse_iso_duration("P0DT1H"), Some(60));
    assert_eq!(parse_iso_duration("PT90S"), Some(2));
    assert_eq!(parse_iso_duration("PT61S"), Some(2));
    for bad in ["", "P", "PT", "1H", "PT5", "soon", "PT1X"] {
        assert_eq!(parse_iso_duration(bad), None, "{bad}");
    }
}

fn ing(l: &str) -> (Option<f64>, Option<&'static str>, Option<String>) {
    let i = parse_ingredient(l);
    assert_eq!(i.text, l);
    (
        i.quantity,
        i.unit
            .map(|u| Box::leak(u.into_boxed_str()) as &'static str),
        i.item,
    )
}

#[test]
fn ingredients() {
    assert_eq!(
        ing("1 ½ cups flour"),
        (Some(1.5), Some("cup"), Some("flour".into()))
    );
    assert_eq!(ing("1½ cups flour").0, Some(1.5));
    assert_eq!(ing("1 1/2 cups flour").0, Some(1.5));
    assert_eq!(
        ing("2-3 cloves garlic"),
        (Some(2.0), Some("clove"), Some("garlic".into()))
    );
    assert_eq!(ing("2 to 3 cloves garlic").0, Some(2.0));
    assert_eq!(
        ing("1/4 tsp salt"),
        (Some(0.25), Some("tsp"), Some("salt".into()))
    );
    assert_eq!(ing("salt to taste"), (None, None, None));
    assert_eq!(
        ing("200 g of butter"),
        (Some(200.0), Some("g"), Some("butter".into()))
    );
    assert_eq!(
        ing("200g butter"),
        (Some(200.0), Some("g"), Some("butter".into()))
    );
    assert_eq!(
        ing("2 garlic bulbs"),
        (Some(2.0), None, Some("garlic bulbs".into()))
    );
    assert_eq!(ing("0.5 lb beef").0, Some(0.5));
    assert_eq!(ing("¾ cup milk").0, Some(0.75));
}

async fn serve(app: Router) -> url::Url {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    url::Url::parse(&format!("http://{addr}")).unwrap()
}

async fn stub_client(content: &str) -> LlmClient {
    let content = content.to_string();
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let c = content.clone();
            async move { Json(json!({"choices":[{"message":{"content": c}}]})) }
        }),
    );
    let base = serve(app).await.join("/v1").unwrap();
    LlmClient::new(
        EndpointConfig {
            base_url: base,
            model: "m".into(),
            api_key: None,
            timeout: Duration::from_secs(5),
            thinking: false,
            dialect: Dialect::OpenaiChat,
        },
        default_http().unwrap(),
    )
}

#[tokio::test]
async fn llm_fallback() {
    let c = stub_client(
        r#"{"name":"Grandma Soup","yield_text":"","servings":2,"prep_minutes":0,"cook_minutes":10,"total_minutes":0,"ingredients":[{"text":"2 cups water","section":null},{"text":"1 tsp salt","section":null}],"steps":[{"text":"Boil.","section":null}],"tags":["soup","soup"]}"#,
    )
    .await;
    let u = url::Url::parse("https://example.com/s").unwrap();
    let d = from_llm(&c, "text", Some(&u)).await.unwrap();
    assert_eq!(d.origin, RecipeOrigin::Llm);
    assert_eq!(d.servings, Some(2));
    assert_eq!((d.prep_minutes, d.cook_minutes), (None, Some(10)));
    assert_eq!(d.yield_text, None);
    assert_eq!(d.ingredients[0].unit.as_deref(), Some("cup"));
    assert_eq!(d.tags, ["soup"]);
    assert_eq!(d.source_url.as_deref(), Some("https://example.com/s"));
}

#[tokio::test]
async fn llm_empty_is_no_recipe() {
    let c = stub_client(
        r#"{"name":"","yield_text":"","servings":0,"prep_minutes":0,"cook_minutes":0,"total_minutes":0,"ingredients":[],"steps":[],"tags":[]}"#,
    )
    .await;
    let e = from_llm(&c, "text", None).await.unwrap_err();
    assert!(matches!(e, fa_llm::LlmError::Schema(m) if m == "no recipe"));
}

#[tokio::test]
async fn extract_flow() {
    let http = Http::with_policy(true, false).unwrap();
    let html = |h: &str| RecipeInput::Html {
        html: h.into(),
        source_url: None,
    };
    let d = extract(&http, None, html(PLAIN)).await.unwrap();
    assert_eq!(d.origin, RecipeOrigin::Jsonld);
    let e = extract(&http, None, html(NO_LD)).await.unwrap_err();
    assert!(matches!(e, RecipeError::LlmUnavailable));
    let c = stub_client(
        r#"{"name":"","yield_text":"","servings":0,"prep_minutes":0,"cook_minutes":0,"total_minutes":0,"ingredients":[],"steps":[],"tags":[]}"#,
    )
    .await;
    let e = extract(&http, Some(&c), html(NO_LD)).await.unwrap_err();
    assert!(matches!(e, RecipeError::NoRecipe));
}

#[tokio::test]
async fn fetch_refuses_private_targets() {
    let http = Http::with_policy(true, false).unwrap();
    let u = url::Url::parse("http://127.0.0.1:1/x").unwrap();
    assert!(matches!(
        fetch_page(&http, &u).await,
        Err(RecipeError::Fetch(_))
    ));
}

const MD_FLAT: &str = include_str!("fixtures/microdata_flat.html");
const MD_STEPS: &str = include_str!("fixtures/microdata_steps.html");
const BOTH: &str = include_str!("fixtures/both.html");

#[test]
fn microdata_flat() {
    let u = url::Url::parse("https://example.com/f").unwrap();
    let d = from_microdata(MD_FLAT, Some(&u)).unwrap();
    assert_eq!(d.name, "Fictional Flatbread");
    assert_eq!(d.origin, RecipeOrigin::Microdata);
    assert_eq!(d.source_url.as_deref(), Some("https://example.com/f"));
    assert_eq!(d.image_url.as_deref(), Some("https://example.com/flat.jpg"));
    assert_eq!(
        (d.prep_minutes, d.cook_minutes, d.total_minutes),
        (Some(10), Some(65), None)
    );
    assert_eq!((d.servings, d.ingredients.len()), (Some(4), 2));
    assert_eq!(d.ingredients[0].unit.as_deref(), Some("cup"));
    assert_eq!(d.steps, ["Mix. Bake."]);
    assert_eq!(d.tags, ["quick", "bread", "Side"]);
}

#[test]
fn microdata_nested_steps() {
    let d = from_microdata(MD_STEPS, None).unwrap();
    assert_eq!(d.name, "Fictional Pie");
    assert_eq!(d.steps, ["Slice the tomatoes.", "Bake the pie."]);
    assert_eq!(d.ingredients[0].unit.as_deref(), Some("pint"));
}

#[test]
fn microdata_none_and_precedence() {
    assert!(from_microdata(NO_LD, None).is_none());
    assert!(from_microdata(PLAIN, None).is_none());
    assert_eq!(from_html(BOTH, None).unwrap().name, "From JSON-LD");
}

#[tokio::test]
async fn extract_microdata_without_llm() {
    let http = Http::with_policy(true, false).unwrap();
    let mk = |h: &str| RecipeInput::Html {
        html: h.into(),
        source_url: None,
    };
    let d = extract(&http, None, mk(MD_FLAT)).await.unwrap();
    assert_eq!(d.origin, RecipeOrigin::Microdata);
    let d = extract(&http, None, mk(BOTH)).await.unwrap();
    assert_eq!(d.origin, RecipeOrigin::Jsonld);
}

#[tokio::test]
async fn llm_pasted_text_without_url() {
    let c = stub_client(
        r#"{"name":"Pasted","yield_text":"","servings":0,"prep_minutes":0,"cook_minutes":0,"total_minutes":0,"ingredients":[{"text":"1 bunch cilantro","section":null}],"steps":[{"text":"Chop.","section":null}],"tags":[]}"#,
    )
    .await;
    let d = from_llm(&c, "pasted text", None).await.unwrap();
    assert_eq!(d.source_url, None);
    assert_eq!(d.ingredients[0].unit.as_deref(), Some("bunch"));
}

#[test]
fn ingredients_m5() {
    assert_eq!(
        ing("1 (14 oz) can diced tomatoes"),
        (Some(1.0), Some("can"), Some("diced tomatoes".into()))
    );
    assert_eq!(
        ing("2 (15-ounce) cans black beans, drained"),
        (Some(2.0), Some("can"), Some("black beans".into()))
    );
    assert_eq!(
        ing("1 pint cherry tomatoes"),
        (Some(1.0), Some("pint"), Some("cherry tomatoes".into()))
    );
    assert_eq!(
        ing("2 fl oz lime juice"),
        (Some(2.0), Some("fl oz"), Some("lime juice".into()))
    );
    assert_eq!(
        ing("2 fluid ounces milk"),
        (Some(2.0), Some("fl oz"), Some("milk".into()))
    );
    assert_eq!(
        ing("1 onion, finely chopped"),
        (Some(1.0), None, Some("onion".into()))
    );
    assert_eq!(ing("salt and pepper to taste"), (None, None, None));
    assert_eq!(
        ing("2 tsp salt, to taste"),
        (Some(2.0), Some("tsp"), Some("salt".into()))
    );
    assert_eq!(
        ing("1 bunch cilantro"),
        (Some(1.0), Some("bunch"), Some("cilantro".into()))
    );
    assert_eq!(
        ing("3 sprigs thyme"),
        (Some(3.0), Some("sprig"), Some("thyme".into()))
    );
    assert_eq!(ing("2 qt stock").1, Some("quart"));
    assert_eq!(ing("1 gal water").1, Some("gallon"));
    assert_eq!(ing("2 litres water").1, Some("l"));
}

const SECTIONED: &str = include_str!("fixtures/sectioned.html");
const WPRM: &str = include_str!("fixtures/wprm.html");
const TASTY: &str = include_str!("fixtures/tasty.html");
const MD_HEADS: &str = include_str!("fixtures/microdata_headings.html");

#[test]
fn jsonld_sections_with_headings() {
    let d = from_html(SECTIONED, None).unwrap();
    assert_eq!(d.sections.len(), 2);
    assert_eq!(d.sections[0].name, "Chicken");
    assert_eq!(d.sections[0].ingredients.len(), 2);
    assert_eq!(d.sections[0].steps, ["Sear the chicken.", "Rest it."]);
    assert_eq!(d.sections[1].name, "Sauce");
    assert_eq!(d.sections[1].ingredients.len(), 2);
    assert_eq!(d.sections[1].steps, ["Whisk the sauce."]);
    assert_eq!(d.ingredients.len(), 1);
    assert_eq!(d.ingredients[0].text, "1 pinch parsley");
    assert_eq!(d.steps, ["Serve warm."]);
    let all = d
        .sections
        .iter()
        .flat_map(|s| &s.ingredients)
        .chain(&d.ingredients);
    assert!(
        all.clone()
            .all(|i| !i.text.ends_with(':') && !i.text.starts_with("For the"))
    );
    assert_eq!(all.count(), 5);
}

#[test]
fn jsonld_step_sections_only() {
    let html = r#"<script type="application/ld+json">{"@type":"Recipe","name":"X","recipeIngredient":["1 egg","2 cups milk"],
"recipeInstructions":[{"@type":"HowToSection","name":"Base","itemListElement":[{"@type":"HowToStep","text":"Mix."}]},
{"@type":"HowToSection","name":"Top","itemListElement":[{"@type":"HowToSection","name":"Inner","itemListElement":[{"@type":"HowToStep","text":"Spread."}]}]}]}</script>"#;
    let d = from_html(html, None).unwrap();
    assert_eq!(d.ingredients.len(), 2);
    assert!(d.steps.is_empty());
    assert_eq!(d.sections.len(), 2);
    assert!(d.sections.iter().all(|s| s.ingredients.is_empty()));
    assert_eq!(d.sections[1].name, "Top");
    assert_eq!(d.sections[1].steps, ["Spread."]);
}

#[test]
fn unpaired_groups_stay_and_plural_pairs() {
    let html = r#"<script type="application/ld+json">{"@type":"Recipe","name":"X",
"recipeIngredient":["Garnish:","1 lime","For the sauces:","1 cup cream"],
"recipeInstructions":[{"@type":"HowToSection","name":"Make the sauce","itemListElement":[{"@type":"HowToStep","text":"Whisk."}]},
{"@type":"HowToSection","name":"Plate","itemListElement":[{"@type":"HowToStep","text":"Arrange."}]}]}</script>"#;
    let d = from_html(html, None).unwrap();
    let names: Vec<_> = d.sections.iter().map(|s| s.name.as_str()).collect();
    // Ingredient-group order is the backbone: Garnish (unpaired), then Sauces merged with
    // "Make the sauce", then the leftover step section.
    assert_eq!(names, ["Garnish", "Make the sauce", "Plate"]);
    assert_eq!(d.sections[1].ingredients.len(), 1);
    assert_eq!(d.sections[1].steps, ["Whisk."]);
}

#[test]
fn wprm_groups() {
    let d = from_html(WPRM, None).unwrap();
    assert_eq!(d.sections.len(), 2);
    assert_eq!(d.sections[0].name, "Dough");
    assert_eq!(d.sections[0].ingredients.len(), 2);
    assert_eq!(d.sections[1].name, "Filling");
    assert_eq!(d.ingredients.len(), 1);
    assert_eq!(d.ingredients[0].text, "1 pinch salt");
}

#[test]
fn tasty_groups() {
    let d = from_html(TASTY, None).unwrap();
    assert_eq!(d.sections.len(), 1);
    assert_eq!(d.sections[0].name, "Glaze");
    assert_eq!(d.sections[0].ingredients.len(), 2);
    assert_eq!(d.ingredients.len(), 2);
}

#[test]
fn microdata_headings() {
    let d = from_microdata(MD_HEADS, None).unwrap();
    assert_eq!(d.ingredients.len(), 1);
    assert_eq!(d.sections.len(), 1);
    assert_eq!(d.sections[0].name, "Topping");
    assert_eq!(d.sections[0].ingredients.len(), 2);
}

#[tokio::test]
async fn llm_sections() {
    let c = stub_client(
        r#"{"name":"Pie","yield_text":"","servings":0,"prep_minutes":0,"cook_minutes":0,"total_minutes":0,
"ingredients":[{"text":"1 pinch salt","section":null},{"text":"2 cups flour","section":"Crust"},{"text":"3 apples","section":"Filling"},{"text":"1 tbsp butter","section":"Crust"}],
"steps":[{"text":"Rub in butter.","section":"Crust"},{"text":"Slice apples.","section":"Filling"},{"text":"Bake.","section":null}],"tags":[]}"#,
    )
    .await;
    let d = from_llm(&c, "text", None).await.unwrap();
    assert_eq!(d.ingredients.len(), 1);
    assert_eq!(d.steps, ["Bake."]);
    assert_eq!(d.sections.len(), 2);
    assert_eq!(d.sections[0].name, "Crust");
    assert_eq!(d.sections[0].ingredients.len(), 2);
    assert_eq!(d.sections[0].steps, ["Rub in butter."]);
    assert_eq!(d.sections[1].name, "Filling");
}
