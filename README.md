![Arrow Banner](https://github.com/Arrow-air/.github/raw/main/profile/assets/arrow_v2_twitter-banner_neu.png)

# Arrow Router Library



![Rust
Checks](https://github.com/arrow-air/lib-router/actions/workflows/rust_ci.yml/badge.svg?branch=main)
![Python Flake8](https://github.com/arrow-air/lib-router/actions/workflows/python_ci.yml/badge.svg?branch=main)
![Arrow DAO
Discord](https://img.shields.io/discord/853833144037277726?style=plastic)


## :telescope: Overview

The router library provides an engine for routing queries.

Under the hood, the router engine builds a graph of nodes and edges. A node represents a "stop" like a vertipad, vertiport, or a rooftop where aircraft can land and/or take off. An edge represents a "route" between two nodes. The router engine provides a number of functionalities to query the graph, such as finding the shortest path between two nodes, or finding all nodes within a certain distance of a given node (WIP).

Directory:
- `src/`: Source Code and Unit Tests
- `tests/`: Integration Tests
- `docs/`: Library Documentation

## :gear: Installation

Install Rust with [Rustup](https://www.rust-lang.org/tools/install).

```bash
cargo test
```

## Make

### Build and test

To ensure consistent build and test outputs, Arrow provides a Docker image with all required software installed to build and test Rust projects.
Using the Makefile, you can easily test and build your code.

```bash
# Run tests
make test

# Run build
make build
```

### Formatting

The Arrow docker image has some formatting tools installed which can fix your code formatting for you.
Using the Makefile, you can easily run the formatters on your code.
Make sure to commit your code before running these commands, as they might not always result in a desired outcome.

```bash
# Format TOML files
make toml-tidy

# Format Rust files
make rust-tidy

# Format Python files
make python-tidy

# Format all at once
make tidy
```

### Other make targets

There are additional make targets available. You can find all possible targets by running make without a target or use `make help`

## :scroll: Documentation
The following documents are relevant to this library:
- [Requirements & User Stories](https://docs.google.com/spreadsheets/d/1Ad238NAEj6QUzgsjPTRRFJy6NiQVQt2e7affwVVDAFo/edit?usp=sharing)
- [Software Design Document](./docs/sdd.md)

## :compass: Roadmap
### Engine APIs:
- [ ] `add_node`: Add a node to the graph
- [ ] `add_edge`: Add an edge to the graph
- [ ] `remove_node`: Remove a node from the graph
- [ ] `remove_edge`: Remove an edge from the graph
- [x] `get_node_by_uid`: Get a node by its unique ID
- [ ] `get_edges_by_node_uid`: Get all edges connected to a node
- [ ] `update_weight`: Update the weight of an edge given two nodes
- [x] `has_node`: Check if a node exists in the graph (this can be achieved by calling the `get_node_index` function. If the node does not exist, it will just return None)
- [ ] `has_edge`: Check if an edge exists in the graph
- [x] ~~`get_nodes_within_distance`: Get all nodes within a certain distance of a given node~~ *This functionality is now expected to be implemented by the user.*

### Tests:
- [ ] Integration tests.
- [ ] Graphical representation: possibly using [leaflet.js](https://leafletjs.com/) or similar.
## :busts_in_silhouette: Arrow DAO
Learn more about us:
- [Website](https://www.arrowair.com/)
- [Arrow Docs](https://www.arrowair.com/docs/intro)
- [Discord](https://discord.com/invite/arrow)
