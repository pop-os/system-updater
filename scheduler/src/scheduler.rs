// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::{Job, JobId, TimeZoneExt};
use chrono::DateTime;
use flume::{RecvError, Sender};
use futures::future::Either;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::{future::Future, time::Duration};

/// A scheduled command associated with a job.
pub type Command = Box<dyn Fn(JobId) + Send + Sync>;

/// Messages going into the scheduler service.
enum SchedMessage<Tz: TimeZoneExt> {
    Insert(JobId, Job<Tz>, Command),
    Remove(JobId),
}

/// The interface for interacting with the scheduler.
///
/// When launching a scheduler, the scheduler and its service are created
/// simultaneously with a channel connecting them. Job insert and remove
/// messages are sent to the service for automatic management. When the
/// scheduler is dropped, so too will the service its attached to exit.
///
/// ```
/// use smol::Timer;
/// use chrono::offset::Local;
///
/// let (mut scheduler, service) = Scheduler::<Local>::launch(Timer::after);
///
/// // Creates a job which executes every 3 seconds.
/// let job = Job::cron("1/3 * * * * *").unwrap();
/// let fizz_id = scheduler.insert(job, |id| println!("Fizz"));
///
/// // Creates a job which executes every 5 seconds.
/// let job = Job::cron("1/5 * * * * *").unwrap();
/// let buzz_id = scheduler.insert(job, |id| println!("Buzz"));
///
/// service.await;
/// ```
pub struct Scheduler<Tz: TimeZoneExt> {
    jobs: SlotMap<DefaultKey, ()>,
    sender: Sender<SchedMessage<Tz>>,
}

impl<Tz: TimeZoneExt + 'static> Scheduler<Tz>
where
    Tz::Offset: Send + Sync,
{
    /// Insert a job into the scheduler with the command to call when scheduled.
    ///
    /// ```
    /// // Creates a job which executes every 3 seconds.
    /// let job = Job::cron("1/3 * * * * *").unwrap();
    /// let fizz_id = scheduler.insert(job, |id| println!("Fizz"));
    /// ```
    pub fn insert(
        &mut self,
        job: Job<Tz>,
        command: impl Fn(JobId) + Send + Sync + 'static,
    ) -> JobId {
        let id = JobId(self.jobs.insert(()));
        let _result = self
            .sender
            .send(SchedMessage::Insert(id, job, Box::new(command)));
        id
    }

    /// Remove a scheduled job from the scheduler.
    ///
    /// ```
    /// scheduler.remove(fizz_id);
    /// ```
    pub fn remove(&mut self, job: JobId) {
        if self.jobs.remove(job.0).is_some() {
            let _res = self.sender.send(SchedMessage::Remove(job));
        }
    }

    /// Initializes the scheduler and its connected service.
    ///
    /// The API is designed to not rely on any async runtimes. This is achieved by
    /// returning a future to allow the caller to decide how it should be executed,
    /// and taking a function for handling sleeps. You can choose to spawn the
    /// returned future, or avoid spawning altgether and await it directly from the
    /// same thread.
    ///
    /// ## Smol runtime
    ///
    /// ```
    /// let (mut scheduler, sched_service) = Scheduler::<Local>::launch(smol::Timer::after);
    /// smol::spawn(sched_service).detach();
    /// ```
    ///
    /// ## Tokio runtime
    ///
    /// ```
    /// let (mut scheduler, sched_service) = Scheduler::<Local>::launch(tokio::time::sleep);
    /// tokio::spawn(sched_service);
    /// ```
    pub fn launch<F, T, X>(timer: T) -> (Self, impl Future<Output = ()> + Send + Sync + 'static)
    where
        F: Future<Output = X> + Send + Sync,
        T: Fn(Duration) -> F + Send + Sync + 'static,
    {
        let (sender, receiver) = flume::unbounded();

        let task = async move {
            let mut state = SchedulerModel {
                tasks: SecondaryMap::new(),
                next: None,
            };

            loop {
                match state.next.take() {
                    Some((key, duration)) => {
                        let message = receiver.recv_async();
                        let wait = timer(duration);

                        futures::pin_mut!(message);
                        futures::pin_mut!(wait);

                        match futures::future::select(message, wait).await {
                            Either::Left((message, _)) => match message {
                                Ok(message) => state.update(message),
                                Err(RecvError::Disconnected) => break,
                            },

                            Either::Right((_, _)) => {
                                state.call(key);
                                state.next();
                            }
                        }
                    }

                    None => match receiver.recv_async().await {
                        Ok(message) => state.update(message),
                        Err(RecvError::Disconnected) => break,
                    },
                };
            }
        };

        (
            Self {
                sender,
                jobs: SlotMap::new(),
            },
            task,
        )
    }
}
struct SchedulerModel<Tz: TimeZoneExt> {
    tasks: SecondaryMap<DefaultKey, (Job<Tz>, Command)>,
    next: Option<(DefaultKey, Duration)>,
}

impl<Tz: TimeZoneExt> SchedulerModel<Tz> {
    pub fn call(&mut self, key: DefaultKey) {
        if let Some((job, func)) = self.tasks.get_mut(key) {
            func(JobId(key));

            if let Some(next) = job.iterator.next() {
                job.next = next;

                return;
            }

            self.tasks.remove(key);
        }
    }

    pub fn next(&mut self) {
        loop {
            let mut next: Option<(DefaultKey, DateTime<Tz>)> = None;

            for (id, (job, _)) in self.tasks.iter() {
                if let Some((_, date_time)) = next.as_ref() {
                    if date_time.timestamp() > job.next.timestamp() {
                        next = Some((id, job.next.clone()));
                    }

                    continue;
                }

                next = Some((id, job.next.clone()));
            }

            if let Some((id, date)) = next {
                let seconds_until = date.signed_duration_since(Tz::now()).num_seconds();
                if let Ok(seconds_until) = u64::try_from(seconds_until) {
                    if seconds_until > 0 {
                        #[cfg(feature = "logging")]
                        tracing::info!("next job in {} seconds", seconds_until);

                        let duration = Duration::from_secs(seconds_until as u64);
                        self.next = Some((id, duration));
                        return;
                    }
                }

                self.call(id);
                continue;
            }

            return;
        }
    }

    pub fn update(&mut self, message: SchedMessage<Tz>) {
        match message {
            SchedMessage::Insert(id, job, func) => {
                self.tasks.insert(id.0, (job, func));
                self.next();
            }

            SchedMessage::Remove(id) => {
                self.tasks.remove(id.0);
                self.next();
            }
        }
    }
}
