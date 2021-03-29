mod args;
mod config;
mod util;

use config::Config;
use futures_util::StreamExt;
use log::{debug, error};
use swayipc::{
    bail,
    reply::{Event, Node},
    Connection, EventType, Fallible,
};

async fn update_workspace_name(config: &Config, workspace: &Node) -> Fallible<()> {
    let mut conn = Connection::new().await?;

    let icons: Vec<String> = workspace
        .nodes
        .iter()
        .map(|node| config.fetch_icon(node).to_string())
        .collect();

    println!("{:?}", icons);

    let name = match &workspace.name {
        Some(name) => name,
        None => bail!("Could not get name for workspace with id: {}", workspace.id),
    };

    let index = match workspace.num {
        Some(num) => num,
        None => bail!("Could not fetch index for: {}", name),
    };

    let new_name = format!("{}: {} ", index, icons.join(" "));

    debug!("rename workspace \"{}\" to \"{}\"", name, new_name);

    conn.run_command(format!("rename workspace \"{}\" to \"{}\"", name, new_name))
        .await?;

    return Ok(());
}

fn find_node_with_id(node_id: &i64, node: Node) -> Option<Node> {
    for node in node.nodes {
        if node.focus.contains(node_id) {
            return Some(node);
        } else {
            if let Some(n) = find_node_with_id(node_id, node) {
                return Some(n);
            }
        }
    }
    return None;
}

async fn get_workspace_for_window(window_id: &i64) -> Fallible<Node> {
    let mut conn = Connection::new().await?;

    let tree = conn.get_tree().await?;

    if let Some(workspace) = find_node_with_id(window_id, tree) {
        return Ok(workspace);
    }

    bail!(format!("Could not find a workspace for {}", window_id))
}

async fn subscribe_to_window_events(config: Config) -> Fallible<()> {
    let mut events = Connection::new()
        .await?
        .subscribe(&[EventType::Workspace, EventType::Window])
        .await?;

    while let Some(e) = events.next().await {
        if let Ok(event) = e {
            if let Event::Workspace(we) = &event {
                debug!("Workspace update");
                if let Some(workspace) = &we.current {
                    if let Err(e) = update_workspace_name(&config, workspace).await {
                        error!("{}", e)
                    }
                }
            }
            if let Event::Window(we) = &event {
                match get_workspace_for_window(&we.container.id).await {
                    Ok(workspace) => {
                        debug!("Window update");
                        if let Err(e) = update_workspace_name(&config, &workspace).await {
                            error!("{}", e)
                        }
                    }
                    Err(e) => error!("{}", e),
                }
            }
        }
    }

    return Ok(());
}

#[tokio::main]
async fn main() -> Fallible<()> {
    args::setup();

    let config = Config::new()?;

    subscribe_to_window_events(config).await?;

    Ok(())
}
