//! Helper functons for working with graphs.

use ordered_float::OrderedFloat;

use crate::types::node::{AsNode, Node};

/// Build edges among nodes.
///
/// The function will try to connect every node to every other node.
/// However, constraints can be added to the graph to prevent ineligible
/// nodes from being connected.
///
/// For example, if the constraint represents the max travel distance of
/// an aircraft, we only want to connect nodes that are within the max
/// travel distance. A constraint function is also needed to determine
/// if a connection is valid.
///
/// # Arguments
/// * `nodes` - A vector of nodes.
/// * `constraint` - Only nodes within a constraint can be connected.
/// * `constraint_function` - A function that takes two nodes and
///   returns a float to compare against `constraint`.
/// * `cost_function` - A function that computes the "weight" between
///   two nodes.
///
/// # Returns
/// A vector of edges in the format of (from_node, to_node, weight).
///
/// # Time Complexity
/// *O*(*n^2*) at worst if the constraint is not met for all nodes.
pub fn build_edges(
    nodes: &[impl AsNode],
    constraint: f32,
    constraint_function: fn(&dyn AsNode, &dyn AsNode) -> f32,
    cost_function: fn(&dyn AsNode, &dyn AsNode) -> f32,
) -> Vec<(&Node, &Node, OrderedFloat<f32>)> {
    let mut edges = Vec::new();
    for from in nodes {
        for to in nodes {
            if from.as_node() != to.as_node()
                && constraint_function(from.as_node(), to.as_node()) <= constraint
            {
                let cost = cost_function(from.as_node(), to.as_node());
                edges.push((from.as_node(), to.as_node(), OrderedFloat(cost)));
            }
        }
    }
    edges
}
