use async_std::prelude::*;
use futures::poll;
use inotify::{Inotify, WatchMask};
use std::{
    error::Error,
    path::{Path, PathBuf},
    task::Poll,
    thread,
    time::Duration,
};
use std::collections::HashSet;

use log::{debug, error, info, warn};
use swayipc_async::{Connection, EventType, Node, NodeType};

pub mod config;
mod util;

use config::Config;

pub type SworkstyleError = Box<dyn Error>;

pub struct Sworkstyle {
    config: Config,
    config_path: Option<PathBuf>,
    inotify: Option<Inotify>,
    deduplicate: bool,
    focused_color: Option<String>,
}

impl Sworkstyle {
    pub fn new<P: AsRef<Path>>(config_path: Option<P>, deduplicate: bool, focused_color: Option<String>) -> Sworkstyle {
        let inotify = config_path
            .as_ref()
            .map(|path| {
                if path.as_ref().exists() {
                    let mut inotify =
                        Inotify::init().expect("Error while initializing inotify instance");
                    inotify
                        .add_watch(&path, WatchMask::CLOSE_WRITE)
                        .expect("Failed to watch config file");
                    Some(inotify)
                } else {
                    None
                }
            })
            .flatten();

        Sworkstyle {
            config: Config::new(&config_path),
            config_path: config_path.map(|p| p.as_ref().to_path_buf()),
            inotify,
            deduplicate,
            focused_color,
        }
    }

    pub async fn run(&mut self) -> Result<(), SworkstyleError> {
        let mut events = Connection::new()
            .await?
            .subscribe(&[EventType::Window])
            .await?;
        let mut connection = Connection::new().await?;

        let mut inotify_events_buffer = [0; 1024];
        loop {
            let p = poll!(events.next());

            if p.is_ready() {
                if let Poll::Ready(Some(event)) = p {
                    match event {
                        Ok(_) => {
                            if let Err(e) = self.update_workspaces(&mut connection).await {
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

            if let Some(inotify) = &mut self.inotify {
                if let Ok(_) = inotify.read_events(&mut inotify_events_buffer) {
                    if let Some(config_path) = &self.config_path {
                        info!("Detected config change, reloading config..");
                        self.config = Config::new(&self.config_path);
                        // Reset watcher
                        inotify
                            .add_watch(config_path, WatchMask::CLOSE_WRITE)
                            .expect("Failed to watch config file");
                    }
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    async fn update_workspaces(&self, conn: &mut Connection) -> Result<(), SworkstyleError> {
        let tree = conn.get_tree().await?;

        let mut workspaces = vec![];
        get_workspaces_recurse(&tree, &mut workspaces);

        for workspace in workspaces {
            self.update_workspace_name(conn, workspace).await?;
        }

        Ok(())
    }

    async fn update_workspace_name(
        &self,
        conn: &mut Connection,
        workspace: &Node,
    ) -> Result<(), SworkstyleError> {
        let mut windows = vec![];
        get_windows(workspace, &mut windows);

        let window_names: Vec<(Option<&String>, Option<String>, bool)> = windows
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

                (exact_name, node.name.clone(), node.focused)
            })
            .collect();

        let icons_iter = window_names
            .into_iter()
            .map(|(exact_name, generic_name, focused)| {
                let icon = if let Some(exact_name) = exact_name {
                    self.config
                        .fetch_icon(exact_name, generic_name.as_ref())
                        .to_string()
                } else {
                    error!(
                        "No exact name found for window with title={:?}",
                        generic_name
                    );
                    self.config
                        .fetch_icon(&String::new(), generic_name.as_ref())
                        .to_string()
                };
                if focused {
                    self.focused_color.as_ref().map(|color| {
                        format!("<span color='{color}'>{icon}</span>")
                    }).unwrap_or(icon)
                } else {
                    format!("\u{202D}{icon}\u{202C}")
                }
            })
            .filter(|icon| !icon.is_empty())
            // Overwrite right to left characters: https://www.unicode.org/versions/Unicode12.0.0/UnicodeStandard-12.0.pdf#G26.16327
            .map(|icon| format!("\u{202D}{icon}\u{202C}"));

        let icons: Vec<String> = if self.deduplicate {
            let mut seen = HashSet::new();
            icons_iter.filter(|icon| {
                if seen.contains(icon) {
                    return false;
                }
                seen.insert(icon.clone());
                true
            }).collect()
        } else {
            icons_iter.collect()
        };

        let name = match &workspace.name {
            Some(name) => name,
            None => {
                return Err(
                    format!("Could not get name for workspace with id: {}", workspace.id).into(),
                );
            }
        };

        let index = match workspace.num {
            Some(num) => num,
            None => return Err(format!("Could not fetch index for: {}", name).into()),
        };

        let delim = self.config.separator
            .as_deref()
            .unwrap_or(" ");

        let mut icons = icons.join(delim);

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
