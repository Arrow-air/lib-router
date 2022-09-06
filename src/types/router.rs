//! The core of the router library.
//!
//! The engine module builds a graph given an input of nodes. Path
//! finding algorithms are also provided to find the shortest path
//! between two nodes.

/// The router engine module.
pub mod engine {
    use std::collections::HashMap;

    use ordered_float::OrderedFloat;
    use petgraph::{algo::astar, graph::NodeIndex, stable_graph::StableDiGraph};

    use crate::{
        edge::Edge,
        types::node::{AsNode, Node},
        utils::graph::build_edges,
    };

    /// A Router struct contains a graph of nodes and also a hashmap
    /// that maps a node to its index in the graph.
    #[derive(Debug)]
    pub struct Router<'a> {
        graph: StableDiGraph<&'a Node, OrderedFloat<f32>>,
        node_indices: HashMap<&'a Node, NodeIndex>,
        edges: Vec<Edge<'a>>,
    }

    /// Path finding algorithms.
    #[derive(Debug, Copy, Clone)]
    pub enum Algorithm {
        /// The Dikstra algorithm.
        Dijkstra,
        /// The A Star algorithm.
        AStar,
    }

    impl Router<'_> {
        /// Creates a new router with the given graph.
        pub fn new(
            nodes: &[impl AsNode],
            constraint: f32,
            constraint_function: fn(&dyn AsNode, &dyn AsNode) -> f32,
            cost_function: fn(&dyn AsNode, &dyn AsNode) -> f32,
        ) -> Router {
            println!("[1/4] Initializing the router engine...");
            println!("[2/4] Building edges...");

            let edges = build_edges(nodes, constraint, constraint_function, cost_function);
            let mut node_indices = HashMap::new();
            let mut graph = StableDiGraph::new();

            println!("[3/4] Building the graph...");
            for edge in &edges {
                let from_index = *node_indices
                    .entry(edge.from)
                    .or_insert_with(|| graph.add_node(edge.from));
                let to_index = *node_indices
                    .entry(edge.to)
                    .or_insert_with(|| graph.add_node(edge.to));
                graph.add_edge(from_index, to_index, edge.cost);
            }

            println!("[4/4] Finalizing the router setup...");
            for node in nodes {
                if !node_indices.contains_key(node.as_node()) {
                    let index = graph.add_node(node.as_node());
                    node_indices.insert(node.as_node(), index);
                }
            }

            println!("✨Done! Router engine is ready to use.");
            Router {
                graph,
                node_indices,
                edges,
            }
        }

        /// Get the NodeIndex struct for a given node. The NodeIndex
        /// struct is used to reference things in the graph.
        pub fn get_node_index(&self, node: &Node) -> Option<NodeIndex> {
            self.node_indices.get(node).cloned()
        }

        /// Return the number of edges in the graph.
        pub fn get_edge_count(&self) -> usize {
            self.graph.edge_count()
        }

        /// Find the shortest path between two nodes.
        ///
        /// The petgraph's dijkstra algorithm is very indentical to the
        /// a star algorithm, so we can use the same function for both.
        /// The only difference might be how the heuristic function is
        /// implemented.
        ///
        /// # Arguments
        /// * `from` - The node to start from.
        /// * `to` - The node to end at.
        /// * `algorithm` - The algorithm to use.
        /// * `heuristic` - The heuristic function to use.
        ///
        /// # Returns
        /// A tuple of the total cost and the path consisting of node
        /// indeces.
        ///
        /// An empty path with a total cost of 0.0 returned if no path
        /// is found.
        ///
        /// An empty path with a total cost of -1.0 is returned if
        /// either the `from` or `to` node is not found.
        pub fn find_shortest_path(
            &self,
            from: &Node,
            to: &Node,
            algorithm: Algorithm,
            heuristic_function: Option<fn(NodeIndex) -> f32>,
        ) -> (f32, Vec<NodeIndex>) {
            if self.get_node_index(from).is_some() && self.get_node_index(to).is_some() {
                let from_index = self.get_node_index(from).unwrap();
                let to_index = self.get_node_index(to).unwrap();
                match algorithm {
                    Algorithm::Dijkstra => astar(
                        &self.graph,
                        from_index,
                        |finish| finish == to_index,
                        |e| (*e.weight()).into_inner(),
                        heuristic_function.unwrap_or(|_| 0.0),
                    )
                    .unwrap_or((0.0, Vec::new())),

                    Algorithm::AStar => astar(
                        &self.graph,
                        from_index,
                        |finish| finish == to_index,
                        |e| (*e.weight()).into_inner(),
                        heuristic_function.unwrap_or(|_| 0.0),
                    )
                    .unwrap_or((0.0, Vec::new())),
                }
            } else {
                (-1.0, Vec::new())
            }
        }

        /// Get the number of nodes in the graph.
        pub fn get_node_count(&self) -> usize {
            self.graph.node_count()
        }

        /// Get all the edges in the graph.
        pub fn get_edges<'a>(&self) -> &'a Vec<Edge> {
            &self.edges
        }
    }
}

#[cfg(test)]
mod router_tests {
    use crate::{
        location::Location,
        node::{AsNode, Node},
        router::engine::Algorithm,
        types::router::engine::Router,
        utils::{generator::generate_nodes_near, haversine},
    };

    use ordered_float::OrderedFloat;

    const SAN_FRANCISCO: Location = Location {
        latitude: OrderedFloat(37.7749),
        longitude: OrderedFloat(-122.4194),
        altitude_meters: OrderedFloat(0.0),
    };
    const CAPACITY: i32 = 500;

    #[test]
    fn test_correct_node_count() {
        let nodes = generate_nodes_near(&SAN_FRANCISCO, 10000.0, CAPACITY);

        let router = Router::new(
            &nodes,
            10000.0,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        );

        assert_eq!(CAPACITY as usize, router.get_node_count());
    }

    /// The graph has no edges.
    #[test]
    fn test_shortest_path_disconnected_graph() {
        let nodes = generate_nodes_near(&SAN_FRANCISCO, 10000.0, CAPACITY);

        let router = Router::new(
            &nodes,
            0.0,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        );

        let from = &nodes[0];
        let to = &nodes[1];

        let (cost, path) = router.find_shortest_path(from, to, Algorithm::AStar, None);

        assert_eq!(cost, 0.0);
        assert_eq!(router.get_edge_count(), 0);
        assert_eq!(router.get_node_count(), CAPACITY as usize);
        assert_eq!(path.len(), 0);
    }

    /// Find the shortest path between two nodes.
    ///
    /// The following points are random coordinates in San Francisco.
    ///
    /// point 1: 37.777843, -122.468207
    ///
    /// point 2: 37.778339, -122.460395
    ///
    /// point 3: 37.780596, -122.434904
    ///
    /// point 4: 37.774397, -122.445366
    ///
    /// The shortest path from 1 to 3 should be 1 -> 3
    #[test]
    fn test_shortest_path_has_path() {
        let nodes = vec![
            Node {
                uid: "1".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.777843),
                    longitude: OrderedFloat(-122.468207),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "2".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.778339),
                    longitude: OrderedFloat(-122.460395),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "3".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.780596),
                    longitude: OrderedFloat(-122.434904),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "4".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.774397),
                    longitude: OrderedFloat(-122.445366),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
        ];

        let router = Router::new(
            &nodes,
            100.0,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        );

        assert_eq!(4, router.get_node_count());
        assert_eq!(
            router.get_node_count() * router.get_node_count() - 4,
            router.get_edge_count()
        );

        let (cost, path) = router.find_shortest_path(&nodes[0], &nodes[2], Algorithm::AStar, None);
        assert_eq!(
            cost,
            haversine::distance(&nodes[0].location, &nodes[2].location)
        );
        // should be 1 -> 3
        assert_eq!(path.len(), 2);
        assert_eq!(
            path,
            vec![
                router.get_node_index(&nodes[0]).unwrap(),
                router.get_node_index(&nodes[2]).unwrap()
            ]
        );
    }

    /// Find the shortest path between a point in San Francisco and a
    /// point in New York.
    ///
    /// The following points are random coordinates in San Francisco
    /// except for point 4.
    ///
    /// point 1: 37.777843, -122.468207
    ///
    /// point 2: 37.778339, -122.460395
    ///
    /// point 3: 37.780596, -122.434904
    ///
    /// point 4: 40.738820, -73.990440
    ///
    /// There should not be any path from 1 to 4 if we constraint our
    /// flight distance to 100 kilometers.
    #[test]
    fn test_shortest_path_no_path() {
        let nodes = vec![
            Node {
                uid: "1".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.777843),
                    longitude: OrderedFloat(-122.468207),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "2".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.778339),
                    longitude: OrderedFloat(-122.460395),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "3".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.780596),
                    longitude: OrderedFloat(-122.434904),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "4".to_string(),
                location: Location {
                    latitude: OrderedFloat(40.738820),
                    longitude: OrderedFloat(-73.990440),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
        ];

        let router = Router::new(
            &nodes,
            100.0,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        );

        assert_eq!(4, router.get_node_count());
        assert_eq!(
            (router.get_node_count() - 1) * (router.get_node_count() - 1) - 3,
            router.get_edge_count()
        );

        let (cost, path) = router.find_shortest_path(&nodes[0], &nodes[3], Algorithm::AStar, None);
        assert_eq!(cost, 0.0);
        // should be 0
        assert_eq!(path.len(), 0);
        assert_eq!(path, vec![]);
    }

    /// Test invalid node queries.
    #[test]
    fn test_invalid_node_shortest_path() {
        let nodes = vec![
            Node {
                uid: "1".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.777843),
                    longitude: OrderedFloat(-122.468207),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "2".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.778339),
                    longitude: OrderedFloat(-122.460395),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "3".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.780596),
                    longitude: OrderedFloat(-122.434904),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "4".to_string(),
                location: Location {
                    latitude: OrderedFloat(40.738820),
                    longitude: OrderedFloat(-73.990440),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
        ];

        let not_in_graph_node = Node {
            uid: "5".to_string(),
            location: Location {
                latitude: OrderedFloat(40.738820),
                longitude: OrderedFloat(-73.990440),
                altitude_meters: OrderedFloat(0.0),
            },
            forward_to: None,
            status: crate::status::Status::Ok,
        };

        let router = Router::new(
            &nodes,
            10000.0,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        );

        let (cost, path) =
            router.find_shortest_path(&nodes[0], &not_in_graph_node, Algorithm::AStar, None);

        assert_eq!(cost, -1.0);
        assert_eq!(path.len(), 0);
    }

    /// Test get_edges
    #[test]
    fn test_get_edges() {
        let nodes = vec![
            Node {
                uid: "1".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.777843),
                    longitude: OrderedFloat(-122.468207),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "2".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.778339),
                    longitude: OrderedFloat(-122.460395),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "3".to_string(),
                location: Location {
                    latitude: OrderedFloat(37.780596),
                    longitude: OrderedFloat(-122.434904),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
            Node {
                uid: "4".to_string(),
                location: Location {
                    latitude: OrderedFloat(40.738820),
                    longitude: OrderedFloat(-73.990440),
                    altitude_meters: OrderedFloat(0.0),
                },
                forward_to: None,
                status: crate::status::Status::Ok,
            },
        ];

        let router = Router::new(
            &nodes,
            10000.0,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        );

        let edges = router.get_edges();
        assert_eq!(edges.len(), 12);
        assert_eq!(edges[0].to.get_uid(), "2");
        assert_eq!(edges[1].to.get_uid(), "3");
    }
}
