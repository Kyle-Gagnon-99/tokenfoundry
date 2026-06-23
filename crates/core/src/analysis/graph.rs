//! The `graph` module contains the core data structures and logic for representing and manipulating the IR tokens into a graph format.
//! The graph format is designed to allow a full representation of the token relationships and dependencies, which can then be used for various
//! transformations, outputs, validation, analysis, and more.

use std::collections::{HashMap, HashSet};

use petgraph::{
    algo::kosaraju_scc,
    graph::{EdgeIndex, NodeIndex},
    stable_graph::StableDiGraph,
};

use crate::{
    errors::{Diagnostic, DiagnosticCode, Severity},
    ir::token::token_types::{
        color::{ColorComponentArrayElement, ColorSpaceString},
        composite::stroke_style::StrokeStyleTokenValue,
        font_family::FontFamilyValue,
    },
    ir::{
        IrDocument, IrNode, IrTokenType, IrTokenValue, JsonPointer, JsonRefObject,
        RefAliasOrLiteral, RefOrLiteral, TokenAlias, TokenId, TokenValue,
    },
};

/// The `OwnerId` enum represents the owner of a slot in the graph, which can either be a token or a group, both identified by their respective `TokenId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OwnerId {
    Token(TokenId),
    Group(TokenId),
}

// Graph node type

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    // A slot representing a token as a whole
    TokenNode,
    // A slot representing a group-level node
    GroupNode,
    // A slot representing a property level in a token's value
    Property,
}

pub type LocalPointer = JsonPointer;

/// The `SlotRecord` struct represents the owner of the slot, which can either be a token or a group, both identified by their respective `TokenId`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SlotRecord {
    /// The `kind` field represents the type of slot in the graph, which can be a token node, group node, or property.
    pub kind: SlotKind,
    /// The `owner` field represents the owner of the slot, which can either be a token or a group, both identified by their respective `TokenId`.
    pub owner: OwnerId,
    /// The `local_pointer` field is a JSON pointer to the built and resolved document the graph is constructed from
    pub local_pointer: LocalPointer,
}

/// The `EdgeKind` enum represents the type of relationship between slots in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Alias,
    JsonRef,
    PropertyRef,
}

/// The `EdgeRecord` struct represents an edge in the graph, connecting two slots with a specific relationship type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeRecord {
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeKey {
    from: NodeIndex,
    to: NodeIndex,
    kind: EdgeKind,
}

#[derive(Debug, Clone)]
pub struct TokenGraph {
    /// The graph field is a directed graph where nodes represents slots (token, group, or property) and edges represent relationships (aliasing, referencing) between them.
    ir_graph: petgraph::stable_graph::StableDiGraph<SlotRecord, EdgeRecord>,
    /// The `slots_to_ids` field is a mapping from `SlotKey` to the corresponding `NodeIndex` in the graph, allowing for efficient lookups and manipulations of the graph structure.
    slots_to_ids: HashMap<SlotRecord, NodeIndex>,
    /// Mapping from edge records to their corresponding `EdgeIndex` in the graph, allowing for efficient lookups and manipulations of edges within the graph structure.
    edges_to_ids: HashMap<EdgeKey, EdgeIndex>,
}

impl Default for TokenGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenGraph {
    /// Creates a new instance of `TokenGraph` with an empty graph and empty mappings for slots and edges.
    pub fn new() -> Self {
        Self {
            ir_graph: StableDiGraph::new(),
            slots_to_ids: HashMap::new(),
            edges_to_ids: HashMap::new(),
        }
    }

    /// Adds a new slot to the graph and updates the mappings accordingly.
    ///
    /// # Arguments
    ///
    /// * `slot` - A `SlotRecord` struct containing information about the kind, owner, and local pointer of the slot to be added.
    ///
    /// # Returns
    ///
    /// A `NodeIndex` representing the index of the newly added slot in the graph.
    pub fn add_slot(&mut self, slot: SlotRecord) -> NodeIndex {
        if let Some(existing_index) = self.slots_to_ids.get(&slot) {
            *existing_index
        } else {
            let node_index = self.ir_graph.add_node(slot.clone());
            self.slots_to_ids.insert(slot, node_index);
            node_index
        }
    }

    /// Adds a new edge to the graph between two nodes and updates the mappings accordingly.
    ///
    /// # Arguments
    ///
    /// * `from` - A `NodeIndex` representing the starting node of the edge.
    /// * `to` - A `NodeIndex` representing the ending node of the edge.
    /// * `kind` - An `EdgeKind` enum value representing the type of relationship between the two nodes.
    ///
    /// # Returns
    ///
    /// An `EdgeIndex` representing the index of the newly added edge in the graph.
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, kind: EdgeKind) -> EdgeIndex {
        let edge_key = EdgeKey { from, to, kind };
        let edge_record = EdgeRecord { kind };
        if let Some(existing_index) = self.edges_to_ids.get(&edge_key) {
            *existing_index
        } else {
            let edge_index = self.ir_graph.add_edge(from, to, edge_record.clone());
            self.edges_to_ids.insert(edge_key, edge_index);
            edge_index
        }
    }

    /// Retrieves the `NodeIndex` of a slot in the graph based on the provided `SlotRecord`.
    ///
    /// # Arguments
    ///
    /// * `slot` - A `SlotRecord` struct containing information about the kind, owner, and local pointer of the slot to be looked up.
    ///
    /// # Returns
    ///
    /// An `Option<NodeIndex>` which is `Some(NodeIndex)` if the slot exists in the graph, or `None` if it does not.
    pub fn get_slot_node_index(&self, slot: &SlotRecord) -> Option<NodeIndex> {
        // Cloning the NodeIndex is necessary because NodeIndex is owned by the map and we need to return an owned value
        self.slots_to_ids.get(slot).cloned()
    }

    /// Checks if a slot exists in the graph based on the provided `SlotRecord`.
    ///
    /// # Arguments
    ///
    /// * `slot` - A `SlotRecord` struct containing information about the kind, owner, and local pointer of the slot to be checked.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the slot exists in the graph (`true`) or not (`false`).
    pub fn slot_exists_by_record(&self, slot: &SlotRecord) -> bool {
        self.slots_to_ids.contains_key(slot)
    }

    /// Checks if a slot exists in the graph based on the provided `NodeIndex`.
    ///
    /// # Arguments
    ///
    /// * `index` - A `NodeIndex` representing the index of the slot to be checked.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the slot exists in the graph (`true`) or not (`false`).
    pub fn slot_exists_by_index(&self, index: NodeIndex) -> bool {
        self.ir_graph.node_weight(index).is_some()
    }

    /// Retrieves the `EdgeIndex` of an edge in the graph based on the provided `EdgeRecord`.
    ///
    /// # Arguments
    ///
    /// * `edge` - An `EdgeRecord` struct containing information about the kind of edge to be looked up.
    ///
    /// # Returns
    ///
    /// An `Option<EdgeIndex>` which is `Some(EdgeIndex)` if the edge exists in the graph, or `None` if it does not.
    pub fn get_edge_index(&self, edge: &EdgeRecord) -> Option<EdgeIndex> {
        self.edges_to_ids
            .iter()
            .find_map(|(key, index)| (key.kind == edge.kind).then_some(*index))
    }

    /// Checks if an edge exists in the graph between two nodes based on the provided `EdgeRecord`.
    ///
    /// # Arguments
    ///
    /// * `from` - A `NodeIndex` representing the starting node of the edge.
    /// * `to` - A `NodeIndex` representing the ending node of the edge.
    /// * `kind` - An `EdgeKind` enum value representing the type of relationship between the two nodes.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the edge exists in the graph (`true`) or not (`false`).
    pub fn edge_exists_between(&self, from: NodeIndex, to: NodeIndex, kind: EdgeKind) -> bool {
        self.edges_to_ids.contains_key(&EdgeKey { from, to, kind })
    }

    /// Checks if an edge exists in the graph based on the provided `EdgeIndex`.
    ///
    /// # Arguments
    ///
    /// * `index` - An `EdgeIndex` representing the index of the edge to be checked.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the edge exists in the graph (`true`) or not (`false`).
    pub fn edge_exists_by_index(&self, index: EdgeIndex) -> bool {
        self.ir_graph.edge_weight(index).is_some()
    }

    /// Returns the total number of nodes (slots) in the graph.
    /// This method provides a count of all the nodes currently present in the graph, which can be useful for analysis and debugging purposes.
    ///
    /// # Returns
    ///
    /// A `usize` value representing the total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.ir_graph.node_count()
    }

    /// Returns the total number of edges in the graph.
    /// This method provides a count of all the edges currently present in the graph, which can be useful for analysis and debugging purposes.
    ///
    /// # Returns
    ///
    /// A `usize` value representing the total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.ir_graph.edge_count()
    }

    /// Counts the number of edges in the graph that match a specific `EdgeKind`.
    ///
    /// # Arguments
    ///
    /// * `kind` - An `EdgeKind` enum value representing the type of edges to be counted.
    ///
    /// # Returns
    ///
    /// A `usize` value representing the total number of edges in the graph that match the specified `EdgeKind`.
    pub fn count_edges_by_kind(&self, kind: EdgeKind) -> usize {
        self.ir_graph
            .edge_weights()
            .filter(|edge| edge.kind == kind)
            .count()
    }

    /// Counts the number of slots in the graph that match a specific `SlotKind`.
    ///
    /// # Arguments
    ///
    /// * `kind` - A `SlotKind` enum value representing the type of slots to be counted.
    ///
    /// # Returns
    ///
    /// A `usize` value representing the total number of slots in the graph that match the specified `SlotKind`.
    pub fn count_slots_by_kind(&self, kind: SlotKind) -> usize {
        self.ir_graph
            .node_weights()
            .filter(|slot| slot.kind == kind)
            .count()
    }

    pub fn from_ir_document(document: &IrDocument) -> Self {
        let mut graph = Self::new();

        let mut token_path_to_owner: HashMap<Vec<String>, OwnerId> = HashMap::new();
        let mut group_path_to_owner: HashMap<Vec<String>, OwnerId> = HashMap::new();

        for node in &document.tokens {
            graph.index_nodes(node, &mut token_path_to_owner, &mut group_path_to_owner);
        }

        for node in &document.tokens {
            graph.connect_reference_edges(node, &token_path_to_owner, &group_path_to_owner);
        }

        graph
    }

    pub fn validate(&self, document: &IrDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let mut owner_to_common: HashMap<OwnerId, &crate::ir::TokenCommon> = HashMap::new();
        let mut token_path_to_token: HashMap<Vec<String>, &crate::ir::IrToken> = HashMap::new();

        for node in &document.tokens {
            index_validation_nodes(node, &mut owner_to_common);
            index_validation_tokens(node, &mut token_path_to_token);
        }

        for node in &document.tokens {
            validate_node(node, &mut diagnostics);
        }

        validate_alias_target_types(&token_path_to_token, &mut diagnostics);

        validate_circular_references(self, &mut diagnostics, &owner_to_common);

        diagnostics
    }

    fn index_nodes(
        &mut self,
        node: &IrNode,
        token_path_to_owner: &mut HashMap<Vec<String>, OwnerId>,
        group_path_to_owner: &mut HashMap<Vec<String>, OwnerId>,
    ) {
        match node {
            IrNode::Token(token) => {
                let owner = OwnerId::Token(token.common.id);
                token_path_to_owner.insert(token.common.path.segments.clone(), owner);

                self.add_slot(SlotRecord {
                    kind: SlotKind::TokenNode,
                    owner,
                    local_pointer: LocalPointer::new(),
                });

                self.add_slot(SlotRecord {
                    kind: SlotKind::Property,
                    owner,
                    local_pointer: LocalPointer::from_segments(["$value"]),
                });
            }
            IrNode::Group(group) => {
                let owner = OwnerId::Group(group.common.id);
                group_path_to_owner.insert(group.common.path.segments.clone(), owner);

                self.add_slot(SlotRecord {
                    kind: SlotKind::GroupNode,
                    owner,
                    local_pointer: LocalPointer::new(),
                });

                for child in &group.children {
                    self.index_nodes(child, token_path_to_owner, group_path_to_owner);
                }
            }
        }
    }

    fn connect_reference_edges(
        &mut self,
        node: &IrNode,
        token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
        group_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    ) {
        match node {
            IrNode::Token(token) => {
                let from_owner = OwnerId::Token(token.common.id);
                let from_slot = SlotRecord {
                    kind: SlotKind::Property,
                    owner: from_owner,
                    local_pointer: LocalPointer::from_segments(["$value"]),
                };

                let Some(from_index) = self.get_slot_node_index(&from_slot) else {
                    return;
                };

                match &token.value {
                    TokenValue::Alias(alias) => {
                        let target_owner = token_path_to_owner.get(&alias.target_path.segments);

                        if let Some(owner) = target_owner {
                            let target_slot = SlotRecord {
                                kind: SlotKind::Property,
                                owner: *owner,
                                local_pointer: LocalPointer::from_segments(["$value"]),
                            };

                            let to_index = self.add_slot(target_slot);
                            self.add_edge(from_index, to_index, EdgeKind::Alias);
                        }
                    }
                    TokenValue::Ref(json_ref) => {
                        // For now, only same-document refs are converted into graph edges.
                        if json_ref.document.is_some() {
                            return;
                        }

                        if let Some((owner, pointer)) = resolve_ref_target(
                            &json_ref.pointer,
                            token_path_to_owner,
                            group_path_to_owner,
                        ) {
                            let target_slot = SlotRecord {
                                kind: if pointer.segments.is_empty() {
                                    match owner {
                                        OwnerId::Token(_) => SlotKind::TokenNode,
                                        OwnerId::Group(_) => SlotKind::GroupNode,
                                    }
                                } else {
                                    SlotKind::Property
                                },
                                owner,
                                local_pointer: pointer,
                            };

                            let to_index = self.add_slot(target_slot);
                            self.add_edge(from_index, to_index, EdgeKind::JsonRef);
                        }
                    }
                    TokenValue::Value(value) => {
                        self.connect_value_property_reference_edges(
                            from_owner,
                            value,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                }
            }
            IrNode::Group(group) => {
                for child in &group.children {
                    self.connect_reference_edges(child, token_path_to_owner, group_path_to_owner);
                }
            }
        }
    }

    fn connect_value_property_reference_edges(
        &mut self,
        owner: OwnerId,
        value: &IrTokenValue,
        token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
        group_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    ) {
        match value {
            IrTokenValue::Color(color) => {
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value", "colorSpace"],
                    &color.color_space,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value", "components"],
                    &color.components,
                    token_path_to_owner,
                    group_path_to_owner,
                );

                if let Some(alpha) = &color.alpha {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "alpha"],
                        alpha,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                if let Some(hex) = &color.hex {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "hex"],
                        hex,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
            }
            IrTokenValue::Dimension(dimension) => {
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value", "value"],
                    &dimension.value,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value", "unit"],
                    &dimension.unit,
                    token_path_to_owner,
                    group_path_to_owner,
                );
            }
            IrTokenValue::Duration(duration) => {
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value", "value"],
                    &duration.value,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value", "unit"],
                    &duration.unit,
                    token_path_to_owner,
                    group_path_to_owner,
                );
            }
            IrTokenValue::Number(number) => {
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value"],
                    &number.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
            }
            IrTokenValue::FontWeight(weight) => {
                self.connect_ref_or_literal_property(
                    owner,
                    &["$value"],
                    &weight.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
            }
            IrTokenValue::CubicBezier(curve) => {
                for (index, value) in curve.0.iter().enumerate() {
                    let idx = index.to_string();
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", idx.as_str()],
                        value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
            }
            IrTokenValue::FontFamily(font_family) => match &font_family.0 {
                FontFamilyValue::Single(value) => {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value"],
                        value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
                FontFamilyValue::Multiple(value) => match value {
                    RefOrLiteral::Ref(ref_obj) => {
                        self.connect_json_ref_at_path(
                            owner,
                            &["$value"],
                            ref_obj,
                            EdgeKind::PropertyRef,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                    RefOrLiteral::Literal(values) => {
                        for (index, item) in values.0.iter().enumerate() {
                            let idx = index.to_string();
                            self.connect_ref_or_literal_property(
                                owner,
                                &["$value", idx.as_str()],
                                item,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                        }
                    }
                },
            },
            IrTokenValue::StrokeStyle(stroke_style) => match stroke_style {
                StrokeStyleTokenValue::String(value) => {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value"],
                        value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
                StrokeStyleTokenValue::Object(value) => match value {
                    RefOrLiteral::Ref(ref_obj) => {
                        self.connect_json_ref_at_path(
                            owner,
                            &["$value"],
                            ref_obj,
                            EdgeKind::PropertyRef,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                    RefOrLiteral::Literal(obj) => {
                        match &obj.dash_array {
                            RefOrLiteral::Ref(ref_obj) => {
                                self.connect_json_ref_at_path(
                                    owner,
                                    &["$value", "dashArray"],
                                    ref_obj,
                                    EdgeKind::PropertyRef,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }
                            RefOrLiteral::Literal(dashes) => {
                                for (index, dash) in dashes.0.iter().enumerate() {
                                    let idx = index.to_string();
                                    self.connect_ref_alias_or_literal_property(
                                        owner,
                                        &["$value", "dashArray", idx.as_str()],
                                        dash,
                                        token_path_to_owner,
                                        group_path_to_owner,
                                    );

                                    if let RefAliasOrLiteral::Literal(dimension) = dash {
                                        self.connect_ref_or_literal_property(
                                            owner,
                                            &["$value", "dashArray", idx.as_str(), "value"],
                                            &dimension.value,
                                            token_path_to_owner,
                                            group_path_to_owner,
                                        );
                                        self.connect_ref_or_literal_property(
                                            owner,
                                            &["$value", "dashArray", idx.as_str(), "unit"],
                                            &dimension.unit,
                                            token_path_to_owner,
                                            group_path_to_owner,
                                        );
                                    }
                                }
                            }
                        }

                        self.connect_ref_or_literal_property(
                            owner,
                            &["$value", "lineCap"],
                            &obj.line_cap,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                },
            },
            IrTokenValue::Border(border) => {
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "color"],
                    &border.color.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                if let RefAliasOrLiteral::Literal(color) = &border.color.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "color", "colorSpace"],
                        &color.color_space,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "color", "components"],
                        &color.components,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    if let Some(alpha) = &color.alpha {
                        self.connect_ref_or_literal_property(
                            owner,
                            &["$value", "color", "alpha"],
                            alpha,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                    if let Some(hex) = &color.hex {
                        self.connect_ref_or_literal_property(
                            owner,
                            &["$value", "color", "hex"],
                            hex,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                }

                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "width"],
                    &border.width.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                if let RefAliasOrLiteral::Literal(width) = &border.width.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "width", "value"],
                        &width.value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "width", "unit"],
                        &width.unit,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "style"],
                    &border.style.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                if let RefAliasOrLiteral::Literal(style) = &border.style.0 {
                    match style {
                        StrokeStyleTokenValue::String(value) => {
                            self.connect_ref_or_literal_property(
                                owner,
                                &["$value", "style"],
                                value,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                        }
                        StrokeStyleTokenValue::Object(value) => match value {
                            RefOrLiteral::Ref(ref_obj) => {
                                self.connect_json_ref_at_path(
                                    owner,
                                    &["$value", "style"],
                                    ref_obj,
                                    EdgeKind::PropertyRef,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }
                            RefOrLiteral::Literal(obj) => {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", "style", "dashArray"],
                                    &obj.dash_array,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", "style", "lineCap"],
                                    &obj.line_cap,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }
                        },
                    }
                }
            }
            IrTokenValue::Transition(transition) => {
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "duration"],
                    &transition.duration.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "delay"],
                    &transition.delay.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "timingFunction"],
                    &transition.timing_function.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );

                if let RefAliasOrLiteral::Literal(duration) = &transition.duration.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "duration", "value"],
                        &duration.value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "duration", "unit"],
                        &duration.unit,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                if let RefAliasOrLiteral::Literal(delay) = &transition.delay.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "delay", "value"],
                        &delay.value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "delay", "unit"],
                        &delay.unit,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                if let RefAliasOrLiteral::Literal(curve) = &transition.timing_function.0 {
                    for (index, value) in curve.0.iter().enumerate() {
                        let idx = index.to_string();
                        self.connect_ref_or_literal_property(
                            owner,
                            &["$value", "timingFunction", idx.as_str()],
                            value,
                            token_path_to_owner,
                            group_path_to_owner,
                        );
                    }
                }
            }
            IrTokenValue::Typography(typography) => {
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "fontFamily"],
                    &typography.font_family.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "fontSize"],
                    &typography.font_size.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "fontWeight"],
                    &typography.font_weight.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "letterSpacing"],
                    &typography.letter_spacing.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );
                self.connect_ref_alias_or_literal_property(
                    owner,
                    &["$value", "lineHeight"],
                    &typography.line_height.0,
                    token_path_to_owner,
                    group_path_to_owner,
                );

                if let RefAliasOrLiteral::Literal(font_family) = &typography.font_family.0 {
                    match &font_family.0 {
                        FontFamilyValue::Single(value) => {
                            self.connect_ref_or_literal_property(
                                owner,
                                &["$value", "fontFamily"],
                                value,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                        }
                        FontFamilyValue::Multiple(value) => match value {
                            RefOrLiteral::Ref(ref_obj) => {
                                self.connect_json_ref_at_path(
                                    owner,
                                    &["$value", "fontFamily"],
                                    ref_obj,
                                    EdgeKind::PropertyRef,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }
                            RefOrLiteral::Literal(items) => {
                                for (index, item) in items.0.iter().enumerate() {
                                    let idx = index.to_string();
                                    self.connect_ref_or_literal_property(
                                        owner,
                                        &["$value", "fontFamily", idx.as_str()],
                                        item,
                                        token_path_to_owner,
                                        group_path_to_owner,
                                    );
                                }
                            }
                        },
                    }
                }

                if let RefAliasOrLiteral::Literal(font_size) = &typography.font_size.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "fontSize", "value"],
                        &font_size.value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "fontSize", "unit"],
                        &font_size.unit,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                if let RefAliasOrLiteral::Literal(font_weight) = &typography.font_weight.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "fontWeight"],
                        &font_weight.0,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                if let RefAliasOrLiteral::Literal(letter_spacing) = &typography.letter_spacing.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "letterSpacing", "value"],
                        &letter_spacing.value,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "letterSpacing", "unit"],
                        &letter_spacing.unit,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }

                if let RefAliasOrLiteral::Literal(line_height) = &typography.line_height.0 {
                    self.connect_ref_or_literal_property(
                        owner,
                        &["$value", "lineHeight"],
                        &line_height.0,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
            }
            IrTokenValue::Gradient(gradient) => match &gradient.0 {
                RefOrLiteral::Ref(ref_obj) => {
                    self.connect_json_ref_at_path(
                        owner,
                        &["$value"],
                        ref_obj,
                        EdgeKind::PropertyRef,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
                RefOrLiteral::Literal(items) => {
                    for (index, item) in items.iter().enumerate() {
                        let idx = index.to_string();
                        self.connect_ref_alias_or_literal_property(
                            owner,
                            &["$value", idx.as_str()],
                            item,
                            token_path_to_owner,
                            group_path_to_owner,
                        );

                        if let RefAliasOrLiteral::Literal(gradient_object) = item {
                            self.connect_ref_alias_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "color"],
                                &gradient_object.color.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                            if let RefAliasOrLiteral::Literal(color) = &gradient_object.color.0 {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "color", "colorSpace"],
                                    &color.color_space,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "color", "components"],
                                    &color.components,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                if let Some(alpha) = &color.alpha {
                                    self.connect_ref_or_literal_property(
                                        owner,
                                        &["$value", idx.as_str(), "color", "alpha"],
                                        alpha,
                                        token_path_to_owner,
                                        group_path_to_owner,
                                    );
                                }
                                if let Some(hex) = &color.hex {
                                    self.connect_ref_or_literal_property(
                                        owner,
                                        &["$value", idx.as_str(), "color", "hex"],
                                        hex,
                                        token_path_to_owner,
                                        group_path_to_owner,
                                    );
                                }
                            }
                            self.connect_ref_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "position"],
                                &gradient_object.position.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                        }
                    }
                }
            },
            IrTokenValue::Shadow(shadow) => match &shadow.0 {
                RefOrLiteral::Ref(ref_obj) => {
                    self.connect_json_ref_at_path(
                        owner,
                        &["$value"],
                        ref_obj,
                        EdgeKind::PropertyRef,
                        token_path_to_owner,
                        group_path_to_owner,
                    );
                }
                RefOrLiteral::Literal(items) => {
                    for (index, item) in items.iter().enumerate() {
                        let idx = index.to_string();
                        self.connect_ref_alias_or_literal_property(
                            owner,
                            &["$value", idx.as_str()],
                            item,
                            token_path_to_owner,
                            group_path_to_owner,
                        );

                        if let RefAliasOrLiteral::Literal(shadow_object) = item {
                            self.connect_ref_alias_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "color"],
                                &shadow_object.color.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                            self.connect_ref_alias_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "offsetX"],
                                &shadow_object.offset_x.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                            self.connect_ref_alias_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "offsetY"],
                                &shadow_object.offset_y.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                            self.connect_ref_alias_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "blur"],
                                &shadow_object.blur.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );
                            self.connect_ref_alias_or_literal_property(
                                owner,
                                &["$value", idx.as_str(), "spread"],
                                &shadow_object.spread.0,
                                token_path_to_owner,
                                group_path_to_owner,
                            );

                            if let Some(inset) = &shadow_object.inset {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "inset"],
                                    &inset.0,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }

                            if let RefAliasOrLiteral::Literal(color) = &shadow_object.color.0 {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "color", "colorSpace"],
                                    &color.color_space,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "color", "components"],
                                    &color.components,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                if let Some(alpha) = &color.alpha {
                                    self.connect_ref_or_literal_property(
                                        owner,
                                        &["$value", idx.as_str(), "color", "alpha"],
                                        alpha,
                                        token_path_to_owner,
                                        group_path_to_owner,
                                    );
                                }
                                if let Some(hex) = &color.hex {
                                    self.connect_ref_or_literal_property(
                                        owner,
                                        &["$value", idx.as_str(), "color", "hex"],
                                        hex,
                                        token_path_to_owner,
                                        group_path_to_owner,
                                    );
                                }
                            }

                            if let RefAliasOrLiteral::Literal(offset_x) = &shadow_object.offset_x.0
                            {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "offsetX", "value"],
                                    &offset_x.value,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "offsetX", "unit"],
                                    &offset_x.unit,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }

                            if let RefAliasOrLiteral::Literal(offset_y) = &shadow_object.offset_y.0
                            {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "offsetY", "value"],
                                    &offset_y.value,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "offsetY", "unit"],
                                    &offset_y.unit,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }

                            if let RefAliasOrLiteral::Literal(blur) = &shadow_object.blur.0 {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "blur", "value"],
                                    &blur.value,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "blur", "unit"],
                                    &blur.unit,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }

                            if let RefAliasOrLiteral::Literal(spread) = &shadow_object.spread.0 {
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "spread", "value"],
                                    &spread.value,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                                self.connect_ref_or_literal_property(
                                    owner,
                                    &["$value", idx.as_str(), "spread", "unit"],
                                    &spread.unit,
                                    token_path_to_owner,
                                    group_path_to_owner,
                                );
                            }
                        }
                    }
                }
            },
        }
    }

    fn connect_ref_or_literal_property<T>(
        &mut self,
        owner: OwnerId,
        local_path: &[&str],
        value: &RefOrLiteral<T>,
        token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
        group_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    ) {
        let RefOrLiteral::Ref(ref_obj) = value else {
            return;
        };

        self.connect_json_ref_at_path(
            owner,
            local_path,
            ref_obj,
            EdgeKind::PropertyRef,
            token_path_to_owner,
            group_path_to_owner,
        );
    }

    fn connect_ref_alias_or_literal_property<T>(
        &mut self,
        owner: OwnerId,
        local_path: &[&str],
        value: &RefAliasOrLiteral<T>,
        token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
        group_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    ) {
        match value {
            RefAliasOrLiteral::Alias(alias) => {
                self.connect_alias_at_path(owner, local_path, alias, token_path_to_owner);
            }
            RefAliasOrLiteral::Ref(ref_obj) => {
                self.connect_json_ref_at_path(
                    owner,
                    local_path,
                    ref_obj,
                    EdgeKind::PropertyRef,
                    token_path_to_owner,
                    group_path_to_owner,
                );
            }
            RefAliasOrLiteral::Literal(_) => {}
        }
    }

    fn connect_alias_at_path(
        &mut self,
        owner: OwnerId,
        local_path: &[&str],
        alias: &TokenAlias,
        token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    ) {
        let Some(target_owner) = token_path_to_owner.get(&alias.target_path.segments) else {
            return;
        };

        let from_slot = SlotRecord {
            kind: SlotKind::Property,
            owner,
            local_pointer: LocalPointer::from_segments(local_path.iter().copied()),
        };
        let to_slot = SlotRecord {
            kind: SlotKind::Property,
            owner: *target_owner,
            local_pointer: LocalPointer::from_segments(["$value"]),
        };

        let from_index = self.add_slot(from_slot);
        let to_index = self.add_slot(to_slot);
        self.add_edge(from_index, to_index, EdgeKind::Alias);
    }

    fn connect_json_ref_at_path(
        &mut self,
        owner: OwnerId,
        local_path: &[&str],
        ref_obj: &JsonRefObject,
        edge_kind: EdgeKind,
        token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
        group_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    ) {
        if ref_obj.reference.document.is_some() {
            return;
        }

        let from_slot = SlotRecord {
            kind: SlotKind::Property,
            owner,
            local_pointer: LocalPointer::from_segments(local_path.iter().copied()),
        };
        let from_index = self.add_slot(from_slot);

        if let Some((target_owner, target_pointer)) = resolve_ref_target(
            &ref_obj.reference.pointer,
            token_path_to_owner,
            group_path_to_owner,
        ) {
            let target_slot = SlotRecord {
                kind: if target_pointer.segments.is_empty() {
                    match target_owner {
                        OwnerId::Token(_) => SlotKind::TokenNode,
                        OwnerId::Group(_) => SlotKind::GroupNode,
                    }
                } else {
                    SlotKind::Property
                },
                owner: target_owner,
                local_pointer: target_pointer,
            };
            let to_index = self.add_slot(target_slot);
            self.add_edge(from_index, to_index, edge_kind);
        }
    }
}

fn resolve_ref_target(
    pointer: &JsonPointer,
    token_path_to_owner: &HashMap<Vec<String>, OwnerId>,
    group_path_to_owner: &HashMap<Vec<String>, OwnerId>,
) -> Option<(OwnerId, LocalPointer)> {
    let segments = &pointer.segments;

    for split in (0..=segments.len()).rev() {
        let prefix = segments[..split].to_vec();
        let suffix = segments[split..].to_vec();

        if let Some(owner) = token_path_to_owner.get(&prefix) {
            let pointer = if suffix.is_empty() {
                LocalPointer::from_segments(["$value"])
            } else {
                LocalPointer::from_segments(suffix)
            };
            return Some((*owner, pointer));
        }

        if let Some(owner) = group_path_to_owner.get(&prefix) {
            return Some((*owner, LocalPointer::from_segments(suffix)));
        }
    }

    None
}

fn index_validation_nodes<'a>(
    node: &'a IrNode,
    owner_to_common: &mut HashMap<OwnerId, &'a crate::ir::TokenCommon>,
) {
    match node {
        IrNode::Token(token) => {
            owner_to_common.insert(OwnerId::Token(token.common.id), &token.common);
        }
        IrNode::Group(group) => {
            owner_to_common.insert(OwnerId::Group(group.common.id), &group.common);
            for child in &group.children {
                index_validation_nodes(child, owner_to_common);
            }
        }
    }
}

fn index_validation_tokens<'a>(
    node: &'a IrNode,
    token_path_to_token: &mut HashMap<Vec<String>, &'a crate::ir::IrToken>,
) {
    match node {
        IrNode::Token(token) => {
            token_path_to_token.insert(token.common.path.segments.clone(), token);
        }
        IrNode::Group(group) => {
            for child in &group.children {
                index_validation_tokens(child, token_path_to_token);
            }
        }
    }
}

fn validate_alias_target_types(
    token_path_to_token: &HashMap<Vec<String>, &crate::ir::IrToken>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for token in token_path_to_token.values() {
        let TokenValue::Alias(alias) = &token.value else {
            continue;
        };

        validate_alias_chain_type(token, alias, token_path_to_token, diagnostics);
    }
}

fn validate_alias_chain_type(
    source_token: &crate::ir::IrToken,
    alias: &TokenAlias,
    token_path_to_token: &HashMap<Vec<String>, &crate::ir::IrToken>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected_type = source_token.token_type;
    let mut current_target_path = alias.target_path.segments.clone();
    let mut visited_paths: HashSet<Vec<String>> = HashSet::new();

    loop {
        if !visited_paths.insert(current_target_path.clone()) {
            // Circular aliases are reported by graph cycle validation.
            break;
        }

        let Some(target_token) = token_path_to_token.get(&current_target_path) else {
            diagnostics.push(make_diagnostic(
                &source_token.common,
                DiagnosticCode::InvalidReferenceTarget,
                format!(
                    "token alias target '{{{}}}' could not be resolved",
                    current_target_path.join(".")
                ),
                source_token.common.value_json_pointer(),
            ));
            break;
        };

        if target_token.token_type != expected_type {
            diagnostics.push(make_diagnostic(
                &source_token.common,
                DiagnosticCode::InvalidReferenceTarget,
                format!(
                    "token alias target '{{{}}}' has incompatible type: expected {}, found {}",
                    current_target_path.join("."),
                    token_type_label(expected_type),
                    token_type_label(target_token.token_type),
                ),
                source_token.common.value_json_pointer(),
            ));
            break;
        }

        let TokenValue::Alias(next_alias) = &target_token.value else {
            break;
        };

        current_target_path = next_alias.target_path.segments.clone();
    }
}

fn token_type_label(token_type: IrTokenType) -> &'static str {
    match token_type {
        IrTokenType::Color => "color",
        IrTokenType::Dimension => "dimension",
        IrTokenType::FontFamily => "fontFamily",
        IrTokenType::FontWeight => "fontWeight",
        IrTokenType::Duration => "duration",
        IrTokenType::CubicBezier => "cubicBezier",
        IrTokenType::Number => "number",
        IrTokenType::StrokeStyle => "strokeStyle",
        IrTokenType::Border => "border",
        IrTokenType::Transition => "transition",
        IrTokenType::Shadow => "shadow",
        IrTokenType::Gradient => "gradient",
        IrTokenType::Typography => "typography",
    }
}

fn validate_node(node: &IrNode, diagnostics: &mut Vec<Diagnostic>) {
    match node {
        IrNode::Token(token) => {
            validate_name_segments(
                &token.common,
                "token",
                diagnostics,
                token.common.path.as_json_pointer(),
            );
            validate_color_token(token, diagnostics);
        }
        IrNode::Group(group) => {
            validate_name_segments(
                &group.common,
                "group",
                diagnostics,
                group.common.path.as_json_pointer(),
            );

            for child in &group.children {
                validate_node(child, diagnostics);
            }
        }
    }
}

/// Validates the name of the token's and segments of the path to ensure they do not contain invalid characters or patterns.
///
/// The rules are:
/// - If a segment starts with a '$', it must be the last segment and must be exactly "$root".
/// - Segments cannot contain any of the following characters: '{', '}', '.', '$' (except for the allowed '$root' case).
/// If any of these rules are violated, a diagnostic with code `InvalidTokenName` will be added to the diagnostics vector.
///
/// # Arguments
///
/// - `common`: The common token information containing the path to validate.
/// - `kind`: A string indicating whether the token is a "token" or a "group", used for error messages.
/// - `diagnostics`: A mutable reference to a vector where any diagnostics will be added if validation fails.
/// - `path`: The JSON pointer path of the token or group being validated, used for error messages.
fn validate_name_segments(
    common: &crate::ir::TokenCommon,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
    path: String,
) {
    for (index, segment) in common.path.segments.iter().enumerate() {
        // If the segment is the last segment, check that if it starts with a $, it must be followed by "root"
        let is_last = index == common.path.segments.len() - 1;
        if segment.starts_with('$') && is_last && segment == "$root" {
            continue;
        }

        // Validate the rest of the segment to make sure it does not contain '{', '}', '.', or '$'
        if segment.contains(['{', '}', '.', '$']) {
            diagnostics.push(make_diagnostic(
                common,
                DiagnosticCode::InvalidTokenName,
                format!(
                    "{} path segments cannot contain '{{', '}}', '.', or '$', got '{}'",
                    kind, segment
                ),
                format!("{}", path),
            ));
        }
    }
}

fn validate_color_token(token: &crate::ir::IrToken, diagnostics: &mut Vec<Diagnostic>) {
    let crate::ir::TokenValue::Value(crate::ir::IrTokenValue::Color(color)) = &token.value else {
        return;
    };

    let common = &token.common;

    if let RefOrLiteral::Literal(space) = &color.color_space {
        if let RefOrLiteral::Literal(components) = &color.components {
            for (index, component) in components.0.iter().enumerate() {
                if let ColorComponentArrayElement::Number(number) = component {
                    let path = format!(
                        "{}/$value/components/{}",
                        common.path.as_json_pointer(),
                        index
                    );
                    if let Some(value) = number.0.as_f64() {
                        if let Some(message) = validate_color_component_range(space, index, value) {
                            diagnostics.push(make_diagnostic(
                                common,
                                DiagnosticCode::InvalidPropertyValue,
                                message,
                                path,
                            ));
                        }
                    }
                }
            }
        }
    }

    if let Some(alpha) = &color.alpha {
        if let RefOrLiteral::Literal(alpha) = alpha {
            if let Some(value) = alpha.0.0.as_f64() {
                if !(0.0..=1.0).contains(&value) {
                    diagnostics.push(make_diagnostic(
                        common,
                        DiagnosticCode::InvalidPropertyValue,
                        format!("color alpha must be between 0 and 1 inclusive, got {value}"),
                        format!("{}/$value/alpha", common.path.as_json_pointer()),
                    ));
                }
            }
        }
    }

    if let Some(hex) = &color.hex {
        if let RefOrLiteral::Literal(hex) = hex {
            if !is_valid_hex_fallback(&hex.0) {
                diagnostics.push(make_diagnostic(
                    common,
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "color hex fallback must be a 6-digit CSS hex color, got '{}'",
                        hex.0
                    ),
                    format!("{}/$value/hex", common.path.as_json_pointer()),
                ));
            }
        }
    }
}

fn validate_color_component_range(
    space: &ColorSpaceString,
    index: usize,
    value: f64,
) -> Option<String> {
    let in_range = match space {
        ColorSpaceString::SRGB
        | ColorSpaceString::SRGBLinear
        | ColorSpaceString::DisplayP3
        | ColorSpaceString::A98RGB
        | ColorSpaceString::ProPhotoRGB
        | ColorSpaceString::Rec2020
        | ColorSpaceString::XYZD65
        | ColorSpaceString::XYZD50 => (0.0..=1.0).contains(&value),
        ColorSpaceString::HSL | ColorSpaceString::HWB => match index {
            0 => (0.0..360.0).contains(&value),
            1 | 2 => (0.0..=100.0).contains(&value),
            _ => true,
        },
        ColorSpaceString::CIELAB => match index {
            0 => (0.0..=100.0).contains(&value),
            _ => true,
        },
        ColorSpaceString::LCH => match index {
            0 => (0.0..=100.0).contains(&value),
            1 => value >= 0.0,
            2 => (0.0..360.0).contains(&value),
            _ => true,
        },
        ColorSpaceString::OKLAB => match index {
            0 => (0.0..=1.0).contains(&value),
            _ => true,
        },
        ColorSpaceString::OKLCH => match index {
            0 => (0.0..=1.0).contains(&value),
            1 => value >= 0.0,
            2 => (0.0..360.0).contains(&value),
            _ => true,
        },
    };

    if in_range {
        None
    } else {
        Some(format!(
            "color component {index} is out of range for {:?}: {value}",
            space
        ))
    }
}

fn is_valid_hex_fallback(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
}

fn validate_circular_references(
    graph: &TokenGraph,
    diagnostics: &mut Vec<Diagnostic>,
    owner_to_common: &HashMap<OwnerId, &crate::ir::TokenCommon>,
) {
    for component in kosaraju_scc(&graph.ir_graph) {
        if component.len() < 2 {
            continue;
        }

        for node_index in component {
            if let Some(slot) = graph.ir_graph.node_weight(node_index) {
                if let Some(common) = owner_to_common.get(&slot.owner) {
                    diagnostics.push(make_diagnostic(
                        common,
                        DiagnosticCode::CircularReference,
                        "reference cycle detected in the token graph",
                        common.value_json_pointer(),
                    ));
                }
            }
        }
    }
}

fn make_diagnostic(
    common: &crate::ir::TokenCommon,
    code: DiagnosticCode,
    message: impl Into<String>,
    path: String,
) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code,
        message: message.into(),
        file_path: common
            .provenance
            .as_ref()
            .map(|provenance| provenance.source.clone()),
        path,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use test_log::test;

    use crate::{ParserContext, analysis::graph::TokenGraph, parsing::parse_document};

    #[test]
    fn builds_graph_from_ir_with_alias_and_ref_edges() {
        let source = json!({
            "base": {
                "color": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [1, 0, 0]
                    },
                    "$type": "color"
                }
            },
            "semantic": {
                "primary": {
                    "$value": "{base.color}",
                    "$type": "color"
                },
                "secondary": {
                    "$value": { "$ref": "#/base/color/$value" },
                    "$type": "color"
                }
            }
        })
        .to_string();

        let mut ctx = ParserContext::new(source);
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(ctx.errors.is_empty());

        let graph = TokenGraph::from_ir_document(&document);

        assert!(graph.node_count() >= 6);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn builds_graph_with_color_property_ref_edges() {
        let source = json!({
            "base": {
                "color": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [1, 0, 0]
                    },
                    "$type": "color"
                }
            },
            "semantic": {
                "brand": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": { "$ref": "#/base/color/$value/components" }
                    },
                    "$type": "color"
                }
            }
        })
        .to_string();

        let mut ctx = ParserContext::new(source);
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(ctx.errors.is_empty());

        let graph = TokenGraph::from_ir_document(&document);

        assert!(graph.edge_count() >= 1);
    }

    #[test]
    fn builds_complex_graph_from_full_spectrum_fixture() {
        let source =
            include_str!("../../tests/resources/tests/integration/full-spectrum.tokens.json");

        let mut ctx = ParserContext::new(source.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let graph = TokenGraph::from_ir_document(&document);

        assert!(graph.node_count() >= 40);
        assert!(graph.edge_count() >= 20);

        assert!(graph.count_edges_by_kind(crate::analysis::graph::EdgeKind::Alias) >= 1);
        assert!(graph.count_edges_by_kind(crate::analysis::graph::EdgeKind::JsonRef) >= 1);
        assert!(graph.count_edges_by_kind(crate::analysis::graph::EdgeKind::PropertyRef) >= 1);

        assert!(graph.validate(&document).is_empty());
    }

    #[test]
    fn validates_color_ranges_and_name_restrictions() {
        let source = json!({
            "bad.name": {
                "$type": "color",
                "$value": {
                    "colorSpace": "srgb",
                    "components": [2, 0, 0],
                    "alpha": 1.5,
                    "hex": "#abc"
                }
            }
        })
        .to_string();

        let mut ctx = ParserContext::new(source);
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let graph = TokenGraph::from_ir_document(&document);
        let diagnostics = graph.validate(&document);

        assert!(
            diagnostics.iter().any(
                |diagnostic| diagnostic.code == crate::errors::DiagnosticCode::InvalidTokenName
            )
        );
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidPropertyValue
            && diagnostic.path.ends_with("/$value/components/0")));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidPropertyValue
            && diagnostic.path.ends_with("/$value/alpha")));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidPropertyValue
            && diagnostic.path.ends_with("/$value/hex")));
    }

    #[test]
    fn validates_alias_chains_require_compatible_target_types() {
        let source = json!({
            "base": {
                "color": {
                    "$type": "color",
                    "primary": {
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [1, 0, 0]
                        }
                    }
                },
                "spacing": {
                    "$type": "dimension",
                    "sm": {
                        "$value": {
                            "value": 8,
                            "unit": "px"
                        }
                    }
                }
            },
            "semantic": {
                "ok": {
                    "$type": "color",
                    "$value": "{base.color.primary}"
                },
                "bad": {
                    "$type": "color",
                    "$value": "{base.spacing.sm}"
                }
            }
        })
        .to_string();

        let mut ctx = ParserContext::new(source);
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let graph = TokenGraph::from_ir_document(&document);
        let diagnostics = graph.validate(&document);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::errors::DiagnosticCode::InvalidReferenceTarget
                && diagnostic.path.ends_with("/semantic/bad/$value")
        }));
        assert!(!diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::errors::DiagnosticCode::InvalidReferenceTarget
                && diagnostic.path.ends_with("/semantic/ok/$value")
        }));
    }

    #[test]
    fn validates_alias_target_must_resolve_to_existing_token() {
        let source = json!({
            "semantic": {
                "missing": {
                    "$type": "color",
                    "$value": "{base.color.unknown}"
                }
            }
        })
        .to_string();

        let mut ctx = ParserContext::new(source);
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let graph = TokenGraph::from_ir_document(&document);
        let diagnostics = graph.validate(&document);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::errors::DiagnosticCode::InvalidReferenceTarget
                && diagnostic.path.ends_with("/semantic/missing/$value")
        }));
    }

    #[test]
    fn validates_alias_chain_when_all_target_types_match() {
        let source = json!({
            "base": {
                "colors": {
                    "$type": "color",
                    "primary": {
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [0.2, 0.4, 1.0],
                            "hex": "#3366ff"
                        }
                    }
                }
            },
            "semantic": {
                "stepOne": {
                    "$type": "color",
                    "$value": "{base.colors.primary}"
                },
                "stepTwo": {
                    "$type": "color",
                    "$value": "{semantic.stepOne}"
                }
            }
        })
        .to_string();

        let mut ctx = ParserContext::new(source);
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let graph = TokenGraph::from_ir_document(&document);
        let diagnostics = graph.validate(&document);

        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code != crate::errors::DiagnosticCode::InvalidReferenceTarget
        }));
    }

    #[test]
    fn validates_token_name_constraints() {
        let source = json!({
            "bad$segment": {
                "$type": "color",
                "$value": {
                    "colorSpace": "srgb",
                    "components": [1, 0, 0]
                }
            },
            "also": {
                "bad$segment": {
                    "$type": "color",
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [1, 0, 0]
                    }
                },
                "{bad}segment": {
                    "$type": "color",
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [1, 0, 0]
                    }
                },
                "bad.segment": {
                    "$type": "color",
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [1, 0, 0]
                    }
                }
            },
            "$root": {
                "$type": "color",
                "$value": {
                    "colorSpace": "srgb",
                    "components": [1, 0, 0]
                }
             }
        });

        let mut ctx = ParserContext::new(source.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");
        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );
        let graph = TokenGraph::from_ir_document(&document);
        let diagnostics = graph.validate(&document);
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidTokenName
            && diagnostic.path.ends_with("/bad$segment")));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidTokenName
            && diagnostic.path.ends_with("/also/bad$segment")));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidTokenName
            && diagnostic.path.ends_with("/also/{bad}segment")));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code
            == crate::errors::DiagnosticCode::InvalidTokenName
            && diagnostic.path.ends_with("/also/bad.segment")));
        assert!(diagnostics.iter().all(|diagnostic| diagnostic.code
            != crate::errors::DiagnosticCode::InvalidTokenName
            || !diagnostic.path.ends_with("/$root")));
    }
}
