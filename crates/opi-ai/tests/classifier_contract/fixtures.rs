//! Classifier inputs and diagnostic checks shared by multiple test modules.

use opi_ai::classifier::{
    ClassifierContext, ClassifierModelRecord, ClassifierQuestion, ClassifierResult,
    ClassifierStopReason,
};
use opi_ai::model_info::ModelPricing;
use opi_ai::stream::Pricing;

pub(super) fn classify_context() -> ClassifierContext {
    ClassifierContext {
        state: serde_json::json!({ "text": "The deployment succeeded, thank you." }),
        questions: [
            (
                "category".to_owned(),
                ClassifierQuestion::Choice {
                    instructions: "Classify the message".into(),
                    criteria: [
                        ("success".to_owned(), "Successful".to_owned()),
                        ("failure".to_owned(), "Failed".to_owned()),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
            (
                "satisfaction".to_owned(),
                ClassifierQuestion::Score {
                    instructions: "Score satisfaction".into(),
                    levels: vec!["low".into(), "neutral".into(), "high".into()],
                },
            ),
            (
                "approved".to_owned(),
                ClassifierQuestion::Bool {
                    instructions: "Does the user approve?".into(),
                    true_criteria: "Approval".into(),
                    false_criteria: "No approval".into(),
                },
            ),
        ]
        .into_iter()
        .collect(),
        images: None,
    }
}

pub(super) fn assert_failure_diagnostics_private(
    result: &ClassifierResult,
    record: &ClassifierModelRecord,
    canary: &str,
) {
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(result.api, record.api.as_str());
    assert_eq!(result.provider, record.provider_id);
    assert_eq!(result.model, record.id);
    let error = result.error.as_ref().expect("typed failure");
    for diagnostic in [
        format!("{result:?}"),
        result.error_message.clone().expect("display diagnostic"),
        error.to_string(),
        format!("{error:?}"),
    ] {
        assert!(!diagnostic.contains(canary), "diagnostic: {diagnostic}");
    }
}

pub(super) fn zero_pricing() -> ModelPricing {
    ModelPricing::try_new(Pricing::default(), Vec::new()).expect("zero pricing is valid")
}

pub(super) fn bool_question(instructions: &str) -> ClassifierQuestion {
    ClassifierQuestion::Bool {
        instructions: instructions.into(),
        true_criteria: "yes".into(),
        false_criteria: "no".into(),
    }
}

pub(super) fn bool_context(ids: &[&str]) -> ClassifierContext {
    let mut context = ClassifierContext::default();
    for id in ids {
        context.questions.insert(
            (*id).into(),
            ClassifierQuestion::Bool {
                instructions: "Is this true?".into(),
                true_criteria: "Yes".into(),
                false_criteria: "No".into(),
            },
        );
    }
    context
}
