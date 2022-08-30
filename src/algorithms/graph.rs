//! Defines the graph and associated functions.
//!
//! Use a [`HashMap`] to represent a directed graph. The key is the
//! [`Node`], the value is a vector of `Nodes`. Using a HashMap allows
//! for fast lookup of `Node`s.
#![allow(dead_code)]

use crate::types::node::{AsNode, Node};
use std::collections::HashMap;

/// Represents a directed graph.
pub struct Graph<'a> {
    /// Represents the connective relationship among nodes.
    pub routes: HashMap<&'a Node, Vec<&'a Node>>,
}

impl<'a> Graph<'a> {
    /// Creates a new graph.
    ///
    /// The function will try to connect every node to every other node.
    /// However, constraints can be added to the graph to prevent
    /// ineligible nodes from being connected.
    ///
    /// For example, if the constraint represents the max travel
    /// distance of an aircraft, we only want to connect nodes that are
    /// within the max travel distance. A constraint function is also
    /// needed to determine if a connection is valid.
    ///
    /// # Arguments
    /// * `nodes` - A vector of nodes.
    /// * `constraint` - Only nodes within a constraint can be
    ///   connected.
    /// * `constraint_function` - A function that takes two nodes and
    ///   returns a float to compare against `constraint`.
    ///
    /// # Time Complexity
    /// *O*(*n^2*) at worst if the constraint is not met for all nodes.
    ///
    /// # Returns
    /// A new graph.
    pub fn new(
        nodes: &[&'a impl AsNode],
        constraint: f32,
        constraint_function: fn(&dyn AsNode, &dyn AsNode) -> f32,
    ) -> Graph<'a> {
        build_graph(nodes, constraint, constraint_function)
    }

    /// Finds the shortest path between two nodes given a custom
    /// algorithm.
    ///
    /// # Arguments
    /// * `from` - The starting node.
    /// * `to` - The ending node.
    /// * `algorithm` - The algorithm to use to find the shortest path.
    ///   Popular algorithms include Dijkstra's algorithm and A* algorithm.
    ///
    /// # Returns
    /// A vector of nodes that represents the shortest path.
    pub fn shortest_path(
        &self,
        from: &'a dyn AsNode,
        to: &'a dyn AsNode,
        algorithm: fn(&Graph<'a>, &dyn AsNode, &dyn AsNode) -> Vec<&'a dyn AsNode>,
    ) -> Vec<&'a dyn AsNode> {
        find_path(self, from, to, algorithm)
    }

    // TODO: mutate the graph by adding or deleting edges
}

//---------------------------------------------------------------
// Private functions
//---------------------------------------------------------------

/// See [`Graph::new`].
fn build_graph<'a>(
    nodes: &[&'a impl AsNode],
    constraint: f32,
    constraint_function: fn(&dyn AsNode, &dyn AsNode) -> f32,
) -> Graph<'a> {
    let mut graph = Graph {
        routes: HashMap::new(),
    };
    for node in nodes {
        graph.routes.insert(node.as_node(), Vec::new());
    }

    for from in nodes {
        for to in nodes {
            if from.as_node() != to.as_node()
                && constraint_function(from.as_node(), to.as_node()) <= constraint
            {
                graph
                    .routes
                    .get_mut(from.as_node())
                    .unwrap()
                    .push(to.as_node());
            }
        }
    }
    graph
}

/// See [`Graph::shortest_path`].
fn find_path<'a>(
    graph: &Graph<'a>,
    from: &'a dyn AsNode,
    to: &'a dyn AsNode,
    algorithm: fn(&Graph<'a>, &dyn AsNode, &dyn AsNode) -> Vec<&'a dyn AsNode>,
) -> Vec<&'a dyn AsNode> {
    algorithm(graph, from, to)
}
