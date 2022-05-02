use async_cron_scheduler::{Job, Scheduler};
use chrono::offset::Local;
use smol::Timer;
use std::time::Duration;

#[allow(clippy::similar_names)]
fn main() {
    smol::block_on(async move {
        // Creates a scheduler based on the Local timezone. Note that the `sched_service`
        // contains the background job as a future for the caller to decide how to await
        // it. When the scheduler is dropped, the scheduler service will exit as well.
        let (mut scheduler, sched_service) = Scheduler::<Local>::launch(Timer::after);

        // Creates a job which executes every 1 seconds.
        let job = Job::cron("1/1 * * * * *").unwrap();
        let fizz_id = scheduler.insert(job, |_id| println!("Fizz"));

        // Creates a job which executes every 3 seconds.
        let job = Job::cron("1/3 * * * * *").unwrap();
        let buzz_id = scheduler.insert(job, |_id| println!("Buzz"));

        // Creates a job which executes every 5 seconds.
        let job = Job::cron("1/5 * * * * *").unwrap();
        let bazz_id = scheduler.insert(job, |_id| println!("Bazz"));

        // A future which gradually drops jobs from the scheduler.
        let dropper = async move {
            Timer::after(Duration::from_secs(7)).await;
            scheduler.remove(fizz_id);
            println!("Fizz gone");
            Timer::after(Duration::from_secs(5)).await;
            scheduler.remove(buzz_id);
            println!("Buzz gone");
            Timer::after(Duration::from_secs(1)).await;
            scheduler.remove(bazz_id);
            println!("Bazz gone");

            // `scheduler` is dropped here, which causes the sched_service to end.
        };

        // Poll the dropper and scheduler service concurrently until both return.
        futures::future::join(sched_service, dropper).await;
    });
}
