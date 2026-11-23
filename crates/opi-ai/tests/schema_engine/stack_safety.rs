//! Deep schema lifecycle checks in a child process with a 2 MiB worker stack.

use opi_ai::schema::{SchemaError, compile};
use serde_json::{Value, json};

fn ordinary_stack_subprocess(name: &str, test: fn()) {
    const CHILD_ENV: &str = "OPI_SCHEMA_LIFECYCLE_CHILD";
    const COMPLETED: &str = "schema stack check completed";
    let (_, topic) = module_path!()
        .split_once("::")
        .expect("nested stack test module");
    let name = format!("{topic}::{name}");
    if std::env::var(CHILD_ENV).as_deref() == Ok(name.as_str()) {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(test)
            .unwrap()
            .join()
            .unwrap();
        println!("\n{COMPLETED}");
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, &name)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.lines().any(|line| line == COMPLETED),
        "{name} did not complete the stack check: {:?}\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn nested_all_of(mut schema: Value) -> Value {
    for _ in 0..10_000 {
        let mut map = serde_json::Map::new();
        map.insert("allOf".into(), Value::Array(vec![schema]));
        schema = Value::Object(map);
    }
    schema
}

#[test]
fn deep_schemas_do_not_exhaust_the_rust_stack() {
    ordinary_stack_subprocess("deep_schemas_do_not_exhaust_the_rust_stack", || {
        let mut schema = nested_all_of(json!({"type":"integer"}));
        schema
            .as_object_mut()
            .unwrap()
            .insert("$id".into(), json!("https://offline.invalid/root"));
        let compiled = compile(schema).expect("compiles");
        assert_eq!(compiled.check(&json!(1)), Ok(true));
        let last_owner = compiled.clone();
        drop(compiled);
        assert_eq!(last_owner.check(&json!("bad")), Ok(false));
        drop(last_owner);
    });
}

#[test]
fn deep_local_ref_search() {
    ordinary_stack_subprocess("deep_local_ref_search", || {
        for reference in ["#target", "#missing"] {
            let mut schema = nested_all_of(json!({"$anchor":"target", "type":"integer"}));
            schema
                .as_object_mut()
                .unwrap()
                .insert("$ref".into(), json!(reference));
            let compiled = compile(schema).unwrap();
            assert_eq!(compiled.check(&json!(1)), Ok(reference == "#target"));
            drop(compiled);
        }
    });
}

#[test]
fn deep_compile_error_releases_owned_schema() {
    ordinary_stack_subprocess("deep_compile_error_releases_owned_schema", || {
        let schema = nested_all_of(json!({"pattern":"["}));
        assert!(matches!(compile(schema), Err(SchemaError::Regex { .. })));
    });
}
