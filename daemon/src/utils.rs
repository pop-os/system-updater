// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

use anyhow::Context;
use as_result::IntoResult;
use async_process::Command;

pub async fn async_commands(cmds: &[&[&str]]) -> anyhow::Result<()> {
    for command in cmds {
        async_command(command).await?;
    }

    Ok(())
}

pub async fn async_command(args: &[&str]) -> anyhow::Result<()> {
    if args.is_empty() {
        return Err(anyhow::anyhow!(
            "async_command fn invoked without arguments"
        ));
    }

    let mut cmd = Command::new(args[0]);

    if args.len() > 1 {
        cmd.args(&args[1..]);
    }

    cmd.status()
        .await
        .and_then(IntoResult::into_result)
        .with_context(|| format!("command execution failed for {:?}", args))?;

    Ok(())
}

pub fn command_exists(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for location in path.split(':') {
            if std::fs::metadata(&[location, "/", cmd].concat()).is_ok() {
                return true;
            }
        }
    }

    false
}

pub async fn error_handler(conn: &zbus::Connection, source: &str, error: anyhow::Error) {
    use std::fmt::Write;

    let mut chain = anyhow::Chain::new(error.as_ref());

    let mut output = String::new();

    if let Some(why) = chain.next() {
        let _ = write!(&mut output, "{}", why);
        output.push_str(&why.to_string());
        for why in chain {
            let _ = write!(&mut output, ": {}", why);
        }

        error!("{}: {}", source, output);

        use pop_system_updater::dbus::server::{context, Server};

        context(conn, |ctx| async move {
            Server::error(&ctx, source, &output).await
        })
        .await;
    }
}
