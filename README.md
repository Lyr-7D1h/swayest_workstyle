# Swayest Workstyle

![AUR version](https://img.shields.io/aur/version/sworkstyle)

Map workspace name to icons defined depending on the windows inside of the workspace.

An executable similar to [workstyle](https://github.com/pierrechevalier83/workstyle).

**Differences between `sworkstyle` and `workstyle`:**

- Plug-and-play solution, build-in matching config, you can extend this config by creating/modifying `.config/sworkstyle/config.toml` or you can make a PR for your package manager or this repository with new matchers.

- Way better matching: using regex, exact app names and generic app titles.

- Specifically meant for Sway and Wayland

- Fallback Icon

Your workspace shall never contain an empty icon again!

**An example of what it does (using waybar which also hides the workspace index):**

<img src="./screenshots/bar.png">
<br />
<img src="./screenshots/desktop.png" width="1000">

## Installation

### Cargo

```bash
cargo install sworkstyle
```

### Arch Linux

You can install it manually or use a aur helper like Yay.

```bash
yay -S sworkstyle
```

## Usage

```bash
sworkstyle
```

## Sway Configuration
```bash
exec sworkstyle &> /tmp/sworkstyle.log
```
> **_NOTE:_** When using the cargo install make sure to add the `.cargo/bin` to the `PATH` environment variable before executing sway. You can do this by adding it to `.zprofile` or `.profile`

Note that since your workspaces will be renamed all the time, you should configure your keybindings to use numbered workspaces instead of assuming that the name is the number:
Prefer

```bash
    bindsym $mod+1 workspace number 1
```

over

```bash
    bindsym $mod+1 workspace 1
```

## Sworkstyle Configuration

The main configuration consists of deciding which icons to use for which applications.

The config file is located at `${XDG_CONFIG_HOME}/sworkstyle/config.toml`. Its values will take precedence over the defaults. The syntax is in TOML and should be pretty self-explanatory.

When an app isn't recognized in the config, `sworkstyle` will log the application name as a warning.
Simply add that string to your config file, with an icon of your choice.

Note that the crate [find_unicode](https://github.com/pierrechevalier83/find_unicode/) can help find a unicode character directly from the command line. It now supports all of nerdfonts unicode space.

For a reference to the regex syntax see the [regex](https://docs.rs/regex/1.5.4/regex/#syntax) crate

### Matching

#### Standard

```toml
'{pattern}' = '{icon}'

# pattern: Can either be the exact "app_name" (app_id/class) of the window or a regex string in the format of `"/{regex}/"` which will match the window "title".
# icon: Your beautiful icon
```

#### Verbose

```toml
'{pattern}' = { type = 'generic' | 'exact', value = '{icon}' }
```

_**Note:**_ You'll only have to use the verbose format when matching generic with a case insensitive text. `'case insensitive title' = { type = 'generic', value = 'A' }`

#### Troubleshooting

If it couldn't match something it will print:

WARN [sworkstyle:config] No match for '{app_name}' with title '{title}'

You can use {title} to do a generic matching

You can use {app_name} to do an exact match

If a workspace name is reversed, you can force left to right formatting

```toml
'force_ltr' = true
```
The default value of `force_ltr` is `false`

### Default Config

```toml
fallback = ''

force_ltr = false

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

## Package Maintainers

If you want to change the build-in config, change `default_config.toml` with your config and install the project.

You might also want [font-awesome](https://fontawesome.com/) as a dependency depending on your config.

You can also make a PR to add a badge and add your install method under #Installation or to add matchers to the build-in config.

See [aur](https://aur.archlinux.org/cgit/aur.git/tree/PKGBUILD?h=sworkstyle) for an example build.

## Roadmap

- Allow multiple instances of a program to be displayed with only one icon `unique = true`

## Known Issues

- Using sway's alt-tab behavior can cause a workspace to be not named
