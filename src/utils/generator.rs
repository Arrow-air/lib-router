//! A number of methods to generate random data for testing.

use crate::types::{location::Location, node::Node, status};
use ordered_float::OrderedFloat;
use quaternion::Quaternion;
use rand::{rngs::ThreadRng, Rng};
use uuid::Uuid;
use vecmath::Vector3;

//-----------------------------------------------------
// Constants
//-----------------------------------------------------
const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;
const RAD_TO_DEG: f32 = 180.0 / std::f32::consts::PI;

/// Generate a vector of random nodes.
///
/// # Arguments
/// * `capacity` - The number of nodes to generate.
///
/// # Returns
/// A vector of nodes.
pub fn generate_nodes(capacity: i32) -> Vec<Node> {
    let mut nodes = Vec::new();
    for _ in 0..capacity {
        nodes.push(generate_random_node());
    }
    nodes
}

/// Generate a vector of random nodes near a location.
///
/// # Arguments
/// * `location` - The location to generate nodes near.
/// * `radius` - The radius in kilometers to generate nodes within.
/// * `capacity` - The number of nodes to generate.
///
/// # Returns
/// A vector of nodes.
pub fn generate_nodes_near(location: &Location, radius: f32, capacity: i32) -> Vec<Node> {
    let mut nodes = Vec::new();
    for _ in 0..capacity {
        nodes.push(generate_random_node_near(location, radius));
    }
    nodes
}

/// Generate a single random node.
pub fn generate_random_node() -> Node {
    Node {
        uid: Uuid::new_v4().to_string(),
        location: generate_location(),
        forward_to: None,
        status: status::Status::Ok,
    }
}

/// Generate a random node near a location within radius in kilometers.
///
/// # Arguments
/// * `location` - The location to generate nodes near.
/// * `radius` - The radius in kilometers to generate nodes within.
///
/// # Returns
/// A node with a location near the given location.
pub fn generate_random_node_near(location: &Location, radius: f32) -> Node {
    Node {
        uid: Uuid::new_v4().to_string(),
        location: generate_location_near(location, radius),
        forward_to: None,
        status: status::Status::Ok,
    }
}

/// Generate a random location anywhere on earth.
///
/// # Returns
/// A random location anywhere on earth.
pub fn generate_location() -> Location {
    let mut rng = rand::thread_rng();
    let latitude = OrderedFloat(rng.gen_range(-90.0..=90.0));
    let longitude = OrderedFloat(rng.gen_range(-180.0..=180.0));
    let altitude_meters = OrderedFloat(rng.gen_range(0.0..=10000.0));
    Location {
        latitude,
        longitude,
        altitude_meters,
    }
}

/// Generate a random location near a given location and radius.
///
/// # Arguments
/// * `location` - The location to generate a random location near.
/// * `radius` - The radius in kilometers.
///
/// # Returns
/// A random location near the given location and radius.
pub fn generate_location_near(location: &Location, radius: f32) -> Location {
    let mut rng = rand::thread_rng();
    let (latitude, longitude) = gen_around_location(
        &mut rng,
        location.latitude.into_inner(),
        location.longitude.into_inner(),
        radius,
    );

    let altitude_meters = OrderedFloat(rng.gen_range(0.0..=10000.0));
    Location {
        latitude,
        longitude,
        altitude_meters,
    }
}

/// Generate a random location within a radius.
///
/// Source: [Reddit](https://www.reddit.com/r/rust/comments/f08lqu/comment/fgsxeik/)
///
/// # Arguments
/// * `rng` - The random number generator.
/// * `latitude` - The latitude of the location.
/// * `longitude` - The longitude of the location.
/// * `radius` - The radius in kilometers.
///
/// # Returns
/// A latitude and longitude pair.
///
/// # Notes
/// @GoodluckH: This function sometimes output invalid coordinates. I'm not sure why.
fn gen_around_location(
    rng: &mut ThreadRng,
    latitude: f32,
    longitude: f32,
    radius: f32,
) -> (OrderedFloat<f32>, OrderedFloat<f32>) {
    // Transform to cartesian coordinates
    let x = (DEG_TO_RAD * longitude).cos();
    let y = (DEG_TO_RAD * longitude).sin();
    let z = (DEG_TO_RAD * latitude).sin();

    // Generate random unit vector
    let x1 = 2.0 * rng.gen::<f32>() - 1.0;
    let y1 = 2.0 * rng.gen::<f32>() - 1.0;
    let z1 = 2.0 * rng.gen::<f32>() - 1.0;
    let len = (x1 * x1 + y1 * y1 + z1 * z1).sqrt();

    // Generate random angle
    let ang = 0.5 * (radius / 1000.0 * DEG_TO_RAD) * rng.gen::<f32>();
    let ca = ang.cos();
    let sa = ang.sin() / len;

    // Create Quaternion components
    let vec: Vector3<f32> = [x, y, z]; // Todo handle 0 case
    let q: Quaternion<f32> = (ca, [sa * x1, sa * y1, sa * z1]);
    let vec = quaternion::rotate_vector(q, vec);

    let r_lon = RAD_TO_DEG * vec[1].atan2(vec[0]);
    let r_lat = RAD_TO_DEG * vec[2].asin();
    if r_lat.is_nan() {
        return gen_around_location(rng, latitude, longitude, radius);
    }
    (OrderedFloat(r_lat), OrderedFloat(r_lon))
}

#[cfg(test)]
mod tests {
    use crate::haversine;

    use super::*;

    #[test]
    fn test_valid_coordinates() {
        let location = generate_location();
        assert!(location.latitude.into_inner() >= -90.0);
        assert!(location.latitude.into_inner() <= 90.0);
        assert!(location.longitude.into_inner() >= -180.0);
        assert!(location.longitude.into_inner() <= 180.0);
        assert!(location.altitude_meters.into_inner() >= 0.0);
        assert!(location.altitude_meters.into_inner() <= 10000.0);
    }

    /// Test that the distance between two locations is less than the radius.
    ///
    /// # Note
    /// Sometimes the test will fail. TODO: Double check the
    /// [`gen_around_location`] function for improvements.
    #[test]
    fn test_generate_location_near() {
        let location = generate_location();
        let location_near = generate_location_near(&location, 10.0);
        println!("Original location: {:?}", location);
        println!("Nearby location: {:?}", location_near);
        println!(
            "Distance: {}",
            haversine::distance(&location, &location_near)
        );
        assert!(haversine::distance(&location, &location_near) <= 10.0);
    }

    #[test]
    fn test_generate_random_nodes() {
        let node = generate_nodes(100);
        assert_eq!(node.len(), 100);
    }

    #[test]
    fn test_generate_random_nodes_near() {
        let location = generate_location();
        let nodes = generate_nodes_near(&location, 10.0, 100);
        assert_eq!(nodes.len(), 100);
        for node in nodes {
            assert!(haversine::distance(&location, &node.location) <= 10.0);
        }
    }
}