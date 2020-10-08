//! Import from an another json format logic.

#[cfg(feature = "hre-format")]
#[cfg(test)]
#[path = "../../../tests/unit/extensions/import/hre_test.rs"]
mod hre_test;

extern crate serde_json;

use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Write};
use vrp_pragmatic::format::problem::Problem;

#[cfg(feature = "hre-format")]
mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Location {
        /// Latitude.
        pub lat: f64,
        /// Longitude.
        pub lng: f64,
    }

    // region Plan

    /// Relation type.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum RelationType {
        /// Tour relation locks jobs to specific vehicle in any order.
        Tour,
        /// Flexible relation locks jobs in specific order allowing insertion of other jobs in between.
        Flexible,
        /// Sequence relation locks jobs in strict order, no insertions in between are allowed.
        Sequence,
    }

    /// Relation is the way to lock specific jobs to specific vehicles.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Relation {
        /// Relation type.
        #[serde(rename(deserialize = "type", serialize = "type"))]
        pub type_field: RelationType,
        /// List of job ids.
        pub jobs: Vec<String>,
        /// Vehicle id.
        pub vehicle_id: String,
        /// Vehicle shift index.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shift_index: Option<usize>,
    }

    /// Defines specific job place.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct JobPlace {
        /// A list of job time windows with time specified in RFC3339 format.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub times: Option<Vec<Vec<String>>>,
        /// Job location.
        pub location: Location,
        /// Job duration (service time).
        pub duration: f64,
        /// An tag which will be propagated back within corresponding activity in solution.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
    }

    /// Specifies pickup and delivery places of the job.
    /// At least one place should be specified. If only delivery specified, then vehicle is loaded with
    /// job's demand at the start location. If only pickup specified, then loaded good is delivered to
    /// the last location on the route. When both, pickup and delivery, are specified, then it is classical
    /// pickup and delivery job.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct JobPlaces {
        /// Pickup place.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub pickup: Option<JobPlace>,
        /// Delivery place.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub delivery: Option<JobPlace>,
    }

    /// Specifies single job.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct Job {
        /// Job id.
        pub id: String,
        /// Job places.
        pub places: JobPlaces,
        /// Job demand.
        pub demand: Vec<i32>,
        /// Job priority, bigger value - less important.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub priority: Option<i32>,
        /// Job skills.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub skills: Option<Vec<String>>,
    }

    /// Specifies a place for sub job.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct MultiJobPlace {
        /// A list of sub job time windows with time specified in RFC3339 format.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub times: Option<Vec<Vec<String>>>,
        /// Sub job location.
        pub location: Location,
        /// Sub job duration (service time).
        pub duration: f64,
        /// Sub job demand.
        pub demand: Vec<i32>,
        /// An tag which will be propagated back within corresponding activity in solution.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
    }

    /// Specifies pickups and deliveries places of multi job.
    /// All of them should be completed or none of them. All pickups must be completed before any of deliveries.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct MultiJobPlaces {
        /// A list of pickups.
        pub pickups: Vec<MultiJobPlace>,
        /// A list of deliveries.
        pub deliveries: Vec<MultiJobPlace>,
    }

    /// Specifies multi job which has multiple child jobs.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct MultiJob {
        /// Multi job id.
        pub id: String,
        /// Multi job places.
        pub places: MultiJobPlaces,
        /// Job priority, bigger value - less important.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub priority: Option<i32>,
        /// Multi job skills.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub skills: Option<Vec<String>>,
    }

    /// Job variant type.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    #[serde(untagged)]
    pub enum JobVariant {
        /// Single job.
        Single(Job),
        /// Multi job.
        Multi(MultiJob),
    }

    /// A plan specifies work which has to be done.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct Plan {
        /// List of jobs.
        pub jobs: Vec<JobVariant>,
        /// List of relations between jobs and vehicles.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub relations: Option<Vec<Relation>>,
    }

    // endregion

    // region Fleet

    /// Specifies vehicle costs.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct VehicleCosts {
        /// Fixed is cost of vehicle usage per tour.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub fixed: Option<f64>,
        /// Cost per distance unit.
        pub distance: f64,
        /// Cost per time unit.
        pub time: f64,
    }

    /// Specifies vehicle place.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct VehiclePlace {
        /// Vehicle start or end time.
        pub time: String,
        /// Vehicle location.
        pub location: Location,
    }

    /// Specifies vehicle shift.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VehicleShift {
        /// Vehicle start place.
        pub start: VehiclePlace,

        /// Vehicle end place.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub end: Option<VehiclePlace>,

        /// Vehicle depots.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub depots: Option<Vec<VehicleDepot>>,

        /// Vehicle breaks.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub breaks: Option<Vec<VehicleBreak>>,

        /// Vehicle reloads which allows vehicle to return back to the depot (or any other place) in
        /// order to unload/load goods during single tour.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub reloads: Option<Vec<VehicleReload>>,
    }

    /// Vehicle depot.
    pub type VehicleDepot = JobPlace;

    /// Vehicle reload.
    pub type VehicleReload = JobPlace;

    /// Vehicle limits.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VehicleLimits {
        /// Max traveling distance per shift/tour.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_distance: Option<f64>,

        /// Max time per shift/tour.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shift_time: Option<f64>,
    }

    /// Vehicle break.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct VehicleBreak {
        /// Break time.
        pub times: Vec<Vec<String>>,

        /// Break duration.
        pub duration: f64,

        /// Break location.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub location: Option<Location>,
    }

    /// Specifies a vehicle type.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct VehicleType {
        /// Vehicle type id.
        pub id: String,
        /// Vehicle profile name.
        pub profile: String,
        /// Vehicle costs.
        pub costs: VehicleCosts,
        /// Vehicle shifts.
        pub shifts: Vec<VehicleShift>,
        /// Vehicle capacity.
        pub capacity: Vec<i32>,
        /// Vehicle amount.
        pub amount: i32,

        /// Vehicle skills.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub skills: Option<Vec<String>>,

        /// Vehicle limits.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub limits: Option<VehicleLimits>,
    }

    /// Specifies routing profile.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct Profile {
        /// Profile name.
        pub name: String,
        /// Profile type.
        #[serde(rename(deserialize = "type", serialize = "type"))]
        pub profile_type: String,
    }

    /// Specifies fleet.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct Fleet {
        /// Vehicle types.
        pub types: Vec<VehicleType>,
        /// Routing profiles.
        pub profiles: Vec<Profile>,
    }

    // endregion

    // region Configuration

    /// Specifies extra configuration.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct Config {}

    // endregion

    // region Common

    /// A VRP problem definition.
    #[derive(Clone, Deserialize, Debug, Serialize)]
    pub struct Problem {
        /// Problem plan: customers to serve.
        pub plan: Plan,
        /// Problem resources: vehicles to be used, routing info.
        pub fleet: Fleet,

        /// Extra configuration.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub config: Option<Config>,
    }

    // endregion
}

#[cfg(feature = "hre-format")]
mod deserialize {
    use super::models;
    use std::io::{BufReader, Error, ErrorKind, Read};
    use vrp_pragmatic::format::problem::*;
    use vrp_pragmatic::format::Location;

    fn to_pragmatic_loc(loc: &models::Location) -> Location {
        Location::Coordinate { lat: loc.lat, lng: loc.lng }
    }

    fn create_pragmatic_plan(plan: &models::Plan) -> Result<Plan, String> {
        let job_place_mapper = |job: &models::Job, place: &models::JobPlace| JobTask {
            places: vec![JobPlace {
                location: to_pragmatic_loc(&place.location),
                duration: place.duration,
                times: place.times.clone(),
            }],
            demand: Some(job.demand.clone()),
            tag: place.tag.clone(),
        };

        let multi_job_place_mapper = |places: &Vec<models::MultiJobPlace>| {
            if places.is_empty() {
                None
            } else {
                Some(
                    places
                        .iter()
                        .map(|place| JobTask {
                            places: vec![JobPlace {
                                location: to_pragmatic_loc(&place.location),
                                duration: place.duration,
                                times: place.times.clone(),
                            }],
                            demand: Some(place.demand.clone()),
                            tag: place.tag.clone(),
                        })
                        .collect(),
                )
            }
        };

        Ok(Plan {
            jobs: plan
                .jobs
                .iter()
                .map(|job| match job {
                    models::JobVariant::Single(job) => Job {
                        id: job.id.clone(),
                        pickups: job.places.pickup.as_ref().map(|place| vec![job_place_mapper(job, place)]),
                        deliveries: job.places.delivery.as_ref().map(|place| vec![job_place_mapper(job, place)]),
                        replacements: None,
                        services: None,
                        priority: job.priority.as_ref().copied(),
                        skills: job.skills.clone(),
                    },
                    models::JobVariant::Multi(job) => Job {
                        id: job.id.clone(),
                        pickups: multi_job_place_mapper(&job.places.pickups),
                        deliveries: multi_job_place_mapper(&job.places.deliveries),
                        replacements: None,
                        services: None,
                        priority: job.priority.as_ref().copied(),
                        skills: job.skills.clone(),
                    },
                })
                .collect(),
            relations: plan.relations.as_ref().map(|relations| {
                relations
                    .iter()
                    .map(|r| Relation {
                        type_field: match r.type_field {
                            models::RelationType::Sequence => RelationType::Strict,
                            models::RelationType::Flexible => RelationType::Sequence,
                            models::RelationType::Tour => RelationType::Any,
                        },
                        jobs: r.jobs.clone(),
                        vehicle_id: r.vehicle_id.clone(),
                        shift_index: r.shift_index,
                    })
                    .collect()
            }),
        })
    }

    fn create_pragmatic_fleet(fleet: &models::Fleet) -> Result<Fleet, String> {
        Ok(Fleet {
            vehicles: fleet
                .types
                .iter()
                .map(|v| VehicleType {
                    type_id: v.id.clone(),
                    vehicle_ids: (1..=v.amount).map(|seq| format!("{}_{}", v.id, seq)).collect(),
                    profile: v.profile.clone(),
                    costs: VehicleCosts { fixed: v.costs.fixed, distance: v.costs.distance, time: v.costs.time },
                    shifts: v
                        .shifts
                        .iter()
                        .map(|shift| VehicleShift {
                            start: ShiftStart {
                                earliest: shift.start.time.clone(),
                                latest: None,
                                location: to_pragmatic_loc(&shift.start.location),
                            },
                            end: shift.end.as_ref().map(|end| ShiftEnd {
                                earliest: None,
                                latest: end.time.clone(),
                                location: to_pragmatic_loc(&end.location),
                            }),
                            depots: shift.depots.as_ref().map(|depots| {
                                depots
                                    .iter()
                                    .map(|d| VehicleCargoPlace {
                                        location: to_pragmatic_loc(&d.location),
                                        duration: d.duration,
                                        times: d.times.clone(),
                                        tag: d.tag.clone(),
                                    })
                                    .collect()
                            }),
                            breaks: shift.breaks.as_ref().map(|breaks| {
                                breaks
                                    .iter()
                                    .map(|b| VehicleBreak {
                                        time: VehicleBreakTime::TimeWindow(b.times.first().unwrap().clone()),
                                        duration: b.duration,
                                        locations: b.location.as_ref().map(|l| vec![to_pragmatic_loc(l)]),
                                    })
                                    .collect()
                            }),
                            reloads: shift.reloads.as_ref().map(|reloads| {
                                reloads
                                    .iter()
                                    .map(|r| VehicleCargoPlace {
                                        location: to_pragmatic_loc(&r.location),
                                        duration: r.duration,
                                        times: r.times.clone(),
                                        tag: r.tag.clone(),
                                    })
                                    .collect()
                            }),
                        })
                        .collect(),
                    capacity: v.capacity.clone(),
                    skills: v.skills.clone(),
                    limits: v.limits.as_ref().map(|l| VehicleLimits {
                        max_distance: l.max_distance,
                        shift_time: l.shift_time,
                        allowed_areas: None,
                    }),
                })
                .collect(),
            profiles: fleet
                .profiles
                .iter()
                .map(|p| Profile { name: p.name.clone(), profile_type: p.profile_type.clone(), speed: None })
                .collect(),
        })
    }

    pub fn convert_to_pragmatic<R: Read>(reader: BufReader<R>) -> Result<Problem, Error> {
        let hre_problem: models::Problem =
            serde_json::from_reader(reader).map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;

        let plan = create_pragmatic_plan(&hre_problem.plan).map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
        let fleet =
            create_pragmatic_fleet(&hre_problem.fleet).map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;

        Ok(Problem { plan, fleet, objectives: None, config: None })
    }
}

#[cfg(not(feature = "hre-format"))]
mod deserialize {
    use super::*;
    pub fn convert_to_pragmatic<R: Read>(_reader: BufReader<R>) -> Result<Problem, Error> {
        unreachable!()
    }
}

#[cfg(feature = "hre-format")]
mod serialize {
    use super::models::*;
    use std::io::{BufWriter, Error, ErrorKind, Write};
    use vrp_pragmatic::format::problem::VehicleBreakTime;

    fn to_hre_loc(loc: &vrp_pragmatic::format::Location) -> Result<Location, String> {
        match loc.clone() {
            vrp_pragmatic::format::Location::Coordinate { lat, lng } => Ok(Location { lat, lng }),
            _ => Err("hre format supports only geocoordinates".to_string()),
        }
    }

    fn create_hre_plan(plan: &vrp_pragmatic::format::problem::Plan) -> Result<Plan, String> {
        let job_tasks_to_job_place =
            |job_tasks: &Option<Vec<vrp_pragmatic::format::problem::JobTask>>| -> Result<Option<JobPlace>, String> {
                if let Some(job_tasks) = &job_tasks {
                    let job_task = job_tasks.first().ok_or("empty job tasks")?;
                    let job_place = job_task.places.first().ok_or("empty job places")?;
                    Ok(Some(JobPlace {
                        times: job_place.times.clone(),
                        location: to_hre_loc(&job_place.location)?,
                        duration: job_place.duration,
                        tag: job_task.tag.clone(),
                    }))
                } else {
                    Ok(None)
                }
            };

        let job_tasks_to_multi_job_place =
            |job_tasks: &Option<Vec<vrp_pragmatic::format::problem::JobTask>>| -> Result<Vec<MultiJobPlace>, String> {
                if let Some(job_tasks) = &job_tasks {
                    Ok(job_tasks
                        .iter()
                        .map(|job_task| {
                            let job_place = job_task.places.first().ok_or("empty job places")?;

                            Ok(MultiJobPlace {
                                times: job_place.times.clone(),
                                location: to_hre_loc(&job_place.location)?,
                                duration: job_place.duration,
                                demand: job_task.demand.clone().ok_or("no demand")?,
                                tag: job_task.tag.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?)
                } else {
                    Err("empty job tasks".to_string())
                }
            };

        Ok(Plan {
            jobs: plan
                .jobs
                .iter()
                .map(|job| {
                    if job.services.as_ref().map_or(false, |t| !t.is_empty())
                        || job.replacements.as_ref().map_or(false, |t| !t.is_empty())
                    {
                        return Err("service or replacement jobs are not supported by hre format".to_string());
                    }

                    let pickups = job.pickups.as_ref().map_or(0, |t| t.len());
                    let deliveries = job.deliveries.as_ref().map_or(0, |t| t.len());

                    if pickups == 0 && deliveries == 0 {
                        return Err(format!("No pickups and deliveries in the job '{}'", job.id));
                    }

                    Ok(if pickups > 1 || deliveries > 1 {
                        JobVariant::Multi(MultiJob {
                            id: job.id.clone(),
                            places: MultiJobPlaces {
                                pickups: job_tasks_to_multi_job_place(&job.pickups)?,
                                deliveries: job_tasks_to_multi_job_place(&job.deliveries)?,
                            },
                            priority: job.priority,
                            skills: job.skills.clone(),
                        })
                    } else {
                        JobVariant::Single(Job {
                            id: job.id.clone(),
                            places: JobPlaces {
                                pickup: job_tasks_to_job_place(&job.pickups)?,
                                delivery: job_tasks_to_job_place(&job.deliveries)?,
                            },
                            demand: job
                                .pickups
                                .as_ref()
                                .or_else(|| job.deliveries.as_ref())
                                .ok_or("no pickups and deliveries")?
                                .first()
                                .ok_or("no job task")?
                                .demand
                                .clone()
                                .ok_or("no demand")?,
                            priority: job.priority,
                            skills: job.skills.clone(),
                        })
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            relations: plan.relations.as_ref().map(|relations| {
                relations
                    .iter()
                    .map(|relation| Relation {
                        type_field: match relation.type_field {
                            vrp_pragmatic::format::problem::RelationType::Strict => RelationType::Sequence,
                            vrp_pragmatic::format::problem::RelationType::Sequence => RelationType::Flexible,
                            vrp_pragmatic::format::problem::RelationType::Any => RelationType::Tour,
                        },
                        jobs: relation.jobs.clone(),
                        vehicle_id: relation.vehicle_id.clone(),
                        shift_index: None,
                    })
                    .collect()
            }),
        })
    }

    fn create_hre_fleet(fleet: &vrp_pragmatic::format::problem::Fleet) -> Result<Fleet, String> {
        Ok(Fleet {
            types: fleet
                .vehicles
                .iter()
                .map(|vehicle| {
                    Ok(VehicleType {
                        id: vehicle.type_id.clone(),
                        profile: vehicle.profile.clone(),
                        costs: VehicleCosts {
                            fixed: vehicle.costs.fixed,
                            distance: vehicle.costs.distance,
                            time: vehicle.costs.time,
                        },
                        shifts: vehicle
                            .shifts
                            .iter()
                            .map(|shift| {
                                Ok(VehicleShift {
                                    start: VehiclePlace {
                                        time: shift.start.earliest.clone(),
                                        location: to_hre_loc(&shift.start.location)?,
                                    },
                                    end: if let Some(end) = &shift.end {
                                        Some(VehiclePlace {
                                            time: end.latest.clone(),
                                            location: to_hre_loc(&shift.start.location)?,
                                        })
                                    } else {
                                        None
                                    },
                                    depots: if let Some(depots) = &shift.depots {
                                        Some(
                                            depots
                                                .iter()
                                                .map(|depot| {
                                                    Ok(VehicleDepot {
                                                        times: depot.times.clone(),
                                                        location: to_hre_loc(&depot.location)?,
                                                        duration: depot.duration,
                                                        tag: depot.tag.clone(),
                                                    })
                                                })
                                                .collect::<Result<Vec<_>, String>>()?,
                                        )
                                    } else {
                                        None
                                    },
                                    breaks: if let Some(breaks) = &shift.breaks {
                                        Some(
                                            breaks
                                                .iter()
                                                .map(|br| {
                                                    Ok(VehicleBreak {
                                                        times: match &br.time {
                                                            VehicleBreakTime::TimeWindow(times) => {
                                                                Ok(vec![times.clone()])
                                                            }
                                                            _ => Err("hre format does not support offset break"),
                                                        }?,
                                                        duration: br.duration,
                                                        location: if let Some(locations) = br.locations.as_ref() {
                                                            locations
                                                                .iter()
                                                                .map(|loc| to_hre_loc(loc))
                                                                .collect::<Result<Vec<_>, String>>()?
                                                                .first()
                                                                .cloned()
                                                        } else {
                                                            None
                                                        },
                                                    })
                                                })
                                                .collect::<Result<Vec<_>, String>>()?,
                                        )
                                    } else {
                                        None
                                    },
                                    reloads: if let Some(reloads) = &shift.reloads {
                                        Some(
                                            reloads
                                                .iter()
                                                .map(|reload| {
                                                    Ok(VehicleReload {
                                                        times: reload.times.clone(),
                                                        location: to_hre_loc(&reload.location)?,
                                                        duration: reload.duration,
                                                        tag: reload.tag.clone(),
                                                    })
                                                })
                                                .collect::<Result<Vec<_>, String>>()?,
                                        )
                                    } else {
                                        None
                                    },
                                })
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                        capacity: vehicle.capacity.clone(),
                        amount: vehicle.vehicle_ids.len() as i32,
                        skills: vehicle.skills.clone(),
                        limits: vehicle.limits.as_ref().map(|limits| VehicleLimits {
                            max_distance: limits.max_distance,
                            shift_time: limits.shift_time,
                        }),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            profiles: fleet
                .profiles
                .iter()
                .map(|p| Profile { name: p.name.clone(), profile_type: p.profile_type.clone() })
                .collect(),
        })
    }

    pub fn write_as_hre<W: Write>(
        writer: BufWriter<W>,
        problem: &vrp_pragmatic::format::problem::Problem,
    ) -> Result<(), Error> {
        let plan = create_hre_plan(&problem.plan).map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
        let fleet = create_hre_fleet(&problem.fleet).map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
        let hre_problem = Problem { plan, fleet, config: None };

        serde_json::to_writer_pretty(writer, &hre_problem).map_err(Error::from)
    }
}

#[cfg(not(feature = "hre-format"))]
mod serialize {
    use super::*;
    pub fn write_as_hre<W: Write>(_writer: BufWriter<W>, _problem: &Problem) -> Result<(), Error> {
        unreachable!()
    }
}

/// Converts pragmatic problem to hre and writes it.
pub fn serialize_hre_problem<W: Write>(writer: BufWriter<W>, pragmatic_problem: &Problem) -> Result<(), Error> {
    if cfg!(feature = "hre-format") {
        serialize::write_as_hre(writer, pragmatic_problem)
    } else {
        Err(Error::new(ErrorKind::Other, "hre format is not enabled"))
    }
}

/// Reads hre problem and converts it to pragmatic format.
pub fn deserialize_hre_problem<R: Read>(reader: BufReader<R>) -> Result<Problem, Error> {
    if cfg!(feature = "hre-format") {
        deserialize::convert_to_pragmatic(reader)
    } else {
        Err(Error::new(ErrorKind::Other, "hre format is not enabled"))
    }
}
