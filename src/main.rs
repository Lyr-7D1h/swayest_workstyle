mod args;
mod config;
mod util;

use config::Config;
use futures_util::StreamExt;
use log::{debug, error};
use swayipc::{
    bail,
    reply::{Node, NodeType},
    Connection, Error, EventType, Fallible,
};

async fn update_workspace_name(config: &mut Config, workspace: &Node) -> Fallible<()> {
    let mut conn = Connection::new().await?;

    let icons: Vec<String> = workspace
        .nodes
        .iter()
        .map(|node| config.fetch_icon(node).to_string())
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

fn get_workspace_with_focus_recurse<'a>(parent: &'a Node, node: &'a Node) -> Option<&'a Node> {
    if node.focused {
        if node.node_type == NodeType::Workspace {
            return Some(node);
        } else if node.node_type == NodeType::Con {
            // println!("{:?}", parent.nodes);
            if parent.node_type == NodeType::Workspace {
                return Some(parent);
            }
        }
    }

    for child in &node.nodes {
        if let Some(n) = get_workspace_with_focus_recurse(node, child) {
            return Some(n);
        }
    }

    return None;
}

fn get_workspace_with_focus(tree: &Node) -> Result<&Node, Error> {
    if let Some(workspace) = get_workspace_with_focus_recurse(tree, tree) {
        return Ok(workspace);
    }

    bail!("Could not find a workspace with focus")
}

async fn update_workspace(con: &mut Connection, config: &mut Config) -> Fallible<()> {
    let tree = con.get_tree().await?;
    let workspace = get_workspace_with_focus(&tree)?;
    update_workspace_name(config, workspace).await?;
    Ok(())
}

async fn subscribe_to_window_events(mut config: Config) -> Fallible<()> {
    let mut events = Connection::new()
        .await?
        .subscribe(&[EventType::Workspace, EventType::Window])
        .await?;

    let mut con = Connection::new().await?;

    while let Some(e) = events.next().await {
        if let Ok(_) = e {
            match update_workspace(&mut con, &mut config).await {
                Ok(_) => {}
                Err(e) => {
                    error!("Could not update workspace name: {}", e);
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
