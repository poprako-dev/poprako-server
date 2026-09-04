//! Linear-time linked-sequence planning for Unit edits.

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::trl;

use crate::model::read::proj::unit::UnitOrder;
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::Patch;
use crate::value::unit::MAX_PAGE_UNIT_COUNT;

// Traversed IDs and their final successor mapping.
type UnitSequenceState<'a> = (Vec<&'a str>, HashMap<&'a str, Option<&'a str>>);

/// One persisted Unit successor changed by an edit sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitSuccessorChange<'a> {
    //
    /// Permanent Unit ID.
    id: &'a str,

    /// Final successor ID, or none for the tail.
    next_id: Option<&'a str>,
}

impl<'a> UnitSuccessorChange<'a> {
    /// Returns the permanent Unit ID.
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// Returns the final successor ID, or none for the tail.
    pub const fn next_id(&self) -> Option<&'a str> {
        self.next_id
    }
}

/// Final linked-list state produced from normalized Unit edits.
#[derive(Debug)]
pub struct UnitEditSequencePlan<'a> {
    //
    /// Unit IDs in final traversal order, retained only for tests.
    #[cfg(test)]
    ordered_ids: Vec<&'a str>,

    /// Final successor keyed by Unit ID.
    next_ids: HashMap<&'a str, Option<&'a str>>,

    /// Changed successors for existing persisted Units.
    changed_successors: Vec<UnitSuccessorChange<'a>>,

    /// Visible Unit count after applying the edits, retained only for tests.
    #[cfg(test)]
    visible_count: usize,
}

impl<'a> UnitEditSequencePlan<'a> {
    /// Plans one normalized Unit edit batch against a persisted chain.
    pub fn build(
        orders: &'a [UnitOrder],
        edits: &'a [UnitEdit],
    ) -> BaseRest<Self> {
        //
        let mut sequence = UnitSequence::from_orders(orders)?;

        let mut hidden_ids = orders
            .iter()
            .filter(|order| order.is_hidden)
            .map(|order| order.id.as_str())
            .collect::<HashSet<_>>();

        for edit in edits {
            //
            let UnitEdit::Create { id, .. } = edit else {
                continue;
            };

            sequence.append_new(id)?;

            hidden_ids.remove(id.as_str());
        }

        for edit in edits {
            //
            match edit {
                //
                UnitEdit::Create { id, next_id, .. } => {
                    sequence.move_before(id, next_id.as_deref())?;
                }

                UnitEdit::Save { id, next_id, .. } => {
                    //
                    hidden_ids.remove(id.as_str());

                    match next_id {
                        //
                        Patch::Skip => {}

                        Patch::Clear => sequence.move_before(id, None)?,

                        Patch::Assign { value: next_id } => {
                            sequence.move_before(id, Some(next_id))?;
                        }
                    }
                }

                UnitEdit::Delete { id } => {
                    hidden_ids.insert(id);
                }
            }
        }

        let visible_count = sequence
            .nodes
            .keys()
            .filter(|id| !hidden_ids.contains(**id))
            .count();

        validate_visible_count(visible_count)?;

        let (_ordered_ids, next_ids) = sequence.finish()?;

        let mut changed_successors = Vec::new();

        for order in orders {
            //
            let Some(next_id) = next_ids.get(order.id.as_str()).copied() else {
                //
                return Err(invalid_sequence(
                    "persisted Unit is missing from the planned sequence",
                ));
            };

            if next_id != order.next_id.as_deref() {
                //
                changed_successors.push(UnitSuccessorChange {
                    id: &order.id,
                    next_id,
                });
            }
        }

        accept(Self {
            #[cfg(test)]
            ordered_ids: _ordered_ids,
            next_ids,
            changed_successors,
            #[cfg(test)]
            visible_count,
        })
    }

    /// Returns all Unit IDs in their final traversal order.
    #[cfg(test)]
    pub fn ordered_ids(&self) -> &[&'a str] {
        &self.ordered_ids
    }

    /// Returns the final successor for one ID in the planned chain.
    pub fn next_id(&self, id: &str) -> BaseRest<Option<&'a str>> {
        //
        self.next_ids
            .get(id)
            .copied()
            .ok_or_else(|| invalid_sequence("planned Unit ID is missing"))
    }

    /// Returns successor changes for existing persisted Units only.
    pub fn changed_successors(&self) -> &[UnitSuccessorChange<'a>] {
        &self.changed_successors
    }

    /// Returns the number of visible Units after applying the edits.
    #[cfg(test)]
    pub const fn visible_count(&self) -> usize {
        self.visible_count
    }
}

// One mutable node in the linked Unit sequence.
#[derive(Clone, Copy)]
struct SequenceNode<'a> {
    //
    // Predecessor Unit ID, or none for the head.
    prev_id: Option<&'a str>,

    // Successor Unit ID, or none for the tail.
    next_id: Option<&'a str>,
}

// Mutable linked sequence used while applying one normalized edit batch.
struct UnitSequence<'a> {
    //
    // Sequence nodes keyed by Unit ID.
    nodes: HashMap<&'a str, SequenceNode<'a>>,

    // Current head Unit ID.
    head_id: Option<&'a str>,

    // Current tail Unit ID.
    tail_id: Option<&'a str>,
}

impl<'a> UnitSequence<'a> {
    /// Returns one mutable node or classifies a broken internal link.
    pub fn node_mut(&mut self, id: &str) -> BaseRest<&mut SequenceNode<'a>> {
        //
        self.nodes
            .get_mut(id)
            .ok_or_else(|| invalid_sequence("Unit sequence node is missing"))
    }

    /// Returns one copied node or classifies a broken internal link.
    pub fn node(&self, id: &str) -> BaseRest<SequenceNode<'a>> {
        //
        self.nodes
            .get(id)
            .copied()
            .ok_or_else(|| invalid_sequence("Unit sequence node is missing"))
    }

    /// Builds a mutable linked sequence from an ordered persisted chain.
    pub fn from_orders(orders: &'a [UnitOrder]) -> BaseRest<Self> {
        //
        let mut nodes = HashMap::with_capacity(orders.len());

        let mut head_id = None;

        let mut prev_id = None;

        let mut order_iter = orders.iter().peekable();

        while let Some(order) = order_iter.next() {
            //
            let id = order.id.as_str();

            let next_id = order_iter.peek().map(|next| next.id.as_str());

            if order.next_id.as_deref() != next_id {
                //
                return Err(invalid_sequence(
                    "persisted Unit order does not match its successor",
                ));
            }

            if nodes
                .insert(id, SequenceNode { prev_id, next_id })
                .is_some()
            {
                //
                return Err(invalid_sequence(
                    "persisted Unit order contains a duplicate ID",
                ));
            }

            head_id = head_id.or(Some(id));

            prev_id = Some(id);
        }

        accept(Self {
            nodes,
            head_id,
            tail_id: prev_id,
        })
    }

    /// Appends one new ID before applying its requested successor.
    pub fn append_new(&mut self, id: &'a str) -> BaseRest<()> {
        //
        if self.nodes.contains_key(id) {
            return Err(invalid_edit_sequence(id, None, "create"));
        }

        match self.tail_id {
            //
            Some(tail_id) => self.node_mut(tail_id)?.next_id = Some(id),

            None if self.head_id.is_some() => {
                //
                return Err(invalid_sequence(
                    "Unit sequence head exists without a tail",
                ));
            }

            None => self.head_id = Some(id),
        }

        let node = SequenceNode {
            prev_id: self.tail_id,
            next_id: None,
        };

        if self.nodes.insert(id, node).is_some() {
            //
            return Err(invalid_sequence(
                "new Unit unexpectedly replaced a sequence node",
            ));
        }

        self.tail_id = Some(id);

        accept(())
    }

    /// Moves one existing ID immediately before a successor, or to the tail.
    pub fn move_before(
        &mut self,
        id: &'a str,
        next_id: Option<&'a str>,
    ) -> BaseRest<()> {
        //
        if !self.nodes.contains_key(id)
            || next_id.is_some_and(|next_id| {
                next_id == id || !self.nodes.contains_key(next_id)
            })
        {
            return Err(invalid_edit_sequence(id, next_id, "move"));
        }

        self.detach(id)?;

        match next_id {
            //
            Some(next_id) => self.insert_before(id, next_id),

            None => self.insert_at_tail(id),
        }
    }

    /// Detaches one node while preserving its neighbors.
    pub fn detach(&mut self, id: &'a str) -> BaseRest<()> {
        //
        let node = self.node(id)?;

        match node.prev_id {
            //
            Some(prev_id) => self.node_mut(prev_id)?.next_id = node.next_id,

            None if self.head_id == Some(id) => self.head_id = node.next_id,

            None => {
                //
                return Err(invalid_sequence(
                    "Unit sequence has an invalid head link",
                ));
            }
        }

        match node.next_id {
            //
            Some(next_id) => self.node_mut(next_id)?.prev_id = node.prev_id,

            None if self.tail_id == Some(id) => self.tail_id = node.prev_id,

            None => {
                //
                return Err(invalid_sequence(
                    "Unit sequence has an invalid tail link",
                ));
            }
        }

        *self.node_mut(id)? = SequenceNode {
            prev_id: None,
            next_id: None,
        };

        accept(())
    }

    /// Inserts a detached node immediately before an existing anchor.
    pub fn insert_before(
        &mut self,
        id: &'a str,
        next_id: &'a str,
    ) -> BaseRest<()> {
        //
        let prev_id = self.node(next_id)?.prev_id;

        *self.node_mut(id)? = SequenceNode {
            prev_id,
            next_id: Some(next_id),
        };

        self.node_mut(next_id)?.prev_id = Some(id);

        match prev_id {
            //
            Some(prev_id) => self.node_mut(prev_id)?.next_id = Some(id),

            None => self.head_id = Some(id),
        }

        accept(())
    }

    /// Inserts a detached node at the current tail.
    pub fn insert_at_tail(&mut self, id: &'a str) -> BaseRest<()> {
        //
        let prev_id = self.tail_id;

        *self.node_mut(id)? = SequenceNode {
            prev_id,
            next_id: None,
        };

        match prev_id {
            //
            Some(prev_id) => self.node_mut(prev_id)?.next_id = Some(id),

            None => self.head_id = Some(id),
        }

        self.tail_id = Some(id);

        accept(())
    }

    /// Consumes the structure into a validated traversal and successor map.
    pub fn finish(self) -> BaseRest<UnitSequenceState<'a>> {
        //
        let mut ordered_ids = Vec::with_capacity(self.nodes.len());

        let mut visited_ids = HashSet::with_capacity(self.nodes.len());

        let mut next_ids = HashMap::with_capacity(self.nodes.len());

        let mut current_id = self.head_id;

        while let Some(id) = current_id {
            //
            if !visited_ids.insert(id) {
                return Err(invalid_sequence("Unit sequence contains a cycle"));
            }

            let node = self.node(id)?;

            ordered_ids.push(id);

            next_ids.insert(id, node.next_id);

            current_id = node.next_id;
        }

        if ordered_ids.len() != self.nodes.len() {
            //
            return Err(invalid_sequence(
                "Unit sequence contains unreachable nodes",
            ));
        }

        accept((ordered_ids, next_ids))
    }
}

// Validate the public visible-Unit business limit.
fn validate_visible_count(visible_count: usize) -> BaseRest<()> {
    //
    if visible_count <= MAX_PAGE_UNIT_COUNT {
        return accept(());
    }

    let err_message = trl("error-invalid-unit-oper");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        visible_count,
        max_visible_count = MAX_PAGE_UNIT_COUNT,
        operation = "reorder",
        "expected error: invalid unit operation",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    })
}

// Build an unrecoverable error for a corrupt internal sequence.
fn invalid_sequence(message: &'static str) -> BaseError {
    //
    tracing::error!(message, "unrecoverable error: invalid Unit sequence");

    BaseError::Unrecoverable {
        message: message.into(),
    }
}

// Build the client-visible error for an invalid edit sequence.
fn invalid_edit_sequence(
    unit_id: &str,
    next_unit_id: Option<&str>,
    operation: &'static str,
) -> BaseError {
    //
    let err_message = trl("error-invalid-unit-oper");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        unit_id,
        next_unit_id,
        operation,
        "expected error: invalid unit operation",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}
