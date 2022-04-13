# pop-task-scheduler

Simple lightweight efficient runtime-agnostic async task scheduler with cron expression support

## Features

- **Simple**: The most important feature of all, integrate easily in any codebase.
- **Lightweight**: Minimal dependencies with a small amount of code implementing it.
- **Efficient**: Tickless design with no reference counters and light structs.
- **Runtime-Agnostic**: Bring your own runtime. No runtime dependencies.
- **Async**: A single future drives the entire scheduler service.
- **Task Scheduling**: Schedule multiple jobs with varying timeframes between them.
- **Cron Expressions**: Standardized format for scheduling syntax.

## Tips

Scheduled jobs block the executor when they are executing, so it's best to keep their execution short. It's recommended practice to either spawn tasks onto an executor, or send messages from a channel. The good news is that each job being executed has a unique ID associated with it, which you can use for tracking specific tasks.

## Demo

[Example here](./examples/simple.rs)

## License

Licensed under the [Mozilla Public License 2.0](https://choosealicense.com/licenses/mpl-2.0/).

### Contribution

Any contribution intentionally submitted for inclusion in the work by you shall be licensed under the Mozilla Public License 2.0 (MPL-2.0).
