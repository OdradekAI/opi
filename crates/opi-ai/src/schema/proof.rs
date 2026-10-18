//! A certificate for one repeated admission prefix.
//!
//! URLs carry a symbolic directory orbit only during certificate replay. Every
//! URI branch must have the same result for every future iteration. Cache
//! membership is checked against both the old finite cache and all earlier
//! iterations of the recorded prefix, including exact cross-iteration lags.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashSet};
use std::ops::Deref;
use std::rc::Rc;

use serde_json::Value;
use url::Url;

use super::SchemaError;
use super::pump::{CacheEquality, Equality, EqualitySchedule, PathPump, UrlOrbit};

pub(crate) type FunctionKey = (*const Value, String);

/// The URL at iteration zero, and its exact variation in later iterations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Orbit {
    pub(crate) value: Url,
    pub(crate) pump: Option<PathPump>,
}

impl Orbit {
    pub(crate) fn fixed(value: Url) -> Self {
        Self { value, pump: None }
    }

    pub(crate) fn growing(value: Url, pump: PathPump) -> Option<Self> {
        pump.growing(value.clone())?;
        Some(Self {
            value,
            pump: Some(pump),
        })
    }

    pub(crate) fn between(before: &Url, after: &Url, preferred: &PathPump) -> Option<Self> {
        if before == after {
            return Some(Self::fixed(before.clone()));
        }
        let pump = if preferred.next(before).as_ref() == Some(after) {
            preferred.clone()
        } else {
            PathPump::from_pair(before, after)?
        };
        Self::growing(before.clone(), pump)
    }

    pub(crate) fn next(&self) -> Option<Self> {
        Some(Self {
            value: match &self.pump {
                Some(pump) => pump.next(&self.value)?,
                None => self.value.clone(),
            },
            pump: self.pump.clone(),
        })
    }

    pub(crate) fn join(&self, input: &str) -> Option<Self> {
        let Some(pump) = &self.pump else {
            return Some(Self::fixed(self.value.join(input).ok()?));
        };
        let output = pump.join_literal(&pump.growing(self.value.clone())?, input)?;
        Some(Self {
            value: output.base,
            pump: output.growing.then(|| pump.clone()),
        })
    }

    fn under(&self, pump: &PathPump) -> Option<UrlOrbit> {
        let Some(own) = &self.pump else {
            return Some(UrlOrbit::fixed(self.value.clone()));
        };
        let candidate = pump.growing(self.value.clone())?;
        PathPump::prove_mixed_equality(
            own,
            &own.growing(self.value.clone())?,
            pump,
            &candidate,
            Equality::Full,
        )?
        .then_some(candidate)
    }

    pub(crate) fn equivalent(&self, other: &Self) -> bool {
        self.equal(other, Equality::Full) == Some(true)
    }

    fn equal(&self, other: &Self, kind: Equality) -> Option<bool> {
        match (&self.pump, &other.pump) {
            (None, None) => Some(match kind {
                Equality::Full => self.value == other.value,
                Equality::Path => self.value.path() == other.value.path(),
            }),
            (Some(left), Some(right)) => PathPump::prove_mixed_equality(
                left,
                &left.growing(self.value.clone())?,
                right,
                &right.growing(other.value.clone())?,
                kind,
            ),
            (Some(pump), None) => pump.prove_equality(
                &pump.growing(self.value.clone())?,
                &UrlOrbit::fixed(other.value.clone()),
                kind,
            ),
            (None, Some(pump)) => pump.prove_equality(
                &UrlOrbit::fixed(self.value.clone()),
                &pump.growing(other.value.clone())?,
                kind,
            ),
        }
    }
}

#[derive(Clone)]
struct KeyOrbit {
    node: *const Value,
    orbit: UrlOrbit,
}

struct Query {
    key: KeyOrbit,
    hit: bool,
    inserted_before: usize,
}

pub(crate) struct ProofContext {
    pub(crate) pump: PathPump,
    failed: Cell<bool>,
    queries: RefCell<Vec<Query>>,
    inserted: RefCell<Vec<KeyOrbit>>,
}

impl ProofContext {
    pub(crate) fn new(pump: PathPump) -> Rc<Self> {
        Rc::new(Self {
            pump,
            failed: Cell::new(false),
            queries: RefCell::new(Vec::new()),
            inserted: RefCell::new(Vec::new()),
        })
    }

    pub(crate) fn fail(&self) {
        self.failed.set(true);
    }

    pub(crate) fn failed(&self) -> bool {
        self.failed.get()
    }

    pub(crate) fn query(&self, node: *const Value, url: &ReferenceUrl, hit: bool) {
        let Some(orbit) = url.orbit.as_ref().and_then(|orbit| orbit.under(&self.pump)) else {
            self.fail();
            return;
        };
        let key = KeyOrbit { node, orbit };
        let inserted_before = self.inserted.borrow().len();
        self.queries.borrow_mut().push(Query {
            key: key.clone(),
            hit,
            inserted_before,
        });
        if !hit {
            self.inserted.borrow_mut().push(key);
        }
    }

    /// Cache truth at iteration n is a finite set of isolated points plus an
    /// optional permanent tail. Its endpoints come from exact word equations,
    /// never from a chosen number of warm-up iterations.
    pub(crate) fn cache_stable(&self, old: &HashSet<FunctionKey>) -> bool {
        if self.failed() {
            return false;
        }
        let inserted = self.inserted.borrow();
        self.queries.borrow().iter().all(|query| {
            let mut truth = TruthSet::default();
            for (node, base) in old {
                if *node != query.key.node {
                    continue;
                }
                let Ok(base) = Url::parse(base) else {
                    return false;
                };
                let Some(schedule) = self.pump.equality_schedule(
                    &query.key.orbit,
                    &UrlOrbit::fixed(base),
                    Equality::Full,
                ) else {
                    return false;
                };
                truth.add_schedule(schedule);
            }
            for key in inserted.iter().take(query.inserted_before) {
                if key.node == query.key.node {
                    let Some(schedule) =
                        self.pump
                            .equality_schedule(&query.key.orbit, &key.orbit, Equality::Full)
                    else {
                        return false;
                    };
                    truth.add_schedule(schedule);
                }
            }
            for key in inserted.iter().filter(|key| key.node == query.key.node) {
                let Some(equality) =
                    self.pump
                        .cache_equality(&query.key.orbit, &key.orbit, Equality::Full)
                else {
                    return false;
                };
                match equality {
                    CacheEquality::Never | CacheEquality::KeyAfter(_) => {}
                    CacheEquality::Always => truth.add_tail(1),
                    CacheEquality::QueryAt(n) if n > 0 => {
                        truth.points.insert(n);
                    }
                    CacheEquality::QueryAt(_) => {}
                    CacheEquality::KeyAt(n) => {
                        let Some(start) = n.checked_add(1) else {
                            return false;
                        };
                        truth.add_tail(start);
                    }
                    CacheEquality::QueryAfter(lag) if lag > 0 => truth.add_tail(lag),
                    CacheEquality::QueryAfter(_) => {}
                }
            }
            truth.constant(query.hit)
        })
    }
}

#[derive(Default)]
struct TruthSet {
    tail: Option<usize>,
    points: BTreeSet<usize>,
}

impl TruthSet {
    fn add_tail(&mut self, start: usize) {
        self.tail = Some(self.tail.map_or(start, |old| old.min(start)));
    }

    fn add_schedule(&mut self, schedule: EqualitySchedule) {
        match schedule {
            EqualitySchedule::Always => self.add_tail(0),
            EqualitySchedule::Never => {}
            EqualitySchedule::At(n) => {
                self.points.insert(n);
            }
        }
    }

    fn constant(&self, expected: bool) -> bool {
        if !expected {
            return self.tail.is_none() && self.points.is_empty();
        }
        let Some(start) = self.tail else {
            return false;
        };
        // A finite set cannot cover a missing prefix of a permanent tail.
        self.points.range(..start).copied().eq(0..start)
    }
}

/// The resolver's URL. Proof metadata is absent on ordinary execution; replay
/// uses the very same resolver and records any URI decision it cannot certify.
#[derive(Clone)]
pub(crate) struct ReferenceUrl {
    actual: Url,
    pub(crate) orbit: Option<Orbit>,
    context: Option<Rc<ProofContext>>,
}

impl ReferenceUrl {
    pub(crate) fn parse(text: &str) -> Result<Self, SchemaError> {
        Url::parse(text)
            .map(|actual| Self {
                actual,
                orbit: None,
                context: None,
            })
            .map_err(|error| SchemaError::Uri {
                detail: error.to_string(),
            })
    }

    pub(crate) fn proven(orbit: Orbit, context: Rc<ProofContext>) -> Self {
        Self {
            actual: orbit.value.clone(),
            orbit: Some(orbit),
            context: Some(context),
        }
    }

    pub(crate) fn join(&self, input: &str) -> Result<Self, SchemaError> {
        let actual = self.actual.join(input).map_err(|error| SchemaError::Uri {
            detail: error.to_string(),
        })?;
        let orbit = self.context.as_ref().and_then(|context| {
            let orbit = self.orbit.as_ref().and_then(|orbit| orbit.join(input));
            if orbit.is_none() {
                context.fail();
            }
            orbit
        });
        Ok(Self {
            actual,
            orbit,
            context: self.context.clone(),
        })
    }

    /// A serialized parsed hierarchical URL is an absolute reference. Its
    /// path orbit is independent of the base used for the source's new URL.
    pub(crate) fn join_url(&self, input: &Self) -> Result<Self, SchemaError> {
        let actual = self
            .actual
            .join(input.actual.as_str())
            .map_err(|error| SchemaError::Uri {
                detail: error.to_string(),
            })?;
        let context = self.context.clone().or_else(|| input.context.clone());
        let orbit = context.as_ref().and_then(|context| {
            let orbit = input.orbit.clone().or_else(|| {
                input
                    .context
                    .is_none()
                    .then(|| Orbit::fixed(input.actual.clone()))
            });
            if actual != input.actual || orbit.is_none() {
                context.fail();
            }
            orbit
        });
        Ok(Self {
            actual,
            orbit,
            context,
        })
    }

    pub(crate) fn without_fragment(&self) -> Self {
        let mut result = self.clone();
        result.actual.set_fragment(None);
        if let Some(orbit) = &mut result.orbit {
            orbit.value.set_fragment(None);
        }
        result
    }

    pub(crate) fn equal(&self, other: &Self, kind: Equality) -> bool {
        let actual = match kind {
            Equality::Full => self.actual == other.actual,
            Equality::Path => self.actual.path() == other.actual.path(),
        };
        if let Some(context) = self.context.as_ref().or(other.context.as_ref()) {
            let left = self.orbit.clone().or_else(|| {
                self.context
                    .is_none()
                    .then(|| Orbit::fixed(self.actual.clone()))
            });
            let right = other.orbit.clone().or_else(|| {
                other
                    .context
                    .is_none()
                    .then(|| Orbit::fixed(other.actual.clone()))
            });
            if left
                .zip(right)
                .and_then(|(left, right)| left.equal(&right, kind))
                != Some(actual)
            {
                context.fail();
            }
        }
        actual
    }

    pub(crate) fn actual(&self) -> &Url {
        &self.actual
    }
}

impl Deref for ReferenceUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.actual
    }
}

impl std::fmt::Display for ReferenceUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.actual.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(path: &str) -> Url {
        Url::parse(&format!("https://offline.invalid/{path}")).unwrap()
    }

    fn context() -> Rc<ProofContext> {
        ProofContext::new(PathPump::from_pair(&url("a/"), &url("a/a/")).unwrap())
    }

    fn growing(context: &Rc<ProofContext>, path: &str) -> ReferenceUrl {
        ReferenceUrl::proven(
            Orbit::growing(url(path), context.pump.clone()).unwrap(),
            context.clone(),
        )
    }

    #[test]
    fn old_cache_keys_can_create_a_late_hit() {
        let node = Value::Null;
        let context = context();
        context.query(&node, &growing(&context, "a/"), false);
        let old = HashSet::from([(&node as *const Value, url("a/a/a/").to_string())]);
        assert!(!context.cache_stable(&old));
    }

    #[test]
    fn cross_iteration_lag_is_not_a_two_sample_warmup() {
        let node = Value::Null;
        let context = context();
        // The shorter query misses in the first two iterations, then hits
        // the longer key inserted two iterations earlier.
        context.query(&node, &growing(&context, "a/a/a/"), false);
        context.query(&node, &growing(&context, "a/"), false);
        assert!(!context.cache_stable(&HashSet::new()));
    }

    #[test]
    fn same_iteration_reuse_is_permanent() {
        let node = Value::Null;
        let context = context();
        let key = growing(&context, "a/");
        context.query(&node, &key, false);
        context.query(&node, &key, true);
        assert!(context.cache_stable(&HashSet::new()));
    }

    #[test]
    fn finite_old_keys_can_bridge_a_proved_cache_lag() {
        let node = Value::Null;
        let context = context();
        context.query(&node, &growing(&context, "a/"), true);
        context.query(&node, &growing(&context, "a/a/a/"), false);
        let old = HashSet::from([
            (&node as *const Value, url("a/").to_string()),
            (&node as *const Value, url("a/a/").to_string()),
        ]);
        assert!(context.cache_stable(&old));
    }

    #[test]
    fn a_new_constant_key_changes_the_next_iteration() {
        let node = Value::Null;
        let first = context();
        let key = ReferenceUrl::proven(Orbit::fixed(url("fixed")), first.clone());
        first.query(&node, &key, false);
        assert!(!first.cache_stable(&HashSet::new()));

        let warmed = context();
        let key = ReferenceUrl::proven(Orbit::fixed(url("fixed")), warmed.clone());
        warmed.query(&node, &key, true);
        let old = HashSet::from([(&node as *const Value, url("fixed").to_string())]);
        assert!(warmed.cache_stable(&old));
    }

    #[test]
    fn cache_membership_keeps_schema_identity() {
        let node = Value::Null;
        let other = Value::Null;
        let context = context();
        context.query(&node, &growing(&context, "a/"), false);
        let old = HashSet::from([(&other as *const Value, url("a/a/a/").to_string())]);
        assert!(context.cache_stable(&old));
    }
}
