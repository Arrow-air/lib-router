//! Fleet Routing Algorithm Library.
//! Handles routing and path-finding tasks.

mod types {
    pub mod location;
    pub mod node;
    pub mod status;
}

mod utils {
    pub mod generator;
    pub mod haversine;
}

pub use types::*;
pub use utils::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_add_one() {
        let val: u8 = 1;
        assert_eq!(val + 1, add_one(val));
    }
}
