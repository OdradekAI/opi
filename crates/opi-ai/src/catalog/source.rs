//! Custom catalog source contract, independent of any transport backend.

use super::ModelsStoreEntry;
use crate::errors::ProviderError;
use futures_util::future::BoxFuture;
use std::collections::{BTreeSet, HashMap};
use tokio_util::sync::CancellationToken;

/// The selected owners and caller policy for one explicit catalog refresh.
/// Sources own freshness interpretation; the SDK forwards `force` unchanged.
#[derive(Clone, Debug)]
pub struct CatalogRefreshContext {
    /// Whether the caller permits source network work.
    pub allow_network: bool,
    /// An optional source-specific forced-refresh choice.
    pub force: Option<bool>,
    /// Active provider owners selected for this source invocation.
    pub providers: BTreeSet<String>,
    /// Cancellation of this refresh, including its publication.
    pub cancel: CancellationToken,
}

/// A source of complete dynamic catalogs, queried during an explicit
/// network-enabled refresh round.
///
/// Implementations must be cheap to clone or wrapped in an `Arc` when
/// registered. The context selects active declared provider owners and forwards
/// optional source-specific freshness policy. Fetches observe cancellation;
/// only Models publishes their acknowledged selected records. Undeclared
/// provider output is rejected, while declared unselected output is ignored.
pub trait DynamicCatalogSource: Send + Sync {
    /// Stable identifier reported in refresh outcomes.
    fn id(&self) -> &str;

    /// Provider owners whose records this source may return. Construction and
    /// this declaration perform no source work.
    fn provider_ids(&self) -> &[String];

    /// Fetch the complete dynamic catalogs this source publishes, keyed by
    /// provider id. An error fails only this source: the refresh report
    /// records it and prior data for its providers is retained.
    fn fetch(
        &self,
        context: CatalogRefreshContext,
    ) -> BoxFuture<'static, Result<HashMap<String, ModelsStoreEntry>, ProviderError>>;
}
