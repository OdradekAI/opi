//! The source compiler's structural and reference-function walk.
//!
//! Function cache hits preserve their preceding resolver side effects. A
//! function's reference bindings and structural children are frozen here;
//! instance-dependent traversal cannot rebuild a different reference frame.
//! Repeated source prefixes are rejected only after a symbolic replay proves
//! that every URI decision and cache query will repeat forever.

use std::collections::HashMap;
use std::rc::Rc;

use serde_json::Value;

use super::SchemaError;
use super::guards;
use super::ir::{PlanChild, PlanId, PlanNode, RefBinding, RegexSlot, schema_children};
use super::proof::{FunctionKey, ProofContext};
use super::pump::PathPump;
use super::refs::RefStack;

#[derive(Clone, Copy)]
enum Visit<'a> {
    Enter {
        schema: &'a Value,
        plan: PlanId,
        function: bool,
    },
    Leave(&'a Value),
    LeaveFunction(&'a Value),
    Reference {
        owner: &'a Value,
        plan: PlanId,
        reference: &'a str,
        kind: usize,
    },
}

impl Visit<'_> {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Enter {
                    schema: a,
                    plan: ap,
                    function: af,
                },
                Self::Enter {
                    schema: b,
                    plan: bp,
                    function: bf,
                },
            ) => std::ptr::eq(*a, *b) && ap == bp && af == bf,
            (Self::Leave(a), Self::Leave(b)) | (Self::LeaveFunction(a), Self::LeaveFunction(b)) => {
                std::ptr::eq(*a, *b)
            }
            (
                Self::Reference {
                    owner: a,
                    plan: ap,
                    reference: ar,
                    kind: ak,
                },
                Self::Reference {
                    owner: b,
                    plan: bp,
                    reference: br,
                    kind: bk,
                },
            ) => std::ptr::eq(*a, *b) && ap == bp && ar == br && ak == bk,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct ReferenceKey {
    owner: *const Value,
    target: *const Value,
    kind: usize,
}

#[derive(Clone)]
struct Machine<'a> {
    root: &'a Value,
    refs: RefStack<'a>,
    work: Vec<Visit<'a>>,
    functions: HashMap<FunctionKey, PlanId>,
    plans: Vec<PlanNode>,
    active_functions: HashMap<*const Value, usize>,
    regexes: &'a HashMap<*const Value, RegexSlot>,
    locations: &'a HashMap<*const Value, usize>,
    proof: Option<Rc<ProofContext>>,
}

impl<'a> Machine<'a> {
    fn plan(&mut self, schema: &'a Value) -> Result<PlanId, SchemaError> {
        let schema_location =
            *self
                .locations
                .get(&(schema as *const Value))
                .ok_or(SchemaError::Internal(
                    "admission schema location is missing",
                ))?;
        let id = self.plans.len();
        self.plans.push(PlanNode {
            schema_location,
            children: Vec::new(),
            refs: std::array::from_fn(|_| RefBinding::Unresolved),
        });
        Ok(id)
    }

    fn child(
        &mut self,
        parent: PlanId,
        keyword: &'static str,
        schema: &'a Value,
    ) -> Result<Visit<'a>, SchemaError> {
        let plan = self.plan(schema)?;
        self.plans[parent].children.push(PlanChild {
            keyword,
            schema: schema as *const Value,
            plan,
        });
        Ok(Visit::Enter {
            schema,
            plan,
            function: false,
        })
    }

    fn step(&mut self) -> Result<Option<ReferenceKey>, SchemaError> {
        let Some(visit) = self.work.pop() else {
            return Ok(None);
        };
        match visit {
            Visit::Leave(schema) => self.refs.pop(schema),
            Visit::LeaveFunction(schema) => {
                let key = schema as *const Value;
                let count = self
                    .active_functions
                    .get_mut(&key)
                    .ok_or(SchemaError::Internal("admission function is not active"))?;
                *count -= 1;
                if *count == 0 {
                    self.active_functions.remove(&key);
                }
            }
            Visit::Enter {
                schema,
                plan,
                function,
            } => {
                if schema.is_null() || schema.is_number() || schema.is_string() {
                    return Err(SchemaError::InvalidSchema {
                        path: schema_path(self.root, schema)?,
                    });
                }
                if function {
                    *self
                        .active_functions
                        .entry(schema as *const Value)
                        .or_default() += 1;
                    self.work.push(Visit::LeaveFunction(schema));
                }
                self.refs.push(schema);
                self.work.push(Visit::Leave(schema));
                if let Some(slot) = self.regexes.get(&(schema as *const Value)) {
                    slot.admit(schema)?;
                }
                let (before, after) = schema_children(schema);
                for (keyword, child) in after.into_iter().rev() {
                    let visit = self.child(plan, keyword, child)?;
                    self.work.push(visit);
                }
                for (keyword, kind) in [("$dynamicRef", 2), ("$recursiveRef", 1), ("$ref", 0)] {
                    if let Some(reference) = guards::get_str(schema, keyword) {
                        self.work.push(Visit::Reference {
                            owner: schema,
                            plan,
                            reference,
                            kind,
                        });
                    }
                }
                for (keyword, child) in before.into_iter().rev() {
                    let visit = self.child(plan, keyword, child)?;
                    self.work.push(visit);
                }
            }
            Visit::Reference {
                owner,
                plan,
                reference,
                kind,
            } => {
                let target = match kind {
                    0 => self.refs.apply_ref(reference),
                    1 => self.refs.apply_recursive_ref(reference),
                    _ => self.refs.apply_dynamic_ref(reference),
                };
                let target = match target {
                    // The source's nullish fallback is the false schema. Keep
                    // resolver effects, but do not build the null target.
                    Ok(Some(Value::Null)) => return Ok(None),
                    Ok(Some(target)) => target,
                    Ok(None) => return Ok(None),
                    Err(error) => {
                        self.plans[plan].refs[kind] = RefBinding::Error(error);
                        return Ok(None);
                    }
                };
                let base = match self.refs.lexical_url() {
                    Ok(base) => base,
                    Err(error) => {
                        self.plans[plan].refs[kind] = RefBinding::Error(error);
                        return Ok(None);
                    }
                };
                let key = (target as *const Value, base.to_string());
                let existing = self.functions.get(&key).copied();
                if let Some(proof) = &self.proof {
                    proof.query(target as *const Value, &base, existing.is_some());
                }
                let target_plan = match existing {
                    Some(plan) => plan,
                    None => {
                        let plan = self.plan(target)?;
                        self.functions.insert(key, plan);
                        self.work.push(Visit::Enter {
                            schema: target,
                            plan,
                            function: true,
                        });
                        plan
                    }
                };
                self.plans[plan].refs[kind] = RefBinding::Target(target_plan);
                if existing.is_none()
                    && self
                        .active_functions
                        .contains_key(&(target as *const Value))
                {
                    return Ok(Some(ReferenceKey {
                        owner: owner as *const Value,
                        target: target as *const Value,
                        kind,
                    }));
                }
            }
        }
        Ok(None)
    }
}

fn schema_path(root: &Value, target: &Value) -> Result<String, SchemaError> {
    let mut work = vec![(root, String::new())];
    while let Some((value, path)) = work.pop() {
        if std::ptr::eq(value, target) {
            return Ok(path);
        }
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    let token = key.replace('~', "~0").replace('/', "~1");
                    work.push((child, format!("{path}/{token}")));
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    work.push((child, format!("{path}/{index}")));
                }
            }
            _ => {}
        }
    }
    Err(SchemaError::Internal("invalid schema location is missing"))
}

fn proves_infinite(before: &Machine<'_>, after: &Machine<'_>, steps: usize) -> bool {
    let Some(top) = before.work.last() else {
        return false;
    };
    let Some(next_top) = after.work.last() else {
        return false;
    };
    if !matches!(top, Visit::Enter { function: true, .. })
        || !matches!(next_top, Visit::Enter { function: true, .. })
    {
        return false;
    }
    let prefix_len = before.work.len() - 1;
    if after.work.len() <= prefix_len
        || !before.work[..prefix_len]
            .iter()
            .zip(&after.work)
            .all(|(a, b)| a.same(b))
    {
        return false;
    }
    let (Ok(left), Ok(right)) = (before.refs.lexical_url(), after.refs.lexical_url()) else {
        return false;
    };
    let Some(pump) = PathPump::from_pair(left.actual(), right.actual()) else {
        return false;
    };
    let context = ProofContext::new(pump);
    let Some(refs) = before.refs.proof_replay(&after.refs, context.clone()) else {
        return false;
    };
    let mut replay = before.clone();
    replay.refs = refs;
    replay.proof = Some(context.clone());
    // This is the measured length of an already executed prefix, not fuel.
    // Failure to certify any step resumes the ordinary source builder.
    for _ in 0..steps {
        if replay.work.len() <= prefix_len || replay.step().is_err() || context.failed() {
            return false;
        }
        if !before.work[..prefix_len]
            .iter()
            .zip(&replay.work)
            .all(|(a, b)| a.same(b))
        {
            return false;
        }
    }
    replay.work.len() == after.work.len()
        && replay.work.iter().zip(&after.work).all(|(a, b)| a.same(b))
        && replay.refs.proof_closed()
        && context.cache_stable(&before.functions.keys().cloned().collect())
}

pub(super) fn admit(
    root: &Value,
    regexes: &HashMap<*const Value, RegexSlot>,
    locations: &HashMap<*const Value, usize>,
) -> Result<Vec<PlanNode>, SchemaError> {
    let context = serde_json::Map::new();
    let refs = RefStack::new(&context, root);
    let root_base = refs.lexical_base_url()?;
    let mut machine = Machine {
        root,
        refs,
        work: Vec::new(),
        functions: HashMap::new(),
        plans: Vec::new(),
        active_functions: HashMap::new(),
        regexes,
        locations,
        proof: None,
    };
    let root_plan = machine.plan(root)?;
    machine
        .functions
        .insert((root as *const Value, root_base), root_plan);
    machine.work.push(Visit::Enter {
        schema: root,
        plan: root_plan,
        function: true,
    });
    let mut checkpoints: HashMap<ReferenceKey, (u128, Machine<'_>)> = HashMap::new();
    let mut count = 0_u128;
    while !machine.work.is_empty() {
        let key = machine.step()?;
        count += 1;
        if let Some(key) = key {
            if let Some((old_count, before)) = checkpoints.get(&key)
                && let Ok(steps) = usize::try_from(count - old_count)
                && proves_infinite(before, &machine, steps)
            {
                return Err(SchemaError::NoProgress);
            }
            checkpoints.insert(key, (count, machine.clone()));
        }
    }
    Ok(machine.plans)
}
