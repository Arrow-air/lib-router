//! Stores the state of the router

use crate::generator::generate_nodes_near;
use crate::location::Location;
use crate::node::Node;
use crate::router::engine::{Algorithm, Router};
use crate::schedule::Calendar;
use crate::{haversine, status};
use chrono::{Duration, NaiveDateTime, TimeZone};
use once_cell::sync::OnceCell;
use ordered_float::OrderedFloat;
use prost_types::Timestamp;
use rrule::Tz;
use std::str::FromStr;
use svc_storage_client_grpc::client::{FlightPlanData, Vehicle, Vertiport};

/// Query struct for generating nodes near a location.
#[derive(Debug, Copy, Clone)]
pub struct NearbyLocationQuery {
    ///location
    pub location: Location,
    ///radius
    pub radius: f32,
    ///capacity
    pub capacity: i32,
}

/// Query struct to find a route between two nodes
#[derive(Debug, Copy, Clone)]
pub struct RouteQuery {
    ///aircraft
    pub aircraft: Aircraft,
    ///from
    pub from: &'static Node,
    ///to
    pub to: &'static Node,
}

/// Enum with all Aircraft types
#[derive(Debug, Copy, Clone)]
pub enum Aircraft {
    ///Cargo aircraft
    Cargo,
}
/// List of vertiport nodes for routing
pub static NODES: OnceCell<Vec<Node>> = OnceCell::new();
/// Cargo router
pub static ARROW_CARGO_ROUTER: OnceCell<Router> = OnceCell::new();

static ARROW_CARGO_CONSTRAINT: f32 = 75.0;
/// SF central location
pub static SAN_FRANCISCO: Location = Location {
    latitude: OrderedFloat(37.7749),
    longitude: OrderedFloat(-122.4194),
    altitude_meters: OrderedFloat(0.0),
};

/// Time to block vertiport for cargo loading and takeoff
pub const LOADING_AND_TAKEOFF_TIME_MIN: f32 = 10.0;
/// Time to block vertiport for cargo unloading and landing
pub const LANDING_AND_UNLOADING_TIME_MIN: f32 = 10.0;
/// Average speed of cargo aircraft
pub const AVG_SPEED_KMH: f32 = 60.0;

/// Creates all possible flight plans based on the given request
/// * `vertiport_depart` - Departure vertiport - svc-storage format
/// * `vertiport_arrive` - Arrival vertiport - svc-storage format
/// * `departure_time` - Departure time
/// * `arrival_time` - Arrival time
/// * `aircrafts` - Aircrafts serving the route and vertiports
/// # Returns
/// A vector of flight plans
pub fn get_possible_flights(
    vertiport_depart: Vertiport,
    vertiport_arrive: Vertiport,
    departure_time: Option<Timestamp>,
    arrival_time: Option<Timestamp>,
    aircrafts: Vec<Vehicle>,
) -> Result<Vec<FlightPlanData>, String> {
    //1. Find route and cost between requested vertiports
    if !is_router_initialized() {
        return Err("Router not initialized".to_string());
    }
    let (route, cost) = get_route(RouteQuery {
        from: get_node_by_id(&vertiport_depart.id).unwrap(),
        to: get_node_by_id(&vertiport_arrive.id).unwrap(),
        aircraft: Aircraft::Cargo,
    })
    .unwrap();
    if route.is_empty() {
        return Err("Route between vertiports not found".to_string());
    }
    println!("route distance: {:?}", cost);

    //2. calculate blocking times for each vertiport and aircraft
    let block_departure_vertiport_minutes = LOADING_AND_TAKEOFF_TIME_MIN;
    let block_arrival_vertiport_minutes = LANDING_AND_UNLOADING_TIME_MIN;
    let block_aircraft_minutes = estimate_flight_time_minutes(cost, Aircraft::Cargo);

    //3. check vertiport schedules and flight plans
    const SAMPLE_CAL: &str =
        "DTSTART:20221020T180000Z;DURATION:PT1H\nRRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR";
    let departure_vertiport_schedule = Calendar::from_str(SAMPLE_CAL).unwrap(); //TODO get from DB
    let arrival_vertiport_schedule = Calendar::from_str(SAMPLE_CAL).unwrap(); //TODO get from DB

    if departure_time.is_none() && arrival_time.is_none() {
        return Err("Either departure_time or arrival_time must be set".to_string());
    }

    let (departure_time, arrival_time) = if departure_time.is_some() {
        let departure_time = Tz::UTC.from_utc_datetime(&NaiveDateTime::from_timestamp(
            departure_time.as_ref().unwrap().seconds,
            departure_time.as_ref().unwrap().nanos as u32,
        ));
        (
            departure_time,
            departure_time + Duration::minutes(block_aircraft_minutes as i64),
        )
    } else {
        let arrival_time = Tz::UTC.from_utc_datetime(&NaiveDateTime::from_timestamp(
            arrival_time.as_ref().unwrap().seconds,
            arrival_time.as_ref().unwrap().nanos as u32,
        ));
        (
            arrival_time - Duration::minutes(block_aircraft_minutes as i64),
            arrival_time,
        )
    };
    let is_departure_vertiport_available = departure_vertiport_schedule.is_available_between(
        departure_time,
        departure_time + Duration::minutes(block_departure_vertiport_minutes as i64),
    );
    let is_arrival_vertiport_available = arrival_vertiport_schedule.is_available_between(
        arrival_time - Duration::minutes(block_arrival_vertiport_minutes as i64),
        arrival_time,
    );
    if !is_departure_vertiport_available {
        return Err("Departure vertiport not available".to_string());
    }
    if !is_arrival_vertiport_available {
        return Err("Arrival vertiport not available".to_string());
    }
    for _aircraft in aircrafts {
        let aircraft_schedule = Calendar::from_str(SAMPLE_CAL).unwrap(); //TODO get from aircraft.schedule
        let is_aircraft_available =
            aircraft_schedule.is_available_between(departure_time, arrival_time);
        if !is_aircraft_available {
            return Err("Aircraft not available".to_string());
        }
    }

    //4. TODO: check other constraints (cargo weight, number of passenger seats)

    //5. return draft flight plan(s)
    let flight_plans = vec![FlightPlanData {
        pilot_id: "".to_string(),
        vehicle_id: "".to_string(),
        cargo_weight: vec![],
        flight_distance: (cost * 1000.0) as u32,
        weather_conditions: "".to_string(),
        departure_vertiport_id: vertiport_depart.id,
        departure_pad_id: "".to_string(),
        destination_vertiport_id: vertiport_arrive.id,
        destination_pad_id: "".to_string(),
        scheduled_departure: Some(Timestamp {
            seconds: departure_time.timestamp(),
            nanos: departure_time.timestamp_subsec_nanos() as i32,
        }),
        scheduled_arrival: Some(Timestamp {
            seconds: arrival_time.timestamp(),
            nanos: arrival_time.timestamp_subsec_nanos() as i32,
        }),
        actual_departure: None,
        actual_arrival: None,
        flight_release_approval: None,
        flight_plan_submitted: None,
        approved_by: None,
        flight_status: 0,
        flight_priority: 0,
    }];
    Ok(flight_plans)
}

/// Estimates the time needed to travel between two locations including loading and unloading
/// Estimate should be rather generous to block resources instead of potentially overloading them
pub fn estimate_flight_time_minutes(distance_km: f32, aircraft: Aircraft) -> f32 {
    match aircraft {
        Aircraft::Cargo => {
            LOADING_AND_TAKEOFF_TIME_MIN
                + distance_km / AVG_SPEED_KMH * 60.0
                + LANDING_AND_UNLOADING_TIME_MIN
        }
    }
}

/// gets node by id
pub fn get_node_by_id(id: &str) -> Result<&'static Node, String> {
    let nodes = NODES.get().expect("Nodes not initialized");
    let node = nodes
        .iter()
        .find(|node| node.uid == id)
        .ok_or_else(|| "Node not found by id: ".to_owned() + id)?;
    Ok(node)
}

/// Initialize the router with vertiports from the storage service
pub fn init_router_from_vertiports(vertiports: &[Vertiport]) {
    let nodes = vertiports
        .iter()
        .map(|vertiport| Node {
            uid: vertiport.id.clone(),
            location: Location {
                latitude: OrderedFloat(vertiport.data.as_ref().unwrap().latitude),
                longitude: OrderedFloat(vertiport.data.as_ref().unwrap().longitude),
                altitude_meters: OrderedFloat(0.0),
            },
            forward_to: None,
            status: status::Status::Ok,
        })
        .collect();
    NODES.set(nodes).expect("Failed to set NODES");
    init_router();
}

/// Takes customer location (src) and required destination (dst) and returns a tuple with nearest vertiports to src and dst
pub fn get_nearest_vertiports<'a>(
    src_location: &'a Location,
    dst_location: &'a Location,
    vertiports: &'static Vec<Node>,
) -> (&'static Node, &'static Node) {
    let mut src_vertiport = &vertiports[0];
    let mut dst_vertiport = &vertiports[0];
    let mut src_distance = haversine::distance(src_location, &src_vertiport.location);
    let mut dst_distance = haversine::distance(dst_location, &dst_vertiport.location);
    for vertiport in vertiports {
        let new_src_distance = haversine::distance(src_location, &vertiport.location);
        let new_dst_distance = haversine::distance(dst_location, &vertiport.location);
        if new_src_distance < src_distance {
            src_distance = new_src_distance;
            src_vertiport = vertiport;
        }
        if new_dst_distance < dst_distance {
            dst_distance = new_dst_distance;
            dst_vertiport = vertiport;
        }
    }
    (src_vertiport, dst_vertiport)
}

/// Returns a list of nodes near the given location
pub fn get_nearby_nodes(query: NearbyLocationQuery) -> &'static Vec<Node> {
    NODES
        .set(generate_nodes_near(
            &query.location,
            query.radius,
            query.capacity,
        ))
        .expect("Failed to generate nodes");
    return NODES.get().unwrap();
}

/// Checks if router is initialized
pub fn is_router_initialized() -> bool {
    ARROW_CARGO_ROUTER.get().is_some()
}

/// Get route
pub fn get_route(req: RouteQuery) -> Result<(Vec<Location>, f32), &'static str> {
    let RouteQuery {
        from,
        to,
        aircraft: _,
    } = req;

    if ARROW_CARGO_ROUTER.get().is_none() {
        return Err("Arrow XL router not initialized. Try to initialize it first.");
    }
    let (cost, path) = ARROW_CARGO_ROUTER
        .get()
        .as_ref()
        .unwrap()
        .find_shortest_path(from, to, Algorithm::Dijkstra, None);
    let locations = path
        .iter()
        .map(|node_idx| {
            ARROW_CARGO_ROUTER
                .get()
                .as_ref()
                .unwrap()
                .get_node_by_id(*node_idx)
                .unwrap()
                .location
        })
        .collect::<Vec<Location>>();
    Ok((locations, cost))
}

/// Initializes the router for the given aircraft
pub fn init_router() -> &'static str {
    if NODES.get().is_none() {
        return "Nodes not initialized. Try to get some nodes first.";
    }
    if ARROW_CARGO_ROUTER.get().is_some() {
        return "Router already initialized. Try to use the router instead of initializing it.";
    }
    ARROW_CARGO_ROUTER
        .set(Router::new(
            NODES.get().as_ref().unwrap(),
            ARROW_CARGO_CONSTRAINT,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        ))
        .expect("Failed to initialize router");
    "Arrow Cargo router initialized."
}

#[cfg(test)]
mod router_tests {
    use super::{
        get_nearby_nodes, get_nearest_vertiports, get_route, init_router, Aircraft,
        NearbyLocationQuery, RouteQuery, SAN_FRANCISCO,
    };
    use crate::location::Location;
    use ordered_float::OrderedFloat;

    #[test]
    fn test_router() {
        let nodes = get_nearby_nodes(NearbyLocationQuery {
            location: SAN_FRANCISCO,
            radius: 25.0,
            capacity: 20,
        });

        //println!("nodes: {:?}", nodes);
        let init_res = init_router();
        println!("init_res: {:?}", init_res);
        let src_location = Location {
            latitude: OrderedFloat(37.52123),
            longitude: OrderedFloat(-122.50892),
            altitude_meters: OrderedFloat(20.0),
        };
        let dst_location = Location {
            latitude: OrderedFloat(37.81032),
            longitude: OrderedFloat(-122.28432),
            altitude_meters: OrderedFloat(20.0),
        };
        let (src, dst) = get_nearest_vertiports(&src_location, &dst_location, nodes);
        println!("src: {:?}, dst: {:?}", src.location, dst.location);
        let (route, cost) = get_route(RouteQuery {
            from: src,
            to: dst,
            aircraft: Aircraft::Cargo,
        })
        .unwrap();
        println!("route: {:?}", route);
        assert!(route.len() > 0, "Route should not be empty");
        assert!(cost > 0.0, "Cost should be greater than 0");
    }
}
