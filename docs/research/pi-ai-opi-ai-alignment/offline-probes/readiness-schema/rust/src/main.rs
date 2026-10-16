//! Research-only: proves selected private-dialect mechanisms; no public SDK.
use regress::Regex;
use serde_json::{Value, json};
use std::io::{self, BufRead};

fn re(pattern: &str, value: &str) -> Result<bool, String> {
    if pattern == "__probe_runtime_failure__" { return Err("injected fallible leaf failure; research control only".into()); }
    Regex::with_flags(pattern, "u").map(|r| r.find(value).is_some()).map_err(|e| e.to_string())
}
fn modifier(c: u32) -> bool {
    matches!(c, 0x300..=0x36f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0xfe20..=0xfe2f | 0xfe00..=0xfe0f)
}
fn length(s: &str) -> usize {
    let p: Vec<_> = s.chars().map(u32::from).collect();
    let (mut i, mut n) = (0, 0);
    while i < p.len() {
        let first = p[i]; i += 1;
        while i < p.len() && modifier(p[i]) { i += 1; }
        while i + 1 < p.len() && p[i] == 0x200d {
            i += 2;
            while i < p.len() && modifier(p[i]) { i += 1; }
        }
        if (0x1f1e6..=0x1f1ff).contains(&first) && i < p.len() && (0x1f1e6..=0x1f1ff).contains(&p[i]) { i += 1; }
        n += 1;
    }
    n
}
fn typ(t: &str, v: &Value) -> bool {
    match t {
        "number" => v.is_number(), "integer" => v.as_f64().is_some_and(|n| n.is_finite() && n.fract() == 0.0),
        "object" => v.is_object(), "array" => v.is_array(), "null" => v.is_null(),
        "string" => v.is_string(), "boolean" => v.is_boolean(), _ => false
    }
}
fn id_target<'a>(root: &'a Value, id: &str) -> Option<&'a Value> {
    if root.get("$id").and_then(Value::as_str) == Some(id) { return Some(root); }
    match root {
        Value::Object(o) => o.values().find_map(|v| id_target(v, id)),
        Value::Array(a) => a.iter().find_map(|v| id_target(v, id)), _ => None
    }
}
// Scope intentionally excludes formats, dynamic/recursive refs and evaluated sets.
// Existing Pi normalized inputs may be supplied separately; no normalizer is implemented.
fn check(s: &Value, v: &Value, root: &Value) -> Result<bool, String> {
    if let Some(b) = s.as_bool() { return Ok(b); }
    let Some(o) = s.as_object() else { return Ok(true) };
    for key in ["format", "$dynamicRef", "$recursiveRef", "unevaluatedItems", "unevaluatedProperties"] {
        if o.contains_key(key) { return Err(format!("research mechanism not implemented: {key}")); }
    }
    if let Some(t) = o.get("type") {
        let valid = if let Some(t) = t.as_str() { typ(t, v) }
            else if let Some(ts) = t.as_array() { ts.iter().filter_map(Value::as_str).any(|t| typ(t, v)) } else { true };
        if !valid { return Ok(false); }
    }
    if let Some(obj) = v.as_object() {
        if let Some(required) = s.get("required").and_then(Value::as_array) {
            if !required.iter().filter_map(Value::as_str).all(|k| obj.contains_key(k)) { return Ok(false); }
        }
        let props = s.get("properties").and_then(Value::as_object);
        let patterns = s.get("patternProperties").and_then(Value::as_object);
        for (k, val) in obj {
            let mut covered = props.is_some_and(|p| p.contains_key(k));
            if let Some(pp) = patterns {
                for (pattern, _) in pp { covered |= re(pattern, k)?; }
            }
            if !covered {
                if let Some(additional) = s.get("additionalProperties") {
                    if !check(additional, val, root)? { return Ok(false); }
                }
            }
        }
        if let Some(pp) = patterns {
            for (pattern, child) in pp {
                for (k, val) in obj {
                    if re(pattern, k)? && !check(child, val, root)? { return Ok(false); }
                }
            }
        }
        if let Some(props) = props {
            for (k, child) in props {
                if let Some(val) = obj.get(k) { if !check(child, val, root)? { return Ok(false); } }
            }
        }
        if let Some(child) = s.get("propertyNames") {
            for k in obj.keys() { if !check(child, &json!(k), root)? { return Ok(false); } }
        }
    }
    if let Some(a) = v.as_array() {
        if let Some(child) = s.get("contains") {
            let mut any = false;
            for v in a { if check(child, v, root)? { any = true; break; } }
            if !any { return Ok(false); }
        }
        if let Some(items) = s.get("items") {
            if let Some(tuple) = items.as_array() {
                for (child, val) in tuple.iter().zip(a) { if !check(child, val, root)? { return Ok(false); } }
            } else {
                for val in a { if !check(items, val, root)? { return Ok(false); } }
            }
        }
        if let Some(tuple) = s.get("prefixItems").and_then(Value::as_array) {
            for (child, val) in tuple.iter().zip(a) { if !check(child, val, root)? { return Ok(false); } }
        }
    }
    if let Some(text) = v.as_str() {
        if let Some(n) = s.get("maxLength").and_then(Value::as_f64) { if length(text) as f64 > n { return Ok(false); } }
        if let Some(n) = s.get("minLength").and_then(Value::as_f64) { if (length(text) as f64) < n { return Ok(false); } }
        if let Some(p) = s.get("pattern").and_then(Value::as_str) { if !re(p, text)? { return Ok(false); } }
    }
    if let Some(n) = v.as_f64() {
        if let Some(min) = s.get("minimum").and_then(Value::as_f64) { if n < min { return Ok(false); } }
        if let Some(max) = s.get("maximum").and_then(Value::as_f64) { if n > max { return Ok(false); } }
        if let Some(m) = s.get("multipleOf").and_then(Value::as_f64) {
            // TypeBox guard numeric branch, including its source tolerance.
            let special = n.fract() == 0.0 && (1.0 / m).fract() == 0.0;
            let rem = n % m;
            if !special && !(rem.abs().min((rem-m).abs()).min((rem+m).abs()) < 1e-10) { return Ok(false); }
        }
    }
    if let Some(reference) = s.get("$ref").and_then(Value::as_str) {
        if reference == "#" { return Err("non-progress self-reference intentionally not executed in this subset harness".into()); }
        let target = if let Some(pointer) = reference.strip_prefix('#') { root.pointer(pointer) } else { id_target(root, reference) };
        let Some(target) = target else { return Ok(false) };
        if !check(target, v, root)? { return Ok(false); }
    }
    if let Some(c) = s.get("const") { if c != v { return Ok(false); } }
    if let Some(cs) = s.get("enum").and_then(Value::as_array) { if !cs.contains(v) { return Ok(false); } }
    if let Some(cond) = s.get("if") {
        let child = if check(cond, v, root)? { s.get("then") } else { s.get("else") };
        if let Some(child) = child { if !check(child, v, root)? { return Ok(false); } }
    }
    if let Some(child) = s.get("not") { if check(child, v, root)? { return Ok(false); } }
    if let Some(children) = s.get("allOf").and_then(Value::as_array) {
        for child in children { if !check(child, v, root)? { return Ok(false); } }
    }
    if let Some(children) = s.get("anyOf").and_then(Value::as_array) {
        let mut valid = false;
        for child in children { if check(child, v, root)? { valid = true; break; } }
        if !valid { return Ok(false); }
    }
    if let Some(children) = s.get("oneOf").and_then(Value::as_array) {
        let mut valid = 0;
        for child in children { if check(child, v, root)? { valid += 1; } }
        if valid != 1 { return Ok(false); }
    }
    Ok(true)
}
fn main() {
    for l in io::stdin().lock().lines() {
        let c: Value = serde_json::from_str(&l.unwrap()).unwrap();
        let result = if c["kind"] == "regex" {
            re(c["pattern"].as_str().unwrap(), c["subject"].as_str().unwrap())
        } else { check(&c["schema"], &c["arguments"], &c["schema"]) };
        let out = match result {
            Ok(valid) => json!({"id":c["id"],"outcome":"evaluated","valid":valid}),
            Err(error) => json!({"id":c["id"],"outcome":"mechanism-error","error":error})
        };
        println!("{out}");
    }
}
