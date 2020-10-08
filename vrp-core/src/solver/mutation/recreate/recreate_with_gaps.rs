use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;
use rand::prelude::*;

/// Returns a sub set of randomly selected jobs.
struct GapsJobSelector {
    min_jobs: usize,
}

impl JobSelector for GapsJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        // TODO we should prefer to always insert locked jobs
        ctx.solution.required.shuffle(&mut ctx.random.get_rng());

        // TODO improve formula
        let max_jobs = self.min_jobs.max(ctx.solution.required.len());
        let take_jobs = ctx.random.uniform_int(self.min_jobs as i32, max_jobs as i32) as usize;

        Box::new(ctx.solution.required.iter().take(take_jobs).cloned())
    }
}

/// A recreate method which selects on each insertion step only subset of randomly chosen jobs.
pub struct RecreateWithGaps {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl RecreateWithGaps {
    /// Creates a new instance of `RecreateWithGaps`.
    pub fn new(min_jobs: usize) -> Self {
        Self {
            job_selector: Box::new(GapsJobSelector { min_jobs }),
            job_reducer: Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
            )),
        }
    }
}

impl Default for RecreateWithGaps {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Recreate for RecreateWithGaps {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}
