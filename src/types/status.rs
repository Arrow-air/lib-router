//! Definition for the [`Status`] type, implemented by an enum.

/// Represent the operating status of a [`super::node::Node`].
#[derive(Debug, PartialEq, Hash, Eq, Copy, Clone)]
#[allow(dead_code)]
pub enum Status {
    /// Indicate that the node is currently operating.
    Ok,
    /// Indicate that the node is currently down.
    Closed,
}
