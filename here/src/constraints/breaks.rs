use core::construction::constraints::*;
use core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use core::models::common::{IdDimension, ValueDimension};
use core::models::problem::{Job, Single};
use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;

pub struct BreakModule {
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl BreakModule {
    pub fn new(code: i32) -> Self {
        Self {
            conditional: ConditionalJobModule::new(Box::new(|ctx, job| is_required_job(ctx, job))),
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(BreakHardActivityConstraint { code }))],
        }
    }
}

impl ConstraintModule for BreakModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.conditional.accept_route_state(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);
        demote_unassigned_breaks(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.conditional.state_keys()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct BreakHardActivityConstraint {
    code: i32,
}

impl HardActivityConstraint for BreakHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let is_break =
            activity_ctx.target.job.as_ref().and_then(|job| Some(job.as_single())).map_or(false, |job| is_break(&job));

        // avoid assigning break right after departure
        if is_break && activity_ctx.prev.job.is_none() {
            Some(ActivityConstraintViolation { code: self.code, stopped: false })
        } else {
            None
        }
    }
}

/// Mark job as ignored only if it has break type and vehicle id is not present in routes
fn is_required_job(ctx: &SolutionContext, job: &Arc<Job>) -> bool {
    match job.as_ref() {
        Job::Single(job) => {
            if is_break(job) {
                let vehicle_id = get_vehicle_id_from_break(job.as_ref()).unwrap();
                !ctx.required.is_empty() && ctx.routes.iter().any(move |rc| get_vehicle_id_from_ctx(rc) == vehicle_id)
            } else {
                true
            }
        }
        Job::Multi(_) => true,
    }
}

fn is_break(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "break")
}

/// Remove some breaks from required jobs as we don't want to consider breaks
/// as unassigned jobs if they are outside of vehicle's time window
fn demote_unassigned_breaks(ctx: &mut SolutionContext) {
    if ctx.unassigned.is_empty() {
        return;
    }
    // TODO do we need to check break-tour time window intersection?

    //    let vehicle_routes = ctx.routes.iter().fold(HashMap::new(), |mut acc, rc| {
    //        let vehicle_id = get_vehicle_id_from_ctx(rc);
    //
    //        let departure = rc.route.tour.all_activities().next().unwrap().schedule.arrival;
    //        let arrival = rc.route.tour.all_activities().last().unwrap().schedule.departure;
    //
    //        acc.entry(vehicle_id).or_insert_with(|| vec![]).push(TimeWindow::new(departure, arrival));
    //
    //        acc
    //    });
    //
    //    let breaks_set: HashSet<_> = ctx
    //        .unassigned
    //        .iter()
    //        .filter_map(|(job, _)| match job.as_ref() {
    //            Job::Single(single) => {
    //                get_vehicle_id_from_break(single.as_ref()).and_then(|id| vehicle_routes.get(&id)).and_then(|times| {
    //                    if single.places.iter().any(|p| TimeWindow::intersects_many(&p.times, times)) {
    //                        Some(job.clone())
    //                    } else {
    //                        None
    //                    }
    //                })
    //            }
    //            Job::Multi(_) => None,
    //        })
    //        .collect();
    //
    //    ctx.unassigned.retain(|job, _| breaks_set.get(job).is_none());
    //    ctx.ignored.extend(breaks_set.into_iter());

    // NOTE remove all breaks from list of unassigned jobs
    let breaks_set: HashSet<_> = ctx
        .unassigned
        .iter()
        .filter_map(|(job, _)| match job.as_ref() {
            Job::Single(single) => get_vehicle_id_from_break(single.as_ref()).map(|_| job.clone()),
            Job::Multi(_) => None,
        })
        .collect();

    ctx.unassigned.retain(|job, _| breaks_set.get(job).is_none());
    ctx.ignored.extend(breaks_set.into_iter());
}

fn get_vehicle_id_from_ctx(ctx: &RouteContext) -> String {
    ctx.route.actor.vehicle.dimens.get_id().unwrap().clone()
}

fn get_vehicle_id_from_break(job: &Single) -> Option<String> {
    job.dimens.get_value::<String>("vehicle_id").cloned()
}
