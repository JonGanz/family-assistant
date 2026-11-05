use schemars::JsonSchema;
use serde_json::Value;

/// JSON Schema for `T` without `$schema`; every object gets `additionalProperties: false`
/// (OpenAI strict mode requires it; Ollama's `format` accepts and ignores it).
pub fn json_schema_for<T: JsonSchema>() -> Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null);
    if let Value::Object(m) = &mut v {
        m.remove("$schema");
    }
    close_objects(&mut v);
    v
}

fn close_objects(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if m.get("type").and_then(Value::as_str) == Some("object")
                && m.contains_key("properties")
            {
                m.entry("additionalProperties")
                    .or_insert(Value::Bool(false));
            }
            for c in m.values_mut() {
                close_objects(c);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(close_objects),
        _ => {}
    }
}
