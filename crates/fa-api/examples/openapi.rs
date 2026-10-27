//! Writes the OpenAPI document without building `fa-server` (same output as
//! `fa-server gen-openapi`): `cargo run -p fa-api --example openapi -- <out>`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "apps/web/src/gen/openapi.json".into());
    let json = serde_json::to_string_pretty(&fa_api::openapi())?;
    std::fs::write(&out, json + "\n")?;
    println!("wrote {out}");
    Ok(())
}
