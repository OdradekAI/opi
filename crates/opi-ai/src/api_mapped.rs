//! One public provider identity and catalog dispatched across concrete wires.
//!
//! Construction validates the complete model-to-wire route graph. A mapped
//! provider delegates auth to its route providers; callers MUST construct every
//! route with the SAME lazy [`crate::AuthResolver`] so every stream observes
//! the current shared credential — one logical provider whose wires observe
//! divergent login/logout state is a correctness defect. `try_new` cannot
//! enforce this at the trait-object boundary (routes arrive as already-built
//! `Box<dyn Provider>` values), so the invariant is the caller's responsibility.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::stream;

use crate::credential::BoxAuthFuture;
use crate::model_info::{ModelInfoError, WireApi};
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};

/// A provider that exposes one provider identity/catalog while delegating to
/// one route per wire API used by its catalog.
///
/// [`try_new`](Self::try_new) rejects duplicate models/routes, missing or
/// unexpected routes, hidden provider ids, catalog mismatches, and
/// wire/compatibility mismatches before any request can reach the network.
/// [`Provider::stream`] then resolves the requested model in this public
/// catalog and delegates to its exact [`WireApi`] route.
pub struct ApiMappedProvider {
    id: String,
    models: Vec<ModelInfo>,
    routes: BTreeMap<WireApi, Box<dyn Provider>>,
}

impl std::fmt::Debug for ApiMappedProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApiMappedProvider")
            .field("id", &self.id)
            .field("models", &self.models)
            .field("routes", &self.routes.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ApiMappedProvider {
    /// Validate a complete catalog and its concrete routes before construction.
    ///
    /// Each route must expose the same provider id and exactly the catalog
    /// subset for its wire. Static mapped providers keep model refresh
    /// substrate disabled by returning `Ok(None)`.
    pub fn try_new<I>(
        id: impl Into<String>,
        models: Vec<ModelInfo>,
        routes: I,
    ) -> Result<Self, ApiMapError>
    where
        I: IntoIterator<Item = (WireApi, Box<dyn Provider>)>,
    {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ApiMapError::EmptyProviderId);
        }
        if models.is_empty() {
            return Err(ApiMapError::EmptyCatalog {
                provider_id: id.clone(),
            });
        }

        let mut model_ids = BTreeSet::new();
        let mut required_wires = BTreeSet::new();
        for model in &models {
            if model.id.trim().is_empty() {
                return Err(ApiMapError::EmptyModelId {
                    provider_id: id.clone(),
                });
            }
            if !model_ids.insert(model.id.clone()) {
                return Err(ApiMapError::DuplicateModel {
                    provider_id: id.clone(),
                    model_id: model.id.clone(),
                });
            }
            model
                .validate()
                .map_err(|source| ApiMapError::InvalidModel {
                    provider_id: id.clone(),
                    model_id: model.id.clone(),
                    source,
                })?;
            required_wires.insert(model.wire_api);
        }

        let mut route_map = BTreeMap::new();
        for (wire_api, route) in routes {
            if route_map.insert(wire_api, route).is_some() {
                return Err(ApiMapError::DuplicateRoute {
                    provider_id: id.clone(),
                    wire_api,
                });
            }
        }

        for wire in &required_wires {
            let route = route_map
                .get(wire)
                .ok_or_else(|| ApiMapError::MissingRoute {
                    provider_id: id.clone(),
                    wire_api: *wire,
                })?;
            if route.id() != id {
                return Err(ApiMapError::RouteProviderIdMismatch {
                    provider_id: id.clone(),
                    wire_api: *wire,
                    route_provider_id: route.id().into(),
                });
            }
            let expected: BTreeSet<&str> = models
                .iter()
                .filter(|model| model.wire_api == *wire)
                .map(|model| model.id.as_str())
                .collect();
            let actual: BTreeSet<&str> = route
                .models()
                .iter()
                .map(|model| model.id.as_str())
                .collect();
            if route.models().len() != actual.len()
                || route.models().iter().any(|route_model| {
                    route_model.wire_api != *wire
                        || !models.iter().any(|model| model == route_model)
                })
                || actual != expected
            {
                return Err(ApiMapError::RouteCatalogMismatch {
                    provider_id: id.clone(),
                    wire_api: *wire,
                });
            }
        }
        if let Some(extra) = route_map
            .keys()
            .find(|wire| !required_wires.contains(wire))
            .copied()
        {
            return Err(ApiMapError::UnexpectedRoute {
                provider_id: id.clone(),
                wire_api: extra,
            });
        }

        Ok(Self {
            id,
            models,
            routes: route_map,
        })
    }

    fn model_id<'a>(&self, spec: &'a str) -> Option<&'a str> {
        match spec.split_once(':') {
            Some((provider_id, model_id)) if provider_id == self.id => Some(model_id),
            Some(_) => None,
            None => Some(spec),
        }
    }
}

impl Provider for ApiMappedProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, request: Request) -> EventStream {
        let Some(model_id) = self.model_id(&request.model) else {
            let provider_id = self.id.clone();
            let model_id = request.model;
            return Box::pin(stream::once(async move {
                Err(ProviderError::UnknownModel {
                    provider_id,
                    model_id,
                })
            }));
        };
        let Some(model) = self.models.iter().find(|model| model.id == model_id) else {
            let provider_id = self.id.clone();
            let model_id = model_id.to_owned();
            return Box::pin(stream::once(async move {
                Err(ProviderError::UnknownModel {
                    provider_id,
                    model_id,
                })
            }));
        };
        if let Err(error) =
            crate::provider::validate_request_for_model(&self.id, Some(model), &request)
        {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        self.routes[&model.wire_api].stream(request)
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        if models.is_empty() {
            return Err(ProviderError::Config(format!(
                "mapped provider '{}' requires at least one model",
                self.id
            )));
        }
        let mut ids = BTreeSet::new();
        for model in &models {
            if model.id.trim().is_empty() || !ids.insert(model.id.as_str()) {
                return Err(ProviderError::Config(format!(
                    "mapped provider '{}' has an invalid or duplicate model id",
                    self.id
                )));
            }
            model
                .validate()
                .map_err(|error| ProviderError::Config(error.to_string()))?;
            if !self.routes.contains_key(&model.wire_api) {
                return Err(ProviderError::Config(format!(
                    "mapped provider '{}' has no route for {}",
                    self.id, model.wire_api
                )));
            }
        }
        for (wire, route) in &mut self.routes {
            let subset = models
                .iter()
                .filter(|model| model.wire_api == *wire)
                .cloned()
                .collect::<Vec<_>>();
            if subset.is_empty() {
                return Err(ProviderError::Config(format!(
                    "mapped provider '{}' would leave route {wire} empty",
                    self.id
                )));
            }
            route.replace_model_catalog(subset)?;
        }
        self.models = models;
        Ok(())
    }

    fn refresh_models(&self) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>> {
        Box::pin(async { Ok(None) })
    }
}

/// Invalid mapped-provider catalog or route graph.
#[derive(Debug, thiserror::Error)]
pub enum ApiMapError {
    #[error("mapped provider id cannot be empty")]
    EmptyProviderId,
    #[error("mapped provider '{provider_id}' requires at least one model")]
    EmptyCatalog { provider_id: String },
    #[error("mapped provider '{provider_id}' has an empty model id")]
    EmptyModelId { provider_id: String },
    #[error("mapped provider '{provider_id}' has duplicate model '{model_id}'")]
    DuplicateModel {
        provider_id: String,
        model_id: String,
    },
    #[error("mapped provider '{provider_id}' declares duplicate route {wire_api}")]
    DuplicateRoute {
        provider_id: String,
        wire_api: WireApi,
    },
    #[error(
        "mapped provider '{provider_id}' model '{model_id}' failed compatibility validation: {source}"
    )]
    InvalidModel {
        provider_id: String,
        model_id: String,
        #[source]
        source: ModelInfoError,
    },
    #[error("mapped provider '{provider_id}' has no route for {wire_api}")]
    MissingRoute {
        provider_id: String,
        wire_api: WireApi,
    },
    #[error(
        "mapped provider '{provider_id}' route {wire_api} exposes hidden provider id '{route_provider_id}'"
    )]
    RouteProviderIdMismatch {
        provider_id: String,
        wire_api: WireApi,
        route_provider_id: String,
    },
    #[error("mapped provider '{provider_id}' route {wire_api} has a mismatched catalog subset")]
    RouteCatalogMismatch {
        provider_id: String,
        wire_api: WireApi,
    },
    #[error("mapped provider '{provider_id}' declares unexpected route {wire_api}")]
    UnexpectedRoute {
        provider_id: String,
        wire_api: WireApi,
    },
}
