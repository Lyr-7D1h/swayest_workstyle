use futures_util::stream::StreamExt;
use swayipc_async::{Connection, EventType, Fallible};
use swayipc_async::{Event::Window, Workspace};

mod args;
mod config;

use config::Config;

async fn subscribe_to_window_events(config: Config) -> Fallible<()> {
    let mut events = Connection::new()
        .await?
        .subscribe(&[EventType::Window])
        .await?;

    // #TODO Switch to workspace event instead of window
    while let Some(event) = events.next().await {
        if let Ok(e) = event {
            if let Window(we) = e {
                println!("{:?}", we.container.node_type);

                let icon = config.fetch_icon(&we.container);

                // #TODO ensure changes
                let mut conn = Connection::new().await?;

                let workspaces: Vec<Workspace> = conn.get_workspaces().await?;
                let workspaces: Vec<&Workspace> = workspaces
                    .iter()
                    .filter(|w| {
                        w.focus
                            .iter()
                            .map(|k| *k as i64)
                            .collect::<Vec<i64>>()
                            .contains(&we.container.id)
                    })
                    .collect();

                if workspaces.len() > 1 {
                    println!(
                        "Found more than one workspace for the given icon {:?}",
                        workspaces
                    );
                    continue;
                } else if workspaces.len() == 0 {
                    println!("No workspaces found for {:?}", we.container);
                    continue;
                }

                let name = &workspaces[0].name;

                println!("{}", name);

                conn.run_command(format!("rename workspace \"{}\" to \"{}\"", name, icon))
                    .await?;
            }
        }
    }

    return Ok(());
}

#[tokio::main]
async fn main() -> Fallible<()> {
    args::help();

    let config = Config::new()?;

    subscribe_to_window_events(config).await?;

    Ok(())
}
