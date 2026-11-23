//! Cross-owner request preparation for the [`Models`](crate::models::Models) facade.

use super::*;

impl Models {
    /// Resolve one canonical `provider:model` route, validate the request,
    /// resolve authentication once, and freeze an opaque
    /// [`PreparedProviderCall`] (advanced entry) with the default acquisition
    /// policy (zero extra sends).
    ///
    /// Equivalent to
    /// [`prepare_call_with_policy`](Self::prepare_call_with_policy) with
    /// [`AcquisitionPolicy::default`](crate::retry::AcquisitionPolicy::default).
    pub async fn prepare_call(
        &self,
        spec: &str,
        request: Request,
    ) -> Result<PreparedProviderCall, ModelsError> {
        // Ordinary entries freeze the provider's protocol-default
        // acquisition policy (helper/Codex wires default to zero extra
        // sends; Bedrock freezes its resolved Smithy configuration).
        let (provider_ref, _model) = self.registry.registry.resolve(spec)?;
        let policy = provider_ref.acquisition_policy_default();
        self.prepare_call_with_policy(spec, request, policy).await
    }

    /// Resolve one canonical `provider:model` route, validate the request,
    /// resolve authentication once, and freeze an opaque
    /// [`PreparedProviderCall`] (advanced entry) with an explicit per-call
    /// acquisition policy.
    ///
    /// Sequential attempts from the prepared call reuse the same frozen route,
    /// request, and authentication without repeating preparation. Route lookup,
    /// capability/wire validation, and auth preparation each precede
    /// model-request dispatch in that order; a failure at any step returns a
    /// typed error without selecting another provider, model, wire, credential
    /// policy, or local implementation. Ordinary entries reuse this exact
    /// compiler and auth resolution through [`ChatCall`].
    /// Pi Messages preserves its source passthrough options. On other wires,
    /// an omitted output limit resolves to the selected registry model's output
    /// capacity before authentication. An enabled thinking budget must fit
    /// below that default limit. Implicit and explicit ceilings are then clamped
    /// to remaining context with 4,096 safety tokens (minimum one output token).
    /// Thinking that no longer fits is reduced to leave 1,024 answer tokens
    /// where possible. Numeric Claude combines the answer ceiling and thinking
    /// budget before applying the model and remaining-context ceilings. The
    /// caller's request is not mutated.
    pub async fn prepare_call_with_policy(
        &self,
        spec: &str,
        mut request: Request,
        policy: crate::retry::AcquisitionPolicy,
    ) -> Result<PreparedProviderCall, ModelsError> {
        let (provider_ref, model) = self.registry.registry.resolve(spec)?;
        request.callbacks = request.callbacks.for_call();
        let provider_id = provider_ref.id().to_owned();
        let model_id = model.id.clone();
        let wire_api = model.wire_api;
        let resource_owner = self.registry.resource_owners.get(&provider_id).cloned();
        let resources = {
            let _registration = self.registry.registration_gate.lock().unwrap();
            resource_owner
                .as_ref()
                .and_then(|owner| owner.register_call(&request))
        };
        request.cancel = resources
            .as_ref()
            .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
        if request.cancel.is_cancelled() {
            return Err(ModelsError::CallCancelled);
        }
        let canonical_request_model = format!("{provider_id}:{model_id}");
        if request.model != canonical_request_model {
            return Err(ModelsError::RequestRouteMismatch {
                request_model: request.model.clone(),
                route_provider: provider_id,
                route_model: model_id,
            });
        }
        compile_prepared_request(provider_ref, model, &mut request)?;
        let resolver = self
            .auth_owner
            .resolvers
            .get(&provider_id)
            .ok_or_else(|| ModelsError::RouteNotDispatchable {
                provider: provider_id.clone(),
            })?
            .clone();
        let provider = self
            .registry
            .registry
            .get_provider_arc(&provider_id)
            .ok_or_else(|| {
                ModelsError::Registry(RegistryError::UnknownProvider(provider_id.clone()))
            })?;
        let source = self
            .auth_owner
            .sources
            .get(&provider_id)
            .cloned()
            .unwrap_or(AuthProvenanceSource::Static);
        let projection = crate::transcript::compile_request_projection(
            &request,
            &provider_id,
            Some(model),
            model.wire_api,
        )?
        .facts;

        let mut auth = resolve_route_auth(&resolver, source, &request.cancel)
            .await
            .map_err(|error| {
                let call_cancelled = matches!(error, ModelsError::CallCancelled);
                let cause = models_error_cause(error).with_projection(&projection);
                ModelsError::Provider(if call_cancelled {
                    cause.with_call_cancellation()
                } else {
                    cause
                })
            })?;
        let mut selected_model = model.clone();
        let provider = provider
            .prepare_model_call(&mut selected_model, &mut auth)
            .map_err(|error| ModelsError::Provider(error.with_projection(&projection)))?
            .unwrap_or(provider);
        let route = PreparedRoute {
            projection,
            provider_id,
            model_id,
            wire_api,
        };
        Ok(PreparedProviderCall::new(
            PreparedDispatch {
                provider,
                selected_model: Some(Box::new(selected_model)),
                simple_options: SimpleOptions::default(),
            },
            request,
            auth,
            route,
            policy,
            resources,
            resource_owner,
        ))
    }
}
