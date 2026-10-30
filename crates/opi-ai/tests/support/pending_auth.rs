use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Default)]
pub struct PendingAuth {
    pub calls: AtomicU32,
    pub released: Arc<AtomicBool>,
}

impl opi_ai::auth::AuthResolver for PendingAuth {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::BoxAuthFuture<
        'a,
        Result<opi_ai::auth::ResolvedAuth, opi_ai::provider::ProviderError>,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            struct Release(Arc<AtomicBool>);
            impl Drop for Release {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }
            let _release = Release(self.released.clone());
            std::future::pending().await
        })
    }
}
