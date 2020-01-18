use crate::extensions::MultiDimensionalCapacity;
use std::sync::Arc;
use vrp_core::construction::constraints::Demand;
use vrp_core::construction::states::{create_end_activity, create_start_activity};
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::solution::*;

const DEFAULT_VEHICLE_COSTS: Costs =
    Costs { fixed: 100.0, per_distance: 1.0, per_driving_time: 1.0, per_waiting_time: 1.0, per_service_time: 1.0 };
pub const DEFAULT_JOB_LOCATION: Location = 0;
pub const DEFAULT_JOB_DURATION: Duration = 0.0;
pub const DEFAULT_JOB_TIME_WINDOW: TimeWindow = TimeWindow { start: 0.0, end: 1000.0 };
pub const DEFAULT_ACTIVITY_SCHEDULE: Schedule = Schedule { departure: 0.0, arrival: 0.0 };

pub fn test_driver() -> Driver {
    Driver { costs: DEFAULT_VEHICLE_COSTS, dimens: Default::default(), details: vec![] }
}

pub fn test_vehicle(id: &str) -> Vehicle {
    let mut dimens = Dimensions::new();
    dimens.set_id(id);

    Vehicle {
        profile: 0,
        costs: DEFAULT_VEHICLE_COSTS,
        dimens,
        details: vec![VehicleDetail { start: Some(0), end: Some(0), time: None }],
    }
}

pub fn create_route_with_activities(fleet: &Fleet, vehicle: &str, activities: Vec<TourActivity>) -> Route {
    let actor = fleet.actors.iter().filter(|a| a.vehicle.dimens.get_id().unwrap() == vehicle).next().unwrap().clone();
    let mut tour = Tour::default();
    tour.set_start(create_start_activity(&actor));
    create_end_activity(&actor).map(|end| tour.set_end(end));

    activities.into_iter().enumerate().for_each(|(index, a)| {
        tour.insert_at(a, index + 1);
    });

    Route { actor, tour }
}

pub fn create_activity_with_job_at_location(job: Arc<Single>, location: Location) -> TourActivity {
    Box::new(Activity {
        place: vrp_core::models::solution::Place {
            location,
            duration: DEFAULT_JOB_DURATION,
            time: DEFAULT_JOB_TIME_WINDOW,
        },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: Some(job),
    })
}

pub fn create_single_with_location(location: Option<Location>) -> Single {
    Single {
        places: vec![vrp_core::models::problem::Place {
            location,
            duration: DEFAULT_JOB_DURATION,
            times: vec![DEFAULT_JOB_TIME_WINDOW],
        }],
        dimens: Default::default(),
    }
}

pub fn single_demand_as_multi(pickup: (i32, i32), delivery: (i32, i32)) -> Demand<MultiDimensionalCapacity> {
    let make = |value| {
        if value == 0 {
            MultiDimensionalCapacity::default()
        } else {
            MultiDimensionalCapacity::new(vec![value])
        }
    };

    Demand { pickup: (make(pickup.0), make(pickup.1)), delivery: (make(delivery.0), make(delivery.1)) }
}