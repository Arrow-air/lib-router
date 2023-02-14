//! Stores the state of the router

use crate::generator::generate_nodes_near;
use crate::location::Location;
use crate::node::Node;
use crate::router::engine::{Algorithm, Router};
use crate::schedule::Calendar;
use crate::{haversine, status};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone};
use once_cell::sync::OnceCell;
use ordered_float::OrderedFloat;
use prost_types::Timestamp;
use rrule::Tz;
use std::str::FromStr;
use svc_storage_client_grpc::flight_plan::{Data as FlightPlanData, Object as FlightPlan};
use svc_storage_client_grpc::vehicle::Object as Vehicle;
use svc_storage_client_grpc::vertiport::Object as Vertiport;

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
/// Minimum time between suggested flight plans in case of multiple flights available
pub const FLIGHT_PLAN_GAP_MINUTES: f32 = 5.0;
/// Max amount of flight plans to return in case of large time window and multiple flights available
pub const MAX_RETURNED_FLIGHT_PLANS: i64 = 10;

/// Helper function to check if two time ranges overlap (touching ranges are not considered overlapping)
/// All parameters are in seconds since epoch
fn time_ranges_overlap(start1: i64, end1: i64, start2: i64, end2: i64) -> bool {
    start1 < end2 && start2 < end1
}

/// Helper function to create a flight plan data object from 5 required parameters
fn create_flight_plan_data(
    vehicle: &Vehicle,
    departure_vertiport: &Vertiport,
    arrival_vertiport: &Vertiport,
    departure_time: DateTime<Tz>,
    arrival_time: DateTime<Tz>,
) -> FlightPlanData {
    FlightPlanData {
        pilot_id: "".to_string(),
        vehicle_id: vehicle.id.clone(),
        cargo_weight_grams: vec![],
        weather_conditions: None,
        departure_vertiport_id: Some(departure_vertiport.id.clone()),
        destination_vertiport_id: Some(arrival_vertiport.id.clone()),
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
        departure_vertipad_id: "".to_string(),
        destination_vertipad_id: "".to_string(),
        flight_distance_meters: 0,
    }
}

/// Checks if a vehicle is available for a given time window date_from to
///    date_from + flight_duration_minutes (this includes takeoff and landing time)
/// This checks both static schedule of the aircraft and existing flight plans which might overlap.
pub fn is_vehicle_available(
    vehicle: &Vehicle,
    date_from: DateTime<Tz>,
    flight_duration_minutes: i64,
    existing_flight_plans: &[FlightPlan],
) -> bool {
    let vehicle_schedule = Calendar::from_str(
        vehicle
            .data
            .as_ref()
            .unwrap()
            .schedule
            .as_ref()
            .unwrap()
            .as_str(),
    )
    .unwrap();
    let date_to = date_from + Duration::minutes(flight_duration_minutes);
    //check if vehicle is available as per schedule
    if !vehicle_schedule.is_available_between(date_from, date_to) {
        return false;
    }
    //check if vehicle is available as per existing flight plans
    let conflicting_flight_plans_count = existing_flight_plans
        .iter()
        .filter(|flight_plan| {
            flight_plan.data.as_ref().unwrap().vehicle_id == vehicle.id
                && time_ranges_overlap(
                    flight_plan
                        .data
                        .as_ref()
                        .unwrap()
                        .scheduled_departure
                        .as_ref()
                        .unwrap()
                        .seconds,
                    flight_plan
                        .data
                        .as_ref()
                        .unwrap()
                        .scheduled_arrival
                        .as_ref()
                        .unwrap()
                        .seconds,
                    date_from.timestamp(),
                    date_to.timestamp(),
                )
        })
        .count();
    if conflicting_flight_plans_count > 0 {
        return false;
    }
    true
}

/// Checks if vertiport is available for a given time window from date_from to date_from + duration
/// of how long vertiport is blocked by takeoff/landing
/// This checks both static schedule of vertiport and existing flight plans which might overlap.
/// is_departure_vertiport is used to determine if we are checking for departure or arrival vertiport
pub fn is_vertiport_available(
    vertiport: &Vertiport,
    date_from: DateTime<Tz>,
    existing_flight_plans: &[FlightPlan],
    is_departure_vertiport: bool,
) -> bool {
    let vertiport_schedule = Calendar::from_str(
        vertiport
            .data
            .as_ref()
            .unwrap()
            .schedule
            .as_ref()
            .unwrap()
            .as_str(),
    )
    .unwrap();
    let block_vertiport_minutes: i64 = if is_departure_vertiport {
        LOADING_AND_TAKEOFF_TIME_MIN as i64
    } else {
        LANDING_AND_UNLOADING_TIME_MIN as i64
    };
    let date_to = date_from + Duration::minutes(block_vertiport_minutes);
    //check if vertiport is available as per schedule
    if !vertiport_schedule.is_available_between(date_from, date_to) {
        return false;
    }
    let conflicting_flight_plans_count = existing_flight_plans
        .iter()
        .filter(|flight_plan| {
            if is_departure_vertiport {
                flight_plan
                    .data
                    .as_ref()
                    .unwrap()
                    .departure_vertiport_id
                    .clone()
                    .unwrap()
                    == vertiport.id
                    && flight_plan
                        .data
                        .as_ref()
                        .unwrap()
                        .scheduled_departure
                        .as_ref()
                        .unwrap()
                        .seconds
                        > date_from.timestamp() - block_vertiport_minutes * 60
                    && flight_plan
                        .data
                        .as_ref()
                        .unwrap()
                        .scheduled_departure
                        .as_ref()
                        .unwrap()
                        .seconds
                        < date_to.timestamp() + block_vertiport_minutes * 60
            } else {
                flight_plan
                    .data
                    .as_ref()
                    .unwrap()
                    .destination_vertiport_id
                    .clone()
                    .unwrap()
                    == vertiport.id
                    && flight_plan
                        .data
                        .as_ref()
                        .unwrap()
                        .scheduled_arrival
                        .as_ref()
                        .unwrap()
                        .seconds
                        > date_from.timestamp() - block_vertiport_minutes * 60
                    && flight_plan
                        .data
                        .as_ref()
                        .unwrap()
                        .scheduled_arrival
                        .as_ref()
                        .unwrap()
                        .seconds
                        < date_to.timestamp() + block_vertiport_minutes * 60
            }
        })
        .count();
    debug!(
        "Checking {} is departure: {}, is available for {} - {}? {}",
        vertiport.id,
        is_departure_vertiport,
        date_from,
        date_to,
        conflicting_flight_plans_count == 0,
    );
    conflicting_flight_plans_count == 0
}

/// Gets vehicle location (vertiport_id) at given timestamp
/// Returns tuple of (vertiport_id, minutes_to_arrival)
/// If minutes_to_arrival is 0, vehicle is parked at the vertiport,
/// otherwise it is in flight to the vertiport and should arrive in minutes_to_arrival
pub fn get_vehicle_scheduled_location(
    vehicle: &Vehicle,
    timestamp: DateTime<Tz>,
    existing_flight_plans: &[FlightPlan],
) -> (String, u32) {
    let mut vehicle_flight_plans = existing_flight_plans
        .iter()
        .filter(|flight_plan| {
            flight_plan.data.as_ref().unwrap().vehicle_id == vehicle.id
                && flight_plan
                    .data
                    .as_ref()
                    .unwrap()
                    .scheduled_departure
                    .as_ref()
                    .unwrap()
                    .seconds
                    <= timestamp.timestamp()
        })
        .collect::<Vec<&FlightPlan>>();
    vehicle_flight_plans.sort_by(|a, b| {
        b.data
            .as_ref()
            .unwrap()
            .scheduled_departure
            .as_ref()
            .unwrap()
            .seconds
            .cmp(
                &a.data
                    .as_ref()
                    .unwrap()
                    .scheduled_departure
                    .as_ref()
                    .unwrap()
                    .seconds,
            )
    });
    if vehicle_flight_plans.is_empty() {
        return (
            vehicle
                .data
                .as_ref()
                .unwrap()
                .last_vertiport_id
                .as_ref()
                .unwrap()
                .clone(),
            0,
        );
    }
    let vehicle_flight_plan = vehicle_flight_plans.first().unwrap();
    debug!(
        "Vehicle {} had last flight plan {} with destination {}",
        vehicle.id,
        vehicle_flight_plan.id.clone(),
        vehicle_flight_plan
            .data
            .as_ref()
            .unwrap()
            .destination_vertiport_id
            .as_ref()
            .unwrap()
    );
    let mut minutes_to_arrival = (vehicle_flight_plan
        .data
        .as_ref()
        .unwrap()
        .scheduled_arrival
        .as_ref()
        .unwrap()
        .seconds
        - timestamp.timestamp())
        / 60;
    if minutes_to_arrival < 0 {
        minutes_to_arrival = 0;
    }
    (
        vehicle_flight_plan
            .data
            .as_ref()
            .unwrap()
            .destination_vertiport_id
            .as_ref()
            .unwrap()
            .to_string(),
        minutes_to_arrival as u32,
    )
}

/// Creates all possible flight plans based on the given request
/// * `vertiport_depart` - Departure vertiport - svc-storage format
/// * `vertiport_arrive` - Arrival vertiport - svc-storage format
/// * `earliest_departure_time` - Earliest departure time of the time window
/// * `latest_arrival_time` - Latest arrival time of the time window
/// * `aircrafts` - Aircrafts serving the route and vertiports
/// # Returns
/// A vector of flight plans
pub fn get_possible_flights(
    vertiport_depart: Vertiport,
    vertiport_arrive: Vertiport,
    earliest_departure_time: Option<Timestamp>,
    latest_arrival_time: Option<Timestamp>,
    vehicles: Vec<Vehicle>,
    existing_flight_plans: Vec<FlightPlan>,
) -> Result<Vec<FlightPlanData>, String> {
    info!("Finding possible flights");
    if earliest_departure_time.is_none() || latest_arrival_time.is_none() {
        error!("Both earliest departure and latest arrival time must be specified");
        return Err(
            "Both earliest departure and latest arrival time must be specified".to_string(),
        );
    }
    //1. Find route and cost between requested vertiports
    info!("[1/5]: Finding route between vertiports");
    if !is_router_initialized() {
        error!("Router not initialized");
        return Err("Router not initialized".to_string());
    }
    let (route, cost) = get_route(RouteQuery {
        from: get_node_by_id(&vertiport_depart.id)?,
        to: get_node_by_id(&vertiport_arrive.id)?,
        aircraft: Aircraft::Cargo,
    })?;
    debug!("Route: {:?}", route);
    debug!("Cost: {:?}", cost);
    if route.is_empty() {
        error!("No route found");
        return Err("Route between vertiports not found".to_string());
    }

    //2. calculate blocking times for each vertiport and aircraft
    info!("[2/5]: Calculating blocking times");

    let block_aircraft_minutes = estimate_flight_time_minutes(cost, Aircraft::Cargo);
    let block_aircraft_and_vertiports_minutes =
        block_aircraft_minutes + LOADING_AND_TAKEOFF_TIME_MIN + LANDING_AND_UNLOADING_TIME_MIN;

    debug!(
        "Estimated flight time in minutes: {}, with takeoff and landing: {}",
        block_aircraft_minutes, block_aircraft_and_vertiports_minutes
    );

    let time_window_duration_minutes: f32 = ((latest_arrival_time.as_ref().unwrap().seconds
        - earliest_departure_time.as_ref().unwrap().seconds)
        / 60) as f32;
    debug!(
        "Time window duration in minutes: {}",
        time_window_duration_minutes
    );
    if (time_window_duration_minutes - block_aircraft_and_vertiports_minutes) < 0.0 {
        error!("Time window too small to schedule flight");
        return Err("Time window too small to schedule flight".to_string());
    }
    let mut num_flight_options: i64 = ((time_window_duration_minutes
        - block_aircraft_and_vertiports_minutes)
        / FLIGHT_PLAN_GAP_MINUTES)
        .floor() as i64
        + 1;
    if num_flight_options > MAX_RETURNED_FLIGHT_PLANS {
        num_flight_options = MAX_RETURNED_FLIGHT_PLANS;
    }
    //3. check vertiport schedules and flight plans
    info!(
        "[3/5]: Checking vertiport schedules and flight plans for {} possible flight plans",
        num_flight_options
    );
    let mut flight_plans: Vec<FlightPlanData> = vec![];
    for i in 0..num_flight_options {
        let departure_time = Tz::UTC.from_utc_datetime(
            &NaiveDateTime::from_timestamp_opt(
                earliest_departure_time.as_ref().unwrap().seconds
                    + i * 60 * FLIGHT_PLAN_GAP_MINUTES as i64,
                earliest_departure_time.as_ref().unwrap().nanos as u32,
            )
            .ok_or("Invalid departure_time")?,
        );
        let arrival_time =
            departure_time + Duration::minutes(block_aircraft_and_vertiports_minutes as i64);
        let is_departure_vertiport_available = is_vertiport_available(
            &vertiport_depart,
            departure_time,
            &existing_flight_plans,
            true,
        );
        let is_arrival_vertiport_available = is_vertiport_available(
            &vertiport_arrive,
            arrival_time - Duration::minutes(LANDING_AND_UNLOADING_TIME_MIN as i64),
            &existing_flight_plans,
            false,
        );
        debug!(
            "DEPARTURE TIME: {}, ARRIVAL TIME: {}, {}, {}",
            departure_time,
            arrival_time,
            is_departure_vertiport_available,
            is_arrival_vertiport_available
        );
        if !is_departure_vertiport_available {
            info!(
                "Departure vertiport not available for departure time {}",
                departure_time
            );
            continue;
        }
        if !is_arrival_vertiport_available {
            info!(
                "Arrival vertiport not available for departure time {}",
                departure_time
            );
            continue;
        }
        //check if aircraft will be parked at the vertiport at the time of departure -
        //  AND check if aircraft has availability for the flight - then add FP
        //if not check if aircraft is en-route to the vertiport AND will have availability, if so, continue the cycle to next iteration
        // otherwise break cycle. We need to get to advanced deadhead scenarios
        let mut available_vehicle: Option<&Vehicle> = None;
        for vehicle in &vehicles {
            debug!(
                "Checking vehicle id:{} for departure time: {}",
                &vehicle.id, departure_time
            );
            let (vehicle_vertiport_id, minutes_to_arrival) =
                get_vehicle_scheduled_location(vehicle, departure_time, &existing_flight_plans);
            if vehicle_vertiport_id != vertiport_depart.id || minutes_to_arrival > 0 {
                debug!(
                    "Vehicle id:{} not available at location for requested time {}. It is/will be at vertiport id: {} in {} minutes",
                    &vehicle.id, departure_time, vehicle_vertiport_id, minutes_to_arrival
                );
                continue;
            }
            let is_vehicle_available = is_vehicle_available(
                vehicle,
                departure_time,
                block_aircraft_and_vertiports_minutes as i64,
                &existing_flight_plans,
            );
            if !is_vehicle_available {
                debug!(
                    "Vehicle id:{} not available for departure time: {} and duration {} minutes",
                    &vehicle.id, departure_time, block_aircraft_and_vertiports_minutes
                );
                continue;
            }
            //when vehicle is available, break the "vehicles" loop early and add flight plan
            available_vehicle = Some(vehicle);
            break;
        }
        if available_vehicle.is_none() {
            info!(
                "No available vehicles for departure time {}",
                departure_time
            );
            continue;
        }

        //4. TODO: check other constraints (cargo weight, number of passenger seats)
        //info!("[4/5]: Checking other constraints (cargo weight, number of passenger seats)");
        flight_plans.push(create_flight_plan_data(
            available_vehicle.unwrap(),
            &vertiport_depart,
            &vertiport_arrive,
            departure_time,
            arrival_time,
        ));
    }
    if flight_plans.is_empty() {
        return Err("No flight plans found (deadhead flights not implemented)".to_string());
        //TODO: another cycle, now with deadhead flights
    }

    //5. return draft flight plan(s)
    info!(
        "[5/5]: Returning {} draft flight plan(s)",
        flight_plans.len()
    );
    info!("Finished getting flight plans");
    debug!("Flight plans: {:?}", flight_plans);
    Ok(flight_plans)
}

/// Estimates the time needed to travel between two locations including loading and unloading
/// Estimate should be rather generous to block resources instead of potentially overloading them
pub fn estimate_flight_time_minutes(distance_km: f32, aircraft: Aircraft) -> f32 {
    debug!("distance_km: {}", distance_km);
    debug!("aircraft: {:?}", aircraft);
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
    debug!("id: {}", id);
    let nodes = NODES.get().expect("Nodes not initialized");
    let node = nodes
        .iter()
        .find(|node| node.uid == id)
        .ok_or_else(|| "Node not found by id: ".to_owned() + id)?;
    Ok(node)
}

/// Initialize the router with vertiports from the storage service
pub fn init_router_from_vertiports(vertiports: &[Vertiport]) -> Result<(), String> {
    info!("Initializing router from vertiports");
    let nodes = vertiports
        .iter()
        .map(|vertiport| Node {
            uid: vertiport.id.clone(),
            location: Location {
                latitude: OrderedFloat(
                    vertiport
                        .data
                        .as_ref()
                        .ok_or_else(|| format!("Something went wrong when parsing latitude data of vertiport id: {}", vertiport.id))
                        .unwrap()
                        .latitude as f32,
                ),
                longitude: OrderedFloat(
                    vertiport
                        .data
                        .as_ref()
                        .ok_or_else(|| format!("Something went wrong when parsing longitude data of vertiport id: {}", vertiport.id))
                        .unwrap()
                        .longitude as f32,
                ),
                altitude_meters: OrderedFloat(0.0),
            },
            forward_to: None,
            status: status::Status::Ok,
        })
        .collect();
    NODES.set(nodes).map_err(|_| "Failed to set NODES")?;
    init_router()
}

/// Takes customer location (src) and required destination (dst) and returns a tuple with nearest vertiports to src and dst
pub fn get_nearest_vertiports<'a>(
    src_location: &'a Location,
    dst_location: &'a Location,
    vertiports: &'static Vec<Node>,
) -> (&'static Node, &'static Node) {
    info!("Getting nearest vertiports");
    let mut src_vertiport = &vertiports[0];
    let mut dst_vertiport = &vertiports[0];
    debug!("src_location: {:?}", src_location);
    debug!("dst_location: {:?}", dst_location);
    let mut src_distance = haversine::distance(src_location, &src_vertiport.location);
    let mut dst_distance = haversine::distance(dst_location, &dst_vertiport.location);
    debug!("src_distance: {}", src_distance);
    debug!("dst_distance: {}", dst_distance);
    for vertiport in vertiports {
        debug!("checking vertiport: {:?}", vertiport);
        let new_src_distance = haversine::distance(src_location, &vertiport.location);
        let new_dst_distance = haversine::distance(dst_location, &vertiport.location);
        debug!("new_src_distance: {}", new_src_distance);
        debug!("new_dst_distance: {}", new_dst_distance);
        if new_src_distance < src_distance {
            src_distance = new_src_distance;
            src_vertiport = vertiport;
        }
        if new_dst_distance < dst_distance {
            dst_distance = new_dst_distance;
            dst_vertiport = vertiport;
        }
    }
    debug!("src_vertiport: {:?}", src_vertiport);
    debug!("dst_vertiport: {:?}", dst_vertiport);
    (src_vertiport, dst_vertiport)
}

/// Returns a list of nodes near the given location
pub fn get_nearby_nodes(query: NearbyLocationQuery) -> &'static Vec<Node> {
    debug!("query: {:?}", query);
    NODES
        .set(generate_nodes_near(
            &query.location,
            query.radius,
            query.capacity,
        ))
        .expect("Failed to generate nodes");
    return NODES.get().expect("Failed to get nodes");
}

/// Checks if router is initialized
pub fn is_router_initialized() -> bool {
    ARROW_CARGO_ROUTER.get().is_some()
}

/// Get route
pub fn get_route(req: RouteQuery) -> Result<(Vec<Location>, f32), &'static str> {
    info!("Getting route");
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
        .ok_or("Can't access router")
        .unwrap()
        .find_shortest_path(from, to, Algorithm::Dijkstra, None);
    debug!("cost: {}", cost);
    debug!("path: {:?}", path);
    let locations = path
        .iter()
        .map(|node_idx| {
            ARROW_CARGO_ROUTER
                .get()
                .as_ref()
                .ok_or("Can't access router")
                .unwrap()
                .get_node_by_id(*node_idx)
                .ok_or(format!("Node not found by index {:?}", *node_idx))
                .unwrap()
                .location
        })
        .collect::<Vec<Location>>();
    debug!("locations: {:?}", locations);
    info!("Finished getting route with cost: {}", cost);
    Ok((locations, cost))
}

/// Initializes the router for the given aircraft
pub fn init_router() -> Result<(), String> {
    if NODES.get().is_none() {
        return Err("Nodes not initialized. Try to get some nodes first.".to_string());
    }
    if ARROW_CARGO_ROUTER.get().is_some() {
        return Err(
            "Router already initialized. Try to use the router instead of initializing it."
                .to_string(),
        );
    }
    ARROW_CARGO_ROUTER
        .set(Router::new(
            NODES.get().as_ref().unwrap(),
            ARROW_CARGO_CONSTRAINT,
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
            |from, to| haversine::distance(&from.as_node().location, &to.as_node().location),
        ))
        .map_err(|_| "Failed to initialize router".to_string())
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
