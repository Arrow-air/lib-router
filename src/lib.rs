//! Fleet Routing Algorithm Library.
//! Handles routing and path-finding tasks.

mod types {
    pub mod location;
    pub mod node;
    pub mod status;
}

mod utils {
    pub mod haversine;
}

/// Adds one to a number.
///
/// # Arguments
///
/// * `val` - Any U8 number.
///
pub fn add_one(val: u8) -> u8 {
    val + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_add_one() {
        let val: u8 = 1;
        assert_eq!(val + 1, add_one(val));
    }
}
