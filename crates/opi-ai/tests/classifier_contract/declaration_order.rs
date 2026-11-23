//! Declaration insertion, enumeration and serialization order.

use opi_ai::api::llama_cpp_classify::render_question;
use opi_ai::classifier::{ClassifierContext, ClassifierQuestion};

use super::fixtures::bool_question;

#[test]
fn existing_key_replacement_updates_value() {
    let context = ClassifierContext {
        state: serde_json::json!({}),
        images: None,
        questions: [(
            "pick".into(),
            ClassifierQuestion::Choice {
                instructions: "pick".into(),
                criteria: [
                    ("alpha".into(), "old".into()),
                    ("zeta".into(), "other".into()),
                    ("alpha".into(), "updated".into()),
                ]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect(),
    };
    let rendered = render_question(&context, "pick").unwrap();
    assert_eq!(rendered.keys, ["alpha", "zeta"]);
    assert!(rendered.content.contains("alpha: updated"));
    assert!(!rendered.content.contains("alpha: old"));
}

#[test]
fn integer_property_prefix_is_numeric_before_insertion_tail() {
    let context = ClassifierContext {
        state: serde_json::json!({}),
        images: None,
        questions: [(
            "pick".into(),
            ClassifierQuestion::Choice {
                instructions: "pick".into(),
                criteria: ["10", "2", "01", "zeta", "alpha"]
                    .map(|key| (key.into(), String::new()))
                    .into_iter()
                    .collect(),
            },
        )]
        .into_iter()
        .collect(),
    };
    assert_eq!(
        render_question(&context, "pick").unwrap().keys,
        ["2", "10", "01", "zeta", "alpha"]
    );
}

#[test]
fn canonical_index_boundaries_and_question_reinsertion() {
    let mut context = ClassifierContext::default();
    for key in [
        "4294967295",
        "01",
        "-0",
        "",
        "4294967294",
        "0",
        "2",
        "zeta",
        "alpha",
    ] {
        assert!(
            context
                .questions
                .insert(key.into(), bool_question(key))
                .is_none()
        );
    }
    let keys = |context: &ClassifierContext| {
        context
            .questions
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("|")
    };
    assert_eq!(
        keys(&context),
        "0|2|4294967294|4294967295|01|-0||zeta|alpha"
    );
    assert_eq!(context.questions.len(), 9);
    let old = context
        .questions
        .insert("zeta".into(), bool_question("updated"))
        .unwrap();
    assert_eq!(old.instructions(), "zeta");
    assert_eq!(
        context.questions.get("zeta").unwrap().instructions(),
        "updated"
    );
    assert_eq!(
        keys(&context),
        "0|2|4294967294|4294967295|01|-0||zeta|alpha"
    );
    let removed = context.questions.remove("zeta").unwrap();
    context.questions.insert("zeta".into(), removed);
    assert_eq!(
        keys(&context),
        "0|2|4294967294|4294967295|01|-0||alpha|zeta"
    );
    assert!(context.questions.remove("absent").is_none());
    assert!(!context.questions.is_empty());
}

#[test]
fn choice_replacement_and_serialization_retain_enumeration() {
    let question = ClassifierQuestion::Choice {
        instructions: "pick".into(),
        criteria: [
            ("zeta".into(), "old".into()),
            ("alpha".into(), "other".into()),
            ("10".into(), "ten".into()),
            ("2".into(), "two".into()),
            ("zeta".into(), "updated".into()),
        ]
        .into(),
    };
    let ClassifierQuestion::Choice { mut criteria, .. } = question else {
        unreachable!()
    };
    assert_eq!(criteria.len(), 4);
    assert_eq!(
        criteria.keys().map(String::as_str).collect::<Vec<_>>(),
        ["2", "10", "zeta", "alpha"]
    );
    assert_eq!(criteria.get("zeta").map(String::as_str), Some("updated"));
    let removed = criteria.remove("zeta").unwrap();
    assert_eq!(criteria.insert("zeta".into(), removed), None);
    let serialized = serde_json::to_value(&criteria).unwrap();
    assert_eq!(
        serialized
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["2", "10", "alpha", "zeta"]
    );
    assert_eq!(serialized["zeta"], "updated");
    assert_eq!(
        criteria.values().map(String::as_str).collect::<Vec<_>>(),
        ["two", "ten", "other", "updated"]
    );
    assert_eq!(
        criteria.clone().into_iter().collect::<Vec<_>>(),
        [
            ("2".into(), "two".into()),
            ("10".into(), "ten".into()),
            ("alpha".into(), "other".into()),
            ("zeta".into(), "updated".into()),
        ]
    );
}
