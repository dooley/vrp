use crate::construction::heuristics::evaluators::{evaluate_job_insertion, InsertionPosition};
use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::construction::Quota;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::models::solution::Activity;
use crate::utils::map_reduce;
use std::borrow::Borrow;
use std::sync::Arc;

/// Specifies insertion result variant.
pub enum InsertionResult {
    /// Successful insertion result.
    Success(InsertionSuccess),
    /// Insertion failure.
    Failure(InsertionFailure),
}

/// Specifies insertion success result needed to insert job into tour.
pub struct InsertionSuccess {
    /// Specifies delta cost change for the insertion.
    pub cost: Cost,

    /// Original job to be inserted.
    pub job: Job,

    /// Specifies activities within index where they have to be inserted.
    pub activities: Vec<(Activity, usize)>,

    /// Specifies route context where insertion happens.
    pub context: RouteContext,
}

/// Specifies insertion failure.
pub struct InsertionFailure {
    /// Failed constraint code.
    pub constraint: i32,
    /// Original job failed to be inserted.
    pub job: Option<Job>,
}

/// On each insertion step, selects a list of routes where jobs can be inserted.
/// It is up to implementation to decide whether list consists of all possible routes or just some subset.
pub trait RouteSelector {
    /// Returns routes for job insertion.
    fn select<'a>(&'a self, ctx: &'a InsertionContext, job: &'a Job) -> Box<dyn Iterator<Item = RouteContext> + 'a>;
}

/// Returns a list of all possible routes for insertion.
pub struct AllRouteSelector {}

impl Default for AllRouteSelector {
    fn default() -> Self {
        Self {}
    }
}

impl RouteSelector for AllRouteSelector {
    fn select<'a>(&'a self, ctx: &'a InsertionContext, _job: &'a Job) -> Box<dyn Iterator<Item = RouteContext> + 'a> {
        Box::new(ctx.solution.routes.iter().cloned().chain(ctx.solution.registry.next()))
    }
}

/// On each insertion step, selects a list of jobs to be inserted.
/// It is up to implementation to decide whether list consists of all jobs or just some subset.
pub trait JobSelector {
    /// Returns a portion of all jobs.
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a>;
}

/// Returns a list of all jobs to be inserted.
pub struct AllJobSelector {}

impl Default for AllJobSelector {
    fn default() -> Self {
        Self {}
    }
}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        Box::new(ctx.solution.required.iter().cloned())
    }
}

/// A job collection reducer.
pub trait JobMapReducer {
    /// Reduces job collection into single insertion result
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        insertion_position: InsertionPosition,
    ) -> InsertionResult;
}

/// A job map reducer which compares pairs of insertion results and pick one from those.
pub struct PairJobMapReducer {
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl PairJobMapReducer {
    /// Creates a new instance of `PairJobMapReducer`.
    pub fn new(
        route_selector: Box<dyn RouteSelector + Send + Sync>,
        result_selector: Box<dyn ResultSelector + Send + Sync>,
    ) -> Self {
        Self { route_selector, result_selector }
    }
}

impl JobMapReducer for PairJobMapReducer {
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        insertion_position: InsertionPosition,
    ) -> InsertionResult {
        map_reduce(
            &jobs,
            |job| evaluate_job_insertion(&job, &ctx, self.route_selector.as_ref(), insertion_position),
            InsertionResult::make_failure,
            |a, b| self.result_selector.select(&ctx, a, b),
        )
    }
}

/// Insertion result selector.
pub trait ResultSelector {
    /// Selects one insertion result from two to promote as best.
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult;
}

/// Selects best result.
pub struct BestResultSelector {}

impl Default for BestResultSelector {
    fn default() -> Self {
        Self {}
    }
}

impl ResultSelector for BestResultSelector {
    fn select(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

/// Implements generalized insertion heuristic.
/// Using `JobSelector` and `ResultSelector`, it tries to identify next job to be inserted until
/// there are no jobs left or it is not possible to insert due to constraint limitations.
pub struct InsertionHeuristic {
    insertion_position: InsertionPosition,
}

impl Default for InsertionHeuristic {
    fn default() -> Self {
        InsertionHeuristic::new(InsertionPosition::Any)
    }
}

impl InsertionHeuristic {
    /// Creates a new instance of `InsertionHeuristic`.
    pub fn new(insertion_position: InsertionPosition) -> Self {
        Self { insertion_position }
    }
}

impl InsertionHeuristic {
    /// Runs common insertion heuristic algorithm using given selector and reducer specializations.
    pub fn process(
        &self,
        job_selector: &(dyn JobSelector + Send + Sync),
        job_reducer: &(dyn JobMapReducer + Send + Sync),
        ctx: InsertionContext,
        quota: &Option<Arc<dyn Quota + Send + Sync>>,
    ) -> InsertionContext {
        let mut ctx = ctx;

        prepare_ctx(&mut ctx);

        while !ctx.solution.required.is_empty() && !quota.as_ref().map_or(false, |q| q.is_reached()) {
            let jobs = job_selector.select(&mut ctx).collect::<Vec<Job>>();
            let result = job_reducer.reduce(&ctx, jobs, self.insertion_position);
            insert(result, &mut ctx);
        }

        finalize_ctx(&mut ctx);

        ctx
    }
}

impl InsertionResult {
    /// Creates result which represents insertion success.
    pub fn make_success(cost: Cost, job: Job, activities: Vec<(Activity, usize)>, route_ctx: RouteContext) -> Self {
        Self::Success(InsertionSuccess { cost, job, activities, context: route_ctx })
    }

    /// Creates result which represents insertion failure.
    pub fn make_failure() -> Self {
        Self::make_failure_with_code(-1, None)
    }

    /// Creates result which represents insertion failure with given code.
    pub fn make_failure_with_code(code: i32, job: Option<Job>) -> Self {
        Self::Failure(InsertionFailure { constraint: code, job })
    }

    /// Compares two insertion results and returns the cheapest by cost.
    pub fn choose_best_result(left: Self, right: Self) -> Self {
        match (left.borrow(), right.borrow()) {
            (Self::Success(_), Self::Failure(_)) => left,
            (Self::Failure(_), Self::Success(_)) => right,
            (Self::Success(lhs), Self::Success(rhs)) => {
                if lhs.cost > rhs.cost {
                    right
                } else {
                    left
                }
            }
            _ => right,
        }
    }
}

fn prepare_ctx(ctx: &mut InsertionContext) {
    ctx.solution.required.extend(ctx.solution.unassigned.drain().map(|(job, _)| job));
    ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
}

fn finalize_ctx(ctx: &mut InsertionContext) {
    ctx.solution.unassigned.extend(ctx.solution.required.drain(0..).map(|job| (job, 0)));
    ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
}

fn insert(result: InsertionResult, ctx: &mut InsertionContext) {
    match result {
        InsertionResult::Success(success) => {
            let is_new_route = ctx.solution.registry.use_route(&success.context);
            let route_index = ctx.solution.routes.iter().position(|ctx| ctx == &success.context).unwrap_or_else(|| {
                assert!(is_new_route);
                ctx.solution.routes.push(success.context.deep_copy());
                ctx.solution.routes.len() - 1
            });

            let route_ctx = ctx.solution.routes.get_mut(route_index).unwrap();
            let route = route_ctx.route_mut();
            success.activities.into_iter().for_each(|(a, index)| {
                route.tour.insert_at(a, index + 1);
            });

            let job = success.job;
            ctx.solution.required.retain(|j| *j != job);
            ctx.problem.constraint.accept_insertion(&mut ctx.solution, route_index, &job);
        }
        InsertionResult::Failure(failure) => {
            if let Some(job) = failure.job {
                ctx.solution.unassigned.insert(job.clone(), failure.constraint);
                ctx.solution.required.retain(|j| *j != job);
            }
        }
    }
}
