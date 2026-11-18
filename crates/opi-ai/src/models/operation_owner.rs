//! Operation registration ownership for the [`Models`](crate::models::Models) facade.

use std::collections::BTreeMap;

use super::{
    ChatApiRegistration, ClassifierApiRegistration, DeferredApiRegistration, ImageApiRegistration,
};
use crate::types::ApiId;

/// Operation registration owner behind the [`Models`] facade.
///
/// Chat, image, classifier and deferred adapters share exact API identity
/// registration and capture rules, so their maps move together behind one
/// private owner.
pub(crate) struct OperationOwner {
    pub(crate) chat_apis: BTreeMap<ApiId, ChatApiRegistration>,
    pub(crate) image_apis: BTreeMap<(String, ApiId), ImageApiRegistration>,
    pub(crate) classifier_apis: BTreeMap<(String, ApiId), ClassifierApiRegistration>,
    pub(crate) deferred_apis: BTreeMap<ApiId, DeferredApiRegistration>,
}

impl OperationOwner {
    pub(crate) fn new() -> Self {
        Self {
            chat_apis: BTreeMap::new(),
            image_apis: BTreeMap::new(),
            classifier_apis: BTreeMap::new(),
            deferred_apis: BTreeMap::new(),
        }
    }
}
