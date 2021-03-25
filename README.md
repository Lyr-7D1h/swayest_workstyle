# Swayest Workstyle

An executable similar to [workstyle](https://github.com/pierrechevalier83/workstyle).

The main difference between this and `workstyle` is that this support specific app id's and names as well instead of only generic title names.

This ensures that icons are always valid based on the application being run instead of soly relying on application title. (Does not work well with browers, since you can type anything and it will show up in title name)

Besides specific id's you can also still generate icons based on title or set a fallback icon.

Your workspace shall never contain an empty icon again!

## Installation

```
cargo install sworkstyle
```

## Usage

```
sworkstyle
```

## Sway Configuration

```
exec_always sworkstyle &> /tmp/sworkstyle.log
```

Note that since your workspaces will be renamed all the time, you should configure your keybindings to use numbered workspaces instead of assuming that the name is the number:
Prefer

```
    bindsym $mod+1 workspace number 1
```

over

```
    bindsym $mod+1 workspace 1
```

## Configuration

The main configuration consists of deciding which icons to use for which applications.

The config file is located at `${XDG_CONFIG_HOME}/sworkstyle/config.toml`. It will be generated if missing. Read the generated file. The syntax is in TOML and should be pretty self-explanatory.

When an app isn't recogised in the config, `sworkstyle` will log the application name as an error.
Simply add that string (case insensitive) to your config file, with an icon of your choice.

Note that the crate [`find_unicode`](https://github.com/pierrechevalier83/find_unicode/) can help find a unicode character directly from the command line. It now supports all of nerdfonts unicode space.
