//! Shared in-memory credential store fixture for OAuth boundary targets.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
use opi_ai::mutation::{MutationOutcome, MutationReceipt};

/// Store mutation outcome injected by Models store tests.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum StoreMode {
    #[default]
    Ok,
    Fail,
    Unknown,
}

/// In-memory `CredentialStore` that records ACK outcomes and serialization.
#[derive(Default)]
pub struct FakeStore {
    pub entries: Mutex<HashMap<String, Credential>>,
    pub mode: Mutex<StoreMode>,
    gate: tokio::sync::Mutex<()>,
    pub modify_log: Mutex<Vec<String>>,
    pub modify_starts: Mutex<usize>,
    pub modify_ends: Mutex<usize>,
    pub max_overlapping: Mutex<usize>,
}

impl FakeStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn set_mode(&self, mode: StoreMode) {
        *self.mode.lock().unwrap() = mode;
    }

    pub fn entry(&self, provider_id: &str) -> Option<Credential> {
        self.entries.lock().unwrap().get(provider_id).cloned()
    }
}

impl CredentialStore for FakeStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move { Ok(self.entries.lock().unwrap().get(provider_id).cloned()) })
    }

    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), cred.clone());
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries.lock().unwrap().remove(provider_id);
            Ok(())
        })
    }

    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move {
            if self.entries.lock().unwrap().contains_key(provider_id) {
                CredentialSource::Present {
                    label: format!("test store {provider_id}"),
                }
            } else {
                CredentialSource::Absent
            }
        })
    }

    fn modify<'a>(
        &'a self,
        provider_id: &'a str,
        op: opi_ai::mutation::CredentialMutationOp,
    ) -> BoxAuthFuture<'a, Result<MutationReceipt, CredentialStoreError>> {
        Box::pin(async move {
            let _gate = self.gate.lock().await;
            let provider_id = provider_id.to_owned();
            {
                let mut starts = self.modify_starts.lock().unwrap();
                *starts += 1;
                let overlapping = *starts - *self.modify_ends.lock().unwrap();
                let mut max = self.max_overlapping.lock().unwrap();
                if overlapping > *max {
                    *max = overlapping;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
            let mode = *self.mode.lock().unwrap();
            self.modify_log.lock().unwrap().push(provider_id.clone());
            let revision = (self.entries.lock().unwrap().len() + 1) as u64;
            let outcome = match mode {
                StoreMode::Ok => match op {
                    opi_ai::mutation::CredentialMutationOp::Write(cred) => {
                        self.entries
                            .lock()
                            .unwrap()
                            .insert(provider_id.clone(), cred);
                        MutationOutcome::Written { revision }
                    }
                    opi_ai::mutation::CredentialMutationOp::Delete => {
                        self.entries.lock().unwrap().remove(&provider_id);
                        MutationOutcome::Deleted { revision }
                    }
                    opi_ai::mutation::CredentialMutationOp::Update(_) => {
                        MutationOutcome::Unchanged { revision }
                    }
                },
                StoreMode::Fail => MutationOutcome::Failed {
                    error: Arc::new(opi_ai::mutation::MutationError::Store(
                        CredentialStoreError::Backend {
                            provider: provider_id.clone(),
                            reason: "injected failure".into(),
                        },
                    )),
                },
                StoreMode::Unknown => MutationOutcome::Unknown {
                    detail: "injected unknown".into(),
                },
            };
            *self.modify_ends.lock().unwrap() += 1;
            Ok(MutationReceipt {
                provider_id,
                generation: 0,
                outcome,
            })
        })
    }
}
