mod args;
mod config;
mod util;

use async_std::prelude::*;
use futures::poll;
use inotify::{Inotify, WatchMask};
use std::{collections::BTreeSet, error::Error, process, task::Poll, thread, time::Duration};

use args::Args;
use config::Config;
use fslock::LockFile;
use log::{debug, error, info, warn};
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
    config: &Config,
    args: &Args,
    workspace: &Node,
) -> Result<(), Box<dyn Error>> {
    let mut windows = vec![];
    get_windows(workspace, &mut windows);

    let mut window_names: Vec<(Option<&String>, Option<String>)> = windows
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

            (exact_name, node.name.clone())
        })
        .collect();

    if args.deduplicate {
        window_names = window_names
            .into_iter()
            .collect::<BTreeSet<(Option<&String>, Option<String>)>>()
            .into_iter()
            .collect();
    }

    let mut icons: Vec<String> = window_names
        .into_iter()
        .map(|(exact_name, generic_name)| {
            if let Some(exact_name) = exact_name {
                config
                    .fetch_icon(exact_name, generic_name.as_ref())
                    .to_string()
            } else {
                error!(
                    "No exact name found for window with title={:?}",
                    generic_name
                );
                config
                    .fetch_icon(&String::new(), generic_name.as_ref())
                    .to_string()
            }
        })
        // Overwrite right to left characters: https://www.unicode.org/versions/Unicode12.0.0/UnicodeStandard-12.0.pdf#G26.16327
        .map(|icon| format!("\u{202D}{icon}\u{202C}"))
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

    if args.deduplicate {
        icons.dedup();
    }

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
    config: &Config,
    args: &Args,
) -> Result<(), Box<dyn Error>> {
    let tree = conn.get_tree().await?;

    let mut workspaces = vec![];
    get_workspaces_recurse(&tree, &mut workspaces);

    for workspace in workspaces {
        update_workspace_name(conn, config, args, workspace).await?;
    }

    Ok(())
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

async fn main_loop(mut config: Config, args: &Args) -> Result<(), Box<dyn Error>> {
    let mut events = Connection::new()
        .await?
        .subscribe(&[EventType::Window])
        .await?;
    let mut connection = Connection::new().await?;

    let mut inotify = Inotify::init().expect("Error while initializing inotify instance");
    if let Some(config_path) = &args.config_path {
        if config_path.exists() {
            inotify
                .add_watch(config_path, WatchMask::CLOSE_WRITE)
                .expect("Failed to watch config file");
        }
    }
    let mut inotify_events_buffer = [0; 1024];

    loop {
        let p = poll!(events.next());

        if p.is_ready() {
            if let Poll::Ready(Some(event)) = p {
                match event {
                    Ok(_) => {
                        if let Err(e) = update_workspaces(&mut connection, &config, args).await {
                            error!("Could not update workspace name: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Connection broken, exiting: {e}");
                        return Err(Box::new(e));
                    }
                }
            }
        }

        if let Ok(_) = inotify.read_events(&mut inotify_events_buffer) {
            info!("Detected config change, reloading config..");
            config = Config::new(args.config_path.as_ref());

            // Reset watcher
            if let Some(config_path) = &args.config_path {
                if config_path.exists() {
                    inotify
                        .add_watch(config_path, WatchMask::CLOSE_WRITE)
                        .expect("Failed to watch config file");
                }
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

#[async_std::main]
async fn main() {
    let args = Args::from_cli();

    SimpleLogger::new()
        .with_level(args.log_level)
        .init()
        .expect("Could not load simple logger");

    check_already_running();

    let config = Config::new(args.config_path.as_ref());

    if let Err(e) = main_loop(config, &args).await {
        error!("{e}")
    }
}
