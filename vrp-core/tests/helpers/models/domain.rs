use crate::construction::constraints::ConstraintPipeline;
use crate::construction::states::{InsertionContext, SolutionContext};
use crate::helpers::models::problem::*;
use crate::models::common::IdDimension;
use crate::models::problem::{Fleet, Job, Jobs};
use crate::models::solution::Registry;
use crate::models::{Problem, Solution};
use crate::refinement::objectives::PenalizeUnassigned;
use crate::utils::DefaultRandom;
use std::borrow::Borrow;
use std::sync::Arc;

pub fn create_empty_problem_with_constraint(constraint: ConstraintPipeline) -> Arc<Problem> {
    let transport = Arc::new(TestTransportCost::new());
    let fleet = Arc::new(test_fleet());
    let jobs = Arc::new(Jobs::new(fleet.borrow(), vec![], transport.as_ref()));
    Arc::new(Problem {
        fleet,
        jobs,
        locks: vec![],
        constraint: Arc::new(constraint),
        activity: Arc::new(TestActivityCost::new()),
        transport,
        objective: Arc::new(PenalizeUnassigned::default()),
        extras: Arc::new(Default::default()),
    })
}

pub fn create_empty_problem() -> Arc<Problem> {
    create_empty_problem_with_constraint(ConstraintPipeline::default())
}

pub fn create_empty_solution() -> Solution {
    Solution {
        registry: Registry::new(&Fleet::new(vec![test_driver()], vec![test_vehicle(0)])),
        routes: vec![],
        unassigned: Default::default(),
        extras: Arc::new(Default::default()),
    }
}

pub fn create_empty_insertion_context() -> InsertionContext {
    InsertionContext {
        problem: create_empty_problem(),
        solution: SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            locked: Default::default(),
            routes: vec![],
            registry: Registry::new(&Fleet::new(vec![test_driver()], vec![test_vehicle(0)])),
        },
        random: Arc::new(DefaultRandom::default()),
    }
}

pub fn get_customer_ids_from_routes_sorted(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    let mut result = get_customer_ids_from_routes(insertion_ctx);
    result.sort();
    result
}

pub fn get_sorted_customer_ids_from_jobs(jobs: &Vec<Job>) -> Vec<String> {
    let mut ids = jobs.iter().map(|job| get_customer_id(&job)).collect::<Vec<String>>();
    ids.sort();
    ids
}

pub fn get_customer_ids_from_routes(insertion_ctx: &InsertionContext) -> Vec<Vec<String>> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(|rc| {
            rc.route
                .tour
                .all_activities()
                .filter(|a| a.job.is_some())
                .map(|a| a.retrieve_job().unwrap())
                .map(|job| get_customer_id(&job))
                .collect::<Vec<String>>()
        })
        .collect()
}

pub fn get_customer_id(job: &Job) -> String {
    job.dimens().get_id().unwrap().clone()
}