use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::utils::DefaultRandom;

fn create_basic_problem(breaks: Option<Vec<VehicleBreak>>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_multi_job(
                    "job2",
                    vec![((2., 0.), 1., vec![1]), ((3., 0.), 1., vec![1])],
                    vec![((4., 0.), 1., vec![2])],
                ),
                create_pickup_job("job3", vec![5., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift { breaks, ..create_default_vehicle_shift() }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    }
}

fn create_default_breaks() -> Option<Vec<VehicleBreak>> {
    Some(vec![VehicleBreak {
        time: VehicleBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
        duration: 2.0,
        locations: None,
    }])
}

fn create_unassigned_jobs(job_ids: &[&str]) -> Option<Vec<UnassignedJob>> {
    Some(
        job_ids
            .iter()
            .map(|job_id| UnassignedJob {
                job_id: job_id.to_string(),
                reasons: vec![UnassignedJobReason { code: 0, description: "unknown".to_string() }],
            })
            .collect(),
    )
}

fn get_init_solution(problem: Problem, solution: &Solution) -> Result<Solution, String> {
    let matrix = create_matrix_from_problem(&problem);
    let core_problem = Arc::new(
        (problem, vec![matrix]).read_pragmatic().unwrap_or_else(|err| panic!("cannot read core problem: {:?}", err)),
    );

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    serialize_solution(writer, solution).expect("cannot serialize test solution");

    let core_solution = read_init_solution(BufReader::new(buffer.as_bytes()), core_problem.clone())?;

    // NOTE: get statistic/tours updated
    let insertion_ctx = InsertionContext::new_from_solution(
        core_problem.clone(),
        (core_solution, None),
        Arc::new(DefaultRandom::default()),
    );
    let core_solution = insertion_ctx.solution.to_solution(core_problem.extras.clone());

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    core_solution.write_pragmatic_json(&core_problem, writer).expect("cannot serialize result solution");

    deserialize_solution(BufReader::new(buffer.as_bytes())).map_err(|err| format!("cannot read solution: {}", err))
}

#[test]
fn can_read_basic_init_solution() {
    let problem = create_basic_problem(create_default_breaks());
    let solution = Solution {
        statistic: Statistic {
            cost: 32.,
            distance: 8,
            duration: 14,
            times: Timing { driving: 8, serving: 4, waiting: 0, break_time: 2 },
        },
        tours: vec![Tour {
            vehicle_id: "my_vehicle_1".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops: vec![
                create_stop_with_activity(
                    "departure",
                    "departure",
                    (0., 0.),
                    1,
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0,
                ),
                create_stop_with_activity(
                    "job1",
                    "delivery",
                    (1., 0.),
                    0,
                    ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    1,
                ),
                create_stop_with_activity_with_tag(
                    "job2",
                    "pickup",
                    (2., 0.),
                    1,
                    ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                    2,
                    "p1",
                ),
                Stop {
                    location: vec![3., 0.].to_loc(),
                    time: Schedule {
                        arrival: "1970-01-01T00:00:05Z".to_string(),
                        departure: "1970-01-01T00:00:08Z".to_string(),
                    },
                    distance: 3,
                    load: vec![2],
                    activities: vec![
                        Activity {
                            job_id: "job2".to_string(),
                            activity_type: "pickup".to_string(),
                            location: Some(vec![3., 0.].to_loc()),
                            time: Some(Interval {
                                start: "1970-01-01T00:00:05Z".to_string(),
                                end: "1970-01-01T00:00:06Z".to_string(),
                            }),
                            job_tag: Some("p2".to_owned()),
                        },
                        Activity {
                            job_id: "break".to_string(),
                            activity_type: "break".to_string(),
                            location: Some(vec![3., 0.].to_loc()),
                            time: Some(Interval {
                                start: "1970-01-01T00:00:06Z".to_string(),
                                end: "1970-01-01T00:00:08Z".to_string(),
                            }),
                            job_tag: None,
                        },
                    ],
                },
                create_stop_with_activity_with_tag(
                    "job2",
                    "delivery",
                    (4., 0.),
                    0,
                    ("1970-01-01T00:00:09Z", "1970-01-01T00:00:10Z"),
                    4,
                    "d1",
                ),
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (0., 0.),
                    0,
                    ("1970-01-01T00:00:14Z", "1970-01-01T00:00:14Z"),
                    8,
                ),
            ],
            statistic: Statistic {
                cost: 32.,
                distance: 8,
                duration: 14,
                times: Timing { driving: 8, serving: 4, waiting: 0, break_time: 2 },
            },
        }],
        unassigned: create_unassigned_jobs(&["job3"]),
        ..create_empty_solution()
    };

    let result_solution =
        get_init_solution(problem, &solution).unwrap_or_else(|err| panic!("cannot get solution: {}", err));

    assert_eq!(result_solution, solution);
}

#[test]
fn can_handle_error_in_init_solution() {
    let problem = create_basic_problem(create_default_breaks());
    let solution = Solution {
        statistic: Default::default(),
        tours: vec![Tour {
            vehicle_id: "my_vehicle_1".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops: vec![],
            statistic: Default::default(),
        }],
        unassigned: None,
        violations: None,
        extras: None,
    };

    let result_solution = get_init_solution(problem, &solution);

    assert_eq!(result_solution, Err("empty tour in init solution".to_owned()));
}
