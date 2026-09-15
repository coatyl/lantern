//! Core sanitization types: [`Treatment`] trait, [`Change`], [`ChangeSet`].
//!
//! Every built-in treatment is a zero-sized unit struct implementing
//! [`Treatment`]. The trait is object-safe so rule sets can hold
//! `Vec<Box<dyn Treatment>>` with heterogeneous treatment types.

use std::borrow::Cow;

use crate::model::document::{Document, Field};
use crate::model::ids::{DocumentId, NodeId};
// BookmarkFlag is defined in model::node and re-exported from here for
// convenience; callers that work with Change don't need to import node.
pub use crate::model::node::BookmarkFlag;
use crate::model::node::Node;

// ---------------------------------------------------------------------------
// TreatmentCategory
// ---------------------------------------------------------------------------

/// High-level grouping used by the UI to organise treatments in the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreatmentCategory {
    UrlQueryParam,
    UrlPath,
    UrlFragment,
    UrlHost,
    Title,
    FolderName,
    CrossField,
}

// ---------------------------------------------------------------------------
// TreatmentConfig
// ---------------------------------------------------------------------------

/// Per-instance configuration for a treatment.
///
/// Built-in treatments that accept no parameters (e.g. `url.qp.utm`) use the
/// default. Parameterised treatments (e.g. `url.qp.custom`) override specific
/// fields.
#[derive(Debug, Clone, Default)]
pub struct TreatmentConfig {
    // Future fields: custom param lists, regex patterns, replacement modes.
}

// ---------------------------------------------------------------------------
// PassContext
// ---------------------------------------------------------------------------

/// Contextual information passed to every [`Treatment::propose`] call.
///
/// Currently minimal; will grow as treatments gain host-aware logic (e.g.
/// the affiliate treatment needs the bookmark's host to pick the right param
/// list). Those details will be injected via the context rather than
/// hardcoded in the treatment.
#[derive(Debug)]
pub struct PassContext {
    pub document_id: DocumentId,
}

// ---------------------------------------------------------------------------
// ChangeKind
// ---------------------------------------------------------------------------

/// The kind of a single proposed change.
///
/// Each variant carries the data needed to:
/// 1. Display a meaningful preview in the UI.
/// 2. Perform the write in [`Document::apply`].
/// 3. Record the inverse for undo (via [`InverseKind`]).
///
/// [`InverseKind`]: crate::model::document::InverseKind
#[derive(Debug, Clone)]
pub enum ChangeKind {
    /// Mutate a single text field from `before` to `after`.
    SetField {
        field: Field,
        before: String,
        after: String,
    },
    /// Remove a node (and its entire subtree) from the document.
    ///
    /// The inverse is to re-insert the node at its original `(parent_id,
    /// index)`, which the `apply` function discovers at apply time.
    DeleteNode,
    /// Set a boolean flag on a bookmark node.
    SetFlag {
        flag: BookmarkFlag,
        before: bool,
        after: bool,
    },
}

// ---------------------------------------------------------------------------
// Change / ChangeSet
// ---------------------------------------------------------------------------

/// A proposed mutation on one node.
///
/// The `approved` flag defaults to `!destructive`: non-destructive changes are
/// pre-approved so the user only needs to explicitly review risky ones.  The UI
/// flips `approved` per-row before calling `Document::apply`.
#[derive(Debug, Clone)]
pub struct Change {
    pub node_id: NodeId,
    pub kind: ChangeKind,
    /// ID of the treatment that proposed this change.
    pub treatment_id: &'static str,
    /// Human-readable explanation (shown in the preview panel).
    pub rationale: Cow<'static, str>,
    /// True for changes that may alter the URL destination or lose information.
    pub destructive: bool,
    /// Whether this specific change will be executed when `Document::apply` is
    /// called.  Set by the UI; initially `!destructive`.
    pub approved: bool,
}

impl Change {
    // ── Convenience constructors ─────────────────────────────────────────────

    /// Propose a single text-field mutation.
    ///
    /// `approved` is set to `!destructive`.
    pub fn set_field(
        node_id: NodeId,
        field: Field,
        before: impl Into<String>,
        after: impl Into<String>,
        treatment_id: &'static str,
        rationale: impl Into<Cow<'static, str>>,
        destructive: bool,
    ) -> Self {
        Change {
            node_id,
            kind: ChangeKind::SetField {
                field,
                before: before.into(),
                after: after.into(),
            },
            treatment_id,
            rationale: rationale.into(),
            destructive,
            approved: !destructive,
        }
    }

    /// Propose the deletion of a node.
    ///
    /// Always destructive and unapproved by default so the user must
    /// explicitly tick the checkbox.
    pub fn delete_node(
        node_id: NodeId,
        treatment_id: &'static str,
        rationale: impl Into<Cow<'static, str>>,
    ) -> Self {
        Change {
            node_id,
            kind: ChangeKind::DeleteNode,
            treatment_id,
            rationale: rationale.into(),
            destructive: true,
            approved: false,
        }
    }

    /// Propose setting a boolean flag on a bookmark.
    ///
    /// Always non-destructive and auto-approved.
    pub fn set_flag(
        node_id: NodeId,
        flag: BookmarkFlag,
        before: bool,
        after: bool,
        treatment_id: &'static str,
        rationale: impl Into<Cow<'static, str>>,
    ) -> Self {
        Change {
            node_id,
            kind: ChangeKind::SetFlag {
                flag,
                before,
                after,
            },
            treatment_id,
            rationale: rationale.into(),
            destructive: false,
            approved: true,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// The `before` text for a `SetField` change; `None` for other kinds.
    pub fn field_before(&self) -> Option<&str> {
        if let ChangeKind::SetField { before, .. } = &self.kind {
            Some(before)
        } else {
            None
        }
    }

    /// The `after` text for a `SetField` change; `None` for other kinds.
    pub fn field_after(&self) -> Option<&str> {
        if let ChangeKind::SetField { after, .. } = &self.kind {
            Some(after)
        } else {
            None
        }
    }

    /// The targeted `Field` for a `SetField` change; `None` for other kinds.
    pub fn target_field(&self) -> Option<Field> {
        if let ChangeKind::SetField { field, .. } = &self.kind {
            Some(*field)
        } else {
            None
        }
    }
}

/// The complete output of a [`run_pass`](super::pass::run_pass) call: all
/// proposed changes from all treatments in the rule set.
#[derive(Debug)]
pub struct ChangeSet {
    /// Name of the rule set that produced this output (recorded in undo history).
    pub rule_set_name: String,
    pub changes: Vec<Change>,
}

// ---------------------------------------------------------------------------
// Treatment trait
// ---------------------------------------------------------------------------

/// A pure, stateless sanitization rule.
///
/// Implement this trait to add a new built-in treatment.  The implementation
/// must be `Send + Sync` so passes can be parallelised with rayon.
///
/// # Contract
///
/// - `propose` **must not** perform I/O, mutate shared state, or panic.
/// - `propose` must return an empty `Vec` for node types it does not handle
///   (e.g. a URL treatment should return `vec![]` for `Node::Folder`).
/// - If the node already satisfies the rule, `propose` must return `vec![]`.
/// - `propose_document` is called once per pass *after* the per-node loop,
///   with a reference to the whole document.  It defaults to no-op; override
///   it only for cross-field treatments (e.g. `cross.dedupe`).
pub trait Treatment: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn category(&self) -> TreatmentCategory;
    fn is_destructive(&self) -> bool;

    /// Propose zero or more changes for a single node.
    fn propose(&self, node: &Node, ctx: &PassContext) -> Vec<Change>;

    /// Propose changes that require examining the whole document (cross-field
    /// treatments such as `cross.dedupe` and `cross.empty_folders`).
    ///
    /// The default implementation returns an empty list, which is correct for
    /// all node-level treatments.
    fn propose_document(&self, _doc: &Document, _ctx: &PassContext) -> Vec<Change> {
        vec![]
    }

    /// Configure the treatment from a TOML value (M5 per-treatment config).
    ///
    /// Called by the rule-set loader when the TOML entry has a `[config]`
    /// table.  The default implementation ignores the value.
    fn configure(&mut self, _config: &toml::Value) -> Result<(), String> {
        Ok(())
    }

    /// Current per-treatment config, in the same shape `configure` accepts.
    ///
    /// Returned by parameterised treatments so the ruleset writer can persist
    /// user choices back to disk and the UI can render the current values
    /// (custom QP list, regex pattern, …).  Stateless treatments return `None`.
    fn current_config(&self) -> Option<toml::Value> {
        None
    }

    /// Optional per-instance configuration.  Returns the default (no-op) config
    /// for stateless unit-struct treatments.
    fn config(&self) -> TreatmentConfig {
        TreatmentConfig::default()
    }
}
