use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::solver::mutation::Recreate;
use crate::solver::RefinementContext;
use crate::utils::{compare_floats, parallel_collect, CollectGroupBy};
use hashbrown::HashSet;

/// A recreate strategy which computes the difference in cost of inserting customer in its
/// best and kth best route, where `k` is a user-defined parameter. Then it inserts the
/// customer with the max difference in its least cost position.
pub struct RecreateWithRegret {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithRegret {
    fn default() -> Self {
        RecreateWithRegret::new(1, 2)
    }
}

impl Recreate for RecreateWithRegret {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}

impl RecreateWithRegret {
    /// Creates a new instance of `RecreateWithRegret`.
    pub fn new(min: usize, max: usize) -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(RegretJobMapReducer::new(min, max)),
        }
    }
}

struct RegretJobMapReducer {
    min: usize,
    max: usize,

    route_selector: Box<dyn RouteSelector + Send + Sync>,
    inner_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl RegretJobMapReducer {
    /// Creates a new instance of `RegretJobMapReducer`.
    pub fn new(min: usize, max: usize) -> Self {
        assert!(min > 0);
        assert!(min <= max);

        Self {
            min,
            max,
            route_selector: Box::new(AllRouteSelector::default()),
            inner_reducer: Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
            )),
        }
    }
}

impl JobMapReducer for RegretJobMapReducer {
    #[allow(clippy::let_and_return)]
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        insertion_position: InsertionPosition,
    ) -> InsertionResult {
        let regret_index = ctx.random.uniform_int(self.min as i32, self.max as i32) as usize;

        // NOTE no need to proceed with regret, fallback to more performant reducer
        if regret_index == 1 || jobs.len() == 1 || ctx.solution.routes.len() < 2 {
            return self.inner_reducer.reduce(ctx, jobs, insertion_position);
        }

        let mut results = parallel_collect(&jobs, |job| {
            self.route_selector
                .select(ctx, job)
                .map(|route_ctx| evaluate_job_insertion_in_route(job, ctx, &route_ctx, insertion_position, None))
                .collect::<Vec<_>>()
        })
        .into_iter()
        .flat_map(|results| results.into_iter())
        .filter_map(|result| match result {
            InsertionResult::Success(success) => Some(success),
            _ => None,
        })
        .collect_group_by_key::<Job, InsertionSuccess, _>(|success| success.job.clone())
        .into_iter()
        .filter_map(|(_, mut success)| {
            if success.len() < regret_index {
                return None;
            }

            success.sort_by(|a, b| compare_floats(a.cost, b.cost));

            let (_, mut job_results) = success.into_iter().fold(
                (HashSet::with_capacity(ctx.solution.routes.len()), Vec::default()),
                |(mut routes, mut results), result| {
                    if !routes.contains(&result.context.route.actor) {
                        results.push(result);
                    } else {
                        routes.insert(result.context.route.actor.clone());
                    }

                    (routes, results)
                },
            );

            if regret_index < job_results.len() {
                let worst = job_results.swap_remove(regret_index);
                let best = job_results.swap_remove(0);

                Some((worst.cost - best.cost, best))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

        if !results.is_empty() {
            results.sort_by(|a, b| compare_floats(b.0, a.0));

            let (_, best_success) = results.swap_remove(0);

            InsertionResult::Success(best_success)
        } else {
            self.inner_reducer.reduce(ctx, jobs, insertion_position)
        }
    }
}
