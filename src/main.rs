mod args;
mod config;
mod util;

use std::{error::Error, process};

use args::Args;
use async_std::prelude::StreamExt;
use config::Config;
use fslock::LockFile;
use log::{debug, error};
use simple_logger::SimpleLogger;
use swayipc_async::{Connection, EventType, Node, NodeType};

/// Rescursively add nodes with node type floatingCon and con to windows
fn get_windows<'a>(node: &'a Node, windows: &mut Vec<&'a Node>) {
    if node.node_type == NodeType::FloatingCon || node.node_type == NodeType::Con {
        if let Some(_) = node.name {
            windows.push(node)
        }
    };

    for node in node.nodes.iter().chain(node.floating_nodes.iter()) {
        get_windows(node, windows);
    }
}

async fn update_workspace_name(
    conn: &mut Connection,
    config: &mut Config,
    workspace: &Node,
) -> Result<(), Box<dyn Error>> {
    let mut windows = vec![];
    get_windows(workspace, &mut windows);

    let icons: Vec<String> = windows
        .iter()
        .map(|node| {
            let mut exact_name: Option<&String> = None;

            // Wayland Exact app
            if let Some(app_id) = &node.app_id {
                exact_name = Some(app_id);
            }

            // X11 Exact
            if let Some(window_props) = &node.window_properties {
                if let Some(class) = &window_props.class {
                    exact_name = Some(class);
                }
            }

            if let Some(exact_name) = exact_name {
                config
                    .fetch_icon(exact_name, node.name.as_ref())
                    .to_string()
            } else {
                error!(
                    "No exact name found for app_id={:?} and title={:?}",
                    node.app_id, node.name
                );
                config
                    .fetch_icon(&String::new(), node.name.as_ref())
                    .to_string()
            }
        })
        .collect();

    let name = match &workspace.name {
        Some(name) => name,
        None => {
            return Err(
                format!("Could not get name for workspace with id: {}", workspace.id).into(),
            )
        }
    };

    let index = match workspace.num {
        Some(num) => num,
        None => return Err(format!("Could not fetch index for: {}", name).into()),
    };

    let mut icons = icons.join(" ");
    if icons.len() > 0 {
        icons.push_str(" ")
    }

    let new_name = if icons.len() > 0 {
        format!("{}: {}", index, icons)
    } else if let Some(num) = workspace.num {
        format!("{}", num)
    } else {
        error!("Could not fetch workspace num for: {:?}", workspace.name);
        " ".to_string()
    };

    if *name != new_name {
        debug!("rename workspace \"{}\" to \"{}\"", name, new_name);

        conn.run_command(format!("rename workspace \"{}\" to \"{}\"", name, new_name))
            .await?;
    }

    return Ok(());
}

fn get_workspaces_recurse<'a>(node: &'a Node, workspaces: &mut Vec<&'a Node>) {
    if node.node_type == NodeType::Workspace && node.name != Some("__i3_scratch".to_string()) {
        workspaces.push(node);
        return;
    }

    for child in node.nodes.iter() {
        get_workspaces_recurse(child, workspaces)
    }
}

async fn update_workspaces(
    conn: &mut Connection,
    config: &mut Config,
) -> Result<(), Box<dyn Error>> {
    let tree = conn.get_tree().await?;

    let mut workspaces = vec![];
    get_workspaces_recurse(&tree, &mut workspaces);

    for workspace in workspaces {
        update_workspace_name(conn, config, workspace).await?;
    }

    Ok(())
}

async fn subscribe_to_window_events(mut config: Config) -> Result<(), Box<dyn Error>> {
    debug!("Subscribing to window events");
    let mut events = Connection::new()
        .await?
        .subscribe(&[EventType::Window])
        .await?;

    let mut con = Connection::new().await?;

    while let Some(event) = events.next().await {
        if let Ok(_) = event {
            if let Err(e) = update_workspaces(&mut con, &mut config).await {
                error!("Could not update workspace name: {}", e);
            }
        }
    }

    return Ok(());
}

fn check_already_running() {
    let mut file = match LockFile::open("/tmp/sworkstyle.lock") {
        Ok(f) => f,
        _ => return,
    };

    let locked = file.try_lock().unwrap();

    if locked == false {
        error!("Sworkstyle already running");
        process::exit(1)
    }

    ctrlc::set_handler(move || {
        debug!("Unlocking /tmp/sworkstyle.lock");
        file.unlock().unwrap();
        process::exit(0)
    })
    .expect("Could not set ctrlc handler")
}

#[async_std::main]
async fn main() {
    let args = Args::from_cli();

    SimpleLogger::new()
        .with_level(args.log_level)
        .init()
        .expect("Could not load simple logger");

    check_already_running();

    let config = Config::new(args.config_path);

    subscribe_to_window_events(config).await.unwrap();
}
