//! Fleet Routing Algorithm Library.
//! Handles routing and path-finding tasks.

mod types {
    pub mod location;
    pub mod node;
    pub mod router;
    pub mod status;
}

mod utils {
    pub mod generator;
    pub mod graph;
    pub mod haversine;
}

pub use types::*;
pub use utils::*;
