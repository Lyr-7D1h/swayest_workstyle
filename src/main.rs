mod args;
mod config;
mod util;

use std::process;

use args::Args;
use config::Config;
use fslock::LockFile;
use futures_util::StreamExt;
use log::{debug, error};
use simple_logger::SimpleLogger;
use swayipc::{
    bail,
    reply::{Node, NodeType},
    Connection, EventType, Fallible,
};

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
) -> Fallible<()> {
    let mut windows = vec![];
    get_windows(workspace, &mut windows);

    let icons: Vec<String> = windows
        .iter()
        .map(|node| config.fetch_icon(&node).to_string())
        .collect();

    let name = match &workspace.name {
        Some(name) => name,
        None => bail!("Could not get name for workspace with id: {}", workspace.id),
    };

    let index = match workspace.num {
        Some(num) => num,
        None => bail!("Could not fetch index for: {}", name),
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

async fn update_workspaces(conn: &mut Connection, config: &mut Config) -> Fallible<()> {
    let tree = conn.get_tree().await?;

    let mut workspaces = vec![];
    get_workspaces_recurse(&tree, &mut workspaces);

    for workspace in workspaces {
        update_workspace_name(conn, config, workspace).await?;
    }

    Ok(())
}

async fn subscribe_to_window_events(mut config: Config) -> Fallible<()> {
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
    let mut file = LockFile::open("/tmp/sworkstyle.lock").unwrap();

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

#[tokio::main]
async fn main() -> Fallible<()> {
    let args = Args::from_cli();

    SimpleLogger::new()
        .with_level(args.log_level)
        .init()
        .unwrap();

    check_already_running();

    let config = Config::new();

    subscribe_to_window_events(config).await?;

    Ok(())
}
