//! Definition of the `Edge` type.
use ordered_float::OrderedFloat;

use crate::node::Node;

/// An edge is a connection between two nodes.
/// The cost represents the "weight" of the edge.
#[derive(Debug, PartialEq, Hash, Eq)]
pub struct Edge<'a> {
    /// One end of the edge.
    pub from: &'a Node,

    /// The other end of the edge.
    pub to: &'a Node,

    /// The weight of the edge.
    pub cost: OrderedFloat<f32>,
}
