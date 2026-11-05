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
    assert_eq!(d.steps, ["Chop.", "Stir.", "Serve."]);
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
            dialect: Dialect::OpenaiChat,
        },
        default_http().unwrap(),
    )
}

#[tokio::test]
async fn llm_fallback() {
    let c = stub_client(
        r#"{"name":"Grandma Soup","yield_text":"","servings":2,"prep_minutes":0,"cook_minutes":10,"total_minutes":0,"ingredients":["2 cups water","1 tsp salt"],"steps":["Boil."],"tags":["soup","soup"]}"#,
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
