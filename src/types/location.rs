//! Struct definitions and implementations for [`Location`].
//!
//! There may be special types of `Location` such as a moving
//! coordinate.

/// A [`Location`] is an interface type that represents a geographic
/// location of an object. Typically, this type is used in tandem with
/// the [`Node`](`super::node::Node`) type.
///
/// Altitude matters because it is used to compute the estimated fuel
/// costs for landing to or taking off from a location.
///
/// Float values are used to achieve a 5-decimal precision (0.00001),
/// which narrows the error margin to a meter.
#[derive(Debug)]
pub struct Location {
    pub longitude: f32,
    pub latitude: f32,
    pub altitude_meters: f32,
}
