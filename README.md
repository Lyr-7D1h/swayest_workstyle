# Swayest Workstyle

Map workspace name to icons defined depending on the windows inside of the workspace.

An executable similar to [workstyle](https://github.com/pierrechevalier83/workstyle).

The main difference between this and `workstyle` is that this supports exact app names instead of only generic titles.

Meant to work best/only with Wayland and Sway.

It also supports a fallback icon for when it couldn't match an App.

This ensures that icons are always valid based on the application being run instead of soly relying on application title. (Does not work well with browers, since you can type anything and it will show up in title name)

Your workspace shall never contain an empty icon again!

**An example of what it does (using waybar which also hides the workspace index):**

<img src="./screenshots/bar.png">
<br />
<img src="./screenshots/desktop.png" width="1000">

## Installation

### Cargo

```
cargo install sworkstyle
```

### Arch Linux

You can install it manually or use a aur helper like Yay.

```
yay -S sworkstyle
```

## Usage

```
sworkstyle
```

## Sway Configuration

```
exec sworkstyle &> /tmp/sworkstyle.log
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

When an app isn't recogised in the config, `sworkstyle` will log the application name as a warning.
Simply add that string to your config file, with an icon of your choice.

Note that the crate [`find_unicode`](https://github.com/pierrechevalier83/find_unicode/) can help find a unicode character directly from the command line. It now supports all of nerdfonts unicode space.

For a reference to the regex syntax see the [`regex`](https://docs.rs/regex/1.5.4/regex/#syntax) crate

### Matching

**Standard**

```
'{pattern}' = '{icon}'

pattern: Can either be the exact "app_name" (app_id/class) of the window or a regex string in the format of `"/{regex}/"` which will match the window "title".
icon: Your beautifull icon
```

**Verbose**

```
'{pattern}' = { type = 'generic' | 'exact', value = '{icon}' }
```

_**Note:**_ You'll only have to use the verbose format when matching generic with a case insensitive text. `'case insensitive title' = { type = 'generic', value = 'A' }`

### Default Config

```toml
fallback = ''

[matching]
'discord' = ''
'balena-etcher' = ''
'Chia Blockchain' = ''
'Steam' = ''
'vlc' = ''
'org.qbittorrent.qBittorrent' = ''
'Thunderbird' = ''
'thunderbird' = ''
'Postman' = ''
'Insomnia' = ''
'Bitwarden' = ''
'Google-chrome' = ''
'Chromium' = ''
'Slack' = ''
'Code' = ''
'code-oss' = ''
'Spotify' = ''
'/(?i)Github.*Firefox/' = ''
'firefox' = ''
'Nightly' = ''
'firefoxdeveloperedition' = ''
'/nvim ?\w*/' = ''
'/npm/' = ''
'/node/' = ''
'/yarn/' = ''
'Alacritty' = ''
```

## Roadmap

- Allow multiple instances of a program to be displayed with only one icon `unique = true`
- Cmdline arg for specifying config location `-c {path_to_config}`

## Known Issues

- Using sway's alt-tab behavior can cause a workspace to be not named
