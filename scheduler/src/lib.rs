// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

//! Simple lightweight efficient runtime-agnostic async task scheduler with cron expression support
//!
//! # Features
//!
//! - **Simple**: The most important feature of all, integrate easily in any codebase.
//! - **Lightweight**: Minimal dependencies with a small amount of code implementing it.
//! - **Efficient**: Tickless design with no reference counters and light structs.
//! - **Runtime-Agnostic**: Bring your own runtime. No runtime dependencies.
//! - **Async**: A single future drives the entire scheduler service.
//! - **Task Scheduling**: Schedule multiple jobs with varying timeframes between them.
//! - **Cron Expressions**: Standardized format for scheduling syntax.
//!
//! # Tips
//!
//! Scheduled jobs block the executor when they are executing, so it's best to keep
//! their execution short. It's recommended practice to either spawn tasks onto an
//! executor, or send messages from a channel. The good news is that each job being
//! executed has a unique ID associated with it, which you can use for tracking
//! specific tasks.
//!
//! # Demo
//!
//! The entire API wrapped up in one example.
//!
//! ```
//! use chrono::offset::Local;
//! use async_cron_scheduler::*;
//! use smol::Timer;
//! use std::time::Duration;
//!
//! smol::block_on(async move {
//!     // Creates a scheduler based on the Local timezone. Note that the `sched_service`
//!     // contains the background job as a future for the caller to decide how to await
//!     // it. When the scheduler is dropped, the scheduler service will exit as well.
//!     let (mut scheduler, sched_service) = Scheduler::<Local>::launch(Timer::after);
//!
//!     // Creates a job which executes every 1 seconds.
//!     let job = Job::cron("1/1 * * * * *").unwrap();
//!     let fizz_id = scheduler.insert(job, |id| println!("Fizz"));
//!
//!     // Creates a job which executes every 3 seconds.
//!     let job = Job::cron("1/3 * * * * *").unwrap();
//!     let buzz_id = scheduler.insert(job, |id| println!("Buzz"));
//!
//!     // Creates a job which executes every 5 seconds.
//!     let job = Job::cron("1/5 * * * * *").unwrap();
//!     let bazz_id = scheduler.insert(job, |id| println!("Bazz"));
//!
//!     // A future which gradually drops jobs from the scheduler.
//!     let dropper = async move {
//!         Timer::after(Duration::from_secs(7)).await;
//!         scheduler.remove(fizz_id);
//!         println!("Fizz gone");
//!         Timer::after(Duration::from_secs(5)).await;
//!         scheduler.remove(buzz_id);
//!         println!("Buzz gone");
//!         Timer::after(Duration::from_secs(1)).await;
//!         scheduler.remove(bazz_id);
//!         println!("Bazz gone");
//!
//!         // `scheduler` is dropped here, which causes the sched_service to end.
//!     };
//!
//!     // Poll the dropper and scheduler service concurrently until both return.
//!     futures::future::join(sched_service, dropper).await;
//! });
//! ```

use chrono::DateTime;
use chrono::TimeZone;
pub use cron;

mod job;
mod scheduler;

pub use self::job::*;
pub use self::scheduler::*;

/// Extensions for the chrono timezone structs.
pub trait TimeZoneExt: TimeZone + Copy + Clone {
    /// Constructs a default timezone struct for this timezone.
    fn timescale() -> Self;

    /// Get the current time in this timezone.
    fn now() -> DateTime<Self>;
}

impl TimeZoneExt for chrono::Local {
    fn timescale() -> Self {
        Self
    }
    fn now() -> DateTime<Self> {
        Self::now()
    }
}

impl TimeZoneExt for chrono::Utc {
    fn timescale() -> Self {
        Self
    }

    fn now() -> DateTime<Self> {
        Self::now()
    }
}
