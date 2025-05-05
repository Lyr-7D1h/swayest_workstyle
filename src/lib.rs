use futures_lite::prelude::*;

use async_io::Async;
use futures_lite::stream;
use inotify::{Inotify, WatchMask};
use std::{
    collections::BTreeSet,
    error::Error,
    path::{Path, PathBuf},
};

use log::{debug, error, info, warn};
use swayipc_async::{Connection, Event, EventType, Node, NodeType, WindowChange};

pub mod config;
mod util;

use config::Config;

pub type SworkstyleError = Box<dyn Error>;

struct ConfigSource {
    path: PathBuf,
    inotify: Option<Inotify>,
}

impl ConfigSource {
    fn new(path: impl AsRef<Path>) -> ConfigSource {
        let inotify = if path.as_ref().exists() {
            let inotify = Inotify::init().expect("Error while initializing inotify instance");
            inotify
                .watches()
                .add(&path, WatchMask::CLOSE_WRITE)
                .expect("Failed to watch config file");
            Some(inotify)
        } else {
            None
        };

        ConfigSource {
            path: path.as_ref().to_path_buf(),
            inotify,
        }
    }
}

pub struct Sworkstyle {
    config: Config,
    config_source: Option<ConfigSource>,
    deduplicate: bool,
}

impl Sworkstyle {
    pub fn new<P: AsRef<Path>>(config_path: Option<P>, deduplicate: bool) -> Sworkstyle {
        Sworkstyle {
            config: Config::new(&config_path),
            config_source: config_path.map(ConfigSource::new),
            deduplicate,
        }
    }

    // Takes `self` by value because we consume `config_source`.
    pub async fn run(mut self) -> Result<(), SworkstyleError> {
        enum Message {
            Event(Event),
            Config(Config),
        }

        let mut events = Connection::new()
            .await?
            .subscribe(&[EventType::Window])
            .await?
            .map(|r| r.map(Message::Event))
            .boxed();
        let mut connection = Connection::new().await?;

        if let Some(mut source) = self.config_source.take() {
            if let Some(inotify) = source.inotify.take() {
                events = events
                    .or(stream::try_unfold(
                        (source.path, inotify),
                        |(path, inotify)| async {
                            let anotify = Async::new(inotify)?;
                            anotify.readable().await?;
                            let mut inotify = anotify.into_inner()?;
                            let mut inotify_events_buffer = [0; 1024];
                            inotify.read_events(&mut inotify_events_buffer)?;
                            info!("Detected config change, reloading config..");
                            let config = Config::new(&Some(&path));
                            // Reset watcher
                            inotify
                                .watches()
                                .add(&path, WatchMask::CLOSE_WRITE)
                                .expect("Failed to watch config file");

                            Ok(Some((Message::Config(config), (path, inotify))))
                        },
                    ))
                    .boxed();
            }
        }

        if let Err(e) = self.update_workspaces(&mut connection).await {
            error!("Could not initialize workspace name: {}", e);
        }

        while let Some(msg) = events.next().await {
            match msg {
                Ok(Message::Event(Event::Window(e))) => {
                    if matches!(
                        e.change,
                        WindowChange::Focus
                            | WindowChange::FullscreenMode
                            | WindowChange::Floating
                            | WindowChange::Urgent
                            | WindowChange::Mark
                    ) {
                        // Event not relevant to us: skip the update_workspaces_call below.
                        continue;
                    }
                }
                // Should not be reachable: we are only subscribed to window events.
                Ok(Message::Event(_)) => {}
                Ok(Message::Config(config)) => {
                    self.config = config;
                }
                Err(e) => {
                    warn!("Error while waiting for Sway or config events, exiting: {e}");
                    return Err(Box::new(e));
                }
            }
            if let Err(e) = self.update_workspaces(&mut connection).await {
                error!("Could not update workspace name: {}", e);
            }
        }

        Ok(())
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

        if self.deduplicate {
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
                }
            })
            .filter(|icon| !icon.is_empty())
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

        if self.deduplicate {
            icons.dedup();
        }

        let delim = self.config.separator.as_deref().unwrap_or(" ");

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
