use serde_json::{Value,json};
use std::io::{self,BufRead};
fn main() {
    for l in io::stdin().lock().lines() {
        let c:Value=serde_json::from_str(&l.unwrap()).unwrap();
        let value=c["value"].as_str().unwrap();
        let result=if c["kind"]=="number-string" {
            let value:f64=value.parse().unwrap();
            json!(ryu_js::Buffer::new().format(value))
        } else { json!(url::Url::parse(value).is_ok()) };
        println!("{}",json!({"id":c["id"],"result":result}));
    }
}
