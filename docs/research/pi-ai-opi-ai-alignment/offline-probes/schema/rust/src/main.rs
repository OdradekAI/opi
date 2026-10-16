use jsonschema::{Draft, Keyword, Retrieve, Uri, ValidationError, Validator};
use jsonschema::paths::{LazyLocation, Location};
use serde_json::{json, Value};
use std::{io::{self, BufRead}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

#[derive(Clone)]
struct DenyRetrieve(Arc<AtomicUsize>);
impl Retrieve for DenyRetrieve {
    fn retrieve(&self, _uri: &Uri<&str>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "offline probe: retrieval denied").into())
    }
}

struct Prefix(Vec<Validator>);
impl Keyword for Prefix {
    fn is_valid(&self, instance: &Value) -> bool {
        instance.as_array().map_or(true, |items| self.0.iter().zip(items).all(|(validator,item)| validator.is_valid(item)))
    }
    fn validate<'i>(&self, instance: &'i Value, location: &LazyLocation) -> Result<(), ValidationError<'i>> {
        if self.is_valid(instance) { Ok(()) } else { Err(ValidationError::custom(Location::new(), location.into(), instance, "prefixItems probe mismatch")) }
    }
}
struct Noop;
impl Keyword for Noop {
    fn is_valid(&self, _: &Value) -> bool { true }
    fn validate<'i>(&self, _: &'i Value, _: &LazyLocation) -> Result<(), ValidationError<'i>> { Ok(()) }
}

fn run(c: &Value, profile: &str, instance: &Value, stage: &str) -> Value {
    let retrieve_calls = Arc::new(AtomicUsize::new(0));
    let hook_calls = Arc::new(AtomicUsize::new(0));
    let mut options = jsonschema::options();
    options.with_retriever(DenyRetrieve(retrieve_calls.clone()));
    if profile.starts_with("draft7") { options.with_draft(Draft::Draft7); }
    else { options.with_draft(Draft::Draft202012); }
    options.should_validate_formats(true).should_ignore_unknown_formats(true);
    if profile == "draft7-content-off" {
        options.without_content_encoding_support("base64").without_content_media_type_support("application/json");
    }
    if profile == "draft7-prefix-hook" {
        let hooks = hook_calls.clone();
        options.with_keyword("prefixItems", move |_parent, value, _path| {
            hooks.fetch_add(1, Ordering::SeqCst);
            // Deliberately scoped mechanism probe: child roots without shared
            // refs. This is not an adapter proposal for arbitrary schemas.
            let mut validators = Vec::new();
            if let Some(children) = value.as_array() {
                for child in children {
                    validators.push(jsonschema::options().with_draft(Draft::Draft7).with_retriever(DenyRetrieve(Arc::new(AtomicUsize::new(0)))).should_validate_formats(true).build(child)?);
                }
            }
            Ok(Box::new(Prefix(validators)) as Box<dyn Keyword>)
        });
    }
    if profile == "draft7-invalid-minlength-hook" {
        let hooks = hook_calls.clone();
        options.with_keyword("minLength", move |_parent, _value, _path| {
            hooks.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(Noop) as Box<dyn Keyword>)
        });
    }
    let mut out=json!({"kind":"case","id":c["id"],"profile":profile,"instance_stage":stage});
    match options.build(&c["schema"]) {
        Ok(validator) => {
            out["build"]=json!("ok");
            out["is_valid"]=json!(validator.is_valid(instance));
            out["validate"]=match validator.validate(instance) {
                Ok(())=>json!({"outcome":"valid"}),
                Err(e)=>json!({"outcome":"invalid","message":e.to_string(),"schema_path":e.schema_path.as_str(),"instance_path":e.instance_path.as_str(),"kind":format!("{:?}", e.kind)})
            };
            out["iter_errors"]=json!(validator.iter_errors(instance).map(|e| json!({"message":e.to_string(),"schema_path":e.schema_path.as_str(),"instance_path":e.instance_path.as_str(),"kind":format!("{:?}",e.kind)})).collect::<Vec<_>>());
        }
        Err(e)=>{out["build"]=json!("error");out["build_error"]=json!({"message":e.to_string(),"schema_path":e.schema_path.as_str(),"instance_path":e.instance_path.as_str(),"kind":format!("{:?}",e.kind)});}
    }
    out["retriever_calls"]=json!(retrieve_calls.load(Ordering::SeqCst));
    out["hook_factory_calls"]=json!(hook_calls.load(Ordering::SeqCst));
    out
}
fn main() {
    for line in io::stdin().lock().lines() {
        let c:Value=serde_json::from_str(&line.unwrap()).unwrap();
        let original=&c["arguments"];
        let mut profiles=vec!["draft7-baseline","draft2020-baseline","draft7-content-off"];
        if c["id"].as_str().unwrap().starts_with("mixed-") { profiles.push("draft7-prefix-hook"); }
        if c["id"]=="malformed-minlength" { profiles.push("draft7-invalid-minlength-hook"); }
        for profile in profiles {
            println!("{}",run(&c,profile,original,"raw"));
            if let Some(js_input)=c.get("pi_input_js") {
                if js_input != original { println!("{}",run(&c,profile,js_input,"pi_input_js")); }
            }
            if let Some(normalized)=c.get("pi_normalized") {
                let js_input=c.get("pi_input_js").unwrap_or(original);
                if normalized != js_input { println!("{}",run(&c,profile,normalized,"pi_normalized")); }
            }
        }
    }
}
