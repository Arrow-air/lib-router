//! Definition for the [`Status`] type, implemented by an enum.

/// Represents the operating status of a [`super::node::Node`].
#[derive(Debug, PartialEq, Hash, Eq)]
#[allow(dead_code)]
pub enum Status {
    Ok,
    Closed,
}
