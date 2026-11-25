# concurtui

Run several commands in parallel and switch between their output in a
full-screen TUI. A minimal, zero-config alternative to `concurrently`.

Each command runs in its own pseudo-terminal, so colors, progress bars and
interactive programs work as they do in a normal terminal. One command's output
is shown at a time, next to a sidebar listing every command and its status.

## Install

### Quick install (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/protibimbok/concurtui/master/scripts/install.sh | bash
```

Downloads the release binary for your platform, verifies its checksum and
installs it to `/usr/local/bin` (sudo only if that isn't writable). Pass
`--install-dir ~/.local/bin` or `--version v0.1.0` after `bash -s --`.

### Quick install (Windows, PowerShell)

```powershell
irm https://raw.githubusercontent.com/protibimbok/concurtui/master/scripts/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\Programs\concurtui` and adds it to your user PATH.

### npm

```sh
npm i -g concurtui
```

Ships a prebuilt binary for Linux (x64, arm64), macOS (x64, arm64) and
Windows (x64); no Rust toolchain needed.

### Homebrew (macOS / Linux)

```sh
brew tap protibimbok/pkg-dist
brew install --cask concurtui
```

### apt (Debian / Ubuntu)

```sh
# One-time: install the signing key
curl -fsSL https://github.com/protibimbok/pkg-dist/raw/master/public.gpg \
  | sudo gpg --dearmor -o /usr/share/keyrings/protibimbok.gpg

# One-time: add the repository
echo "deb [signed-by=/usr/share/keyrings/protibimbok.gpg] https://protibimbok.github.io/pkg-dist/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/protibimbok.list

sudo apt update
sudo apt install concurtui
```

### AUR (Arch Linux)

```sh
yay -S concurtui-bin
# or
paru -S concurtui-bin
```

### rpm (Fedora / RHEL / openSUSE) and Alpine

Download the `.rpm` or `.apk` from the
[latest release](https://github.com/protibimbok/concurtui/releases/latest), then:

```sh
sudo rpm -i concurtui_*_linux_amd64.rpm
sudo apk add --allow-untrusted concurtui_*_linux_amd64.apk
```

### Cargo

```sh
cargo install concurtui   # builds from source
```

### Manual

Every [release](https://github.com/protibimbok/concurtui/releases) has
`concurtui_<os>_<arch>.tar.gz` (`.zip` on Windows) archives and a
`checksums.txt`. Extract the binary anywhere on your PATH.

## Usage

```sh
concurtui "npm run dev" "npm run api" "cargo watch -x test"
concurtui -n web,api,test "npm run dev" "npm run api" "cargo watch -x test"
```

Each argument is one command, run with `sh -c` (`cmd /C` on Windows).
`-n/--names` sets the sidebar labels; without it, they show the command text.
Labels longer than 20 characters are cut short. `-t/--title` adds a top row
with that text and the working directory.

The sidebar marks each command as starting (a spinner), running (its number),
exited successfully (green `✓`) or failed (red `✕`); the exit code is shown
above the output. Exited commands keep their output on screen until you
restart them or quit.

A scrollbar appears once a command has more output than fits, and the bottom
line lists the keys that apply and tells you when new output arrived while you
were scrolled up. On terminals too small for the sidebar, only the current
command's output is shown, under a row with its name and status, and the
bottom line just points at the help (`h`).

## Keybindings

| Key                             | Action                                       |
| ------------------------------- | -------------------------------------------- |
| `Tab` / `Shift+Tab`, `1`–`9`    | Switch to the next / previous / nth command  |
| Click on a sidebar entry        | Switch to that command                       |
| `r`                             | Restart the current command                  |
| `c`                             | Clear the current command's output           |
| `↑` / `↓`, `k` / `j`            | Scroll one line                              |
| `PgUp` / `PgDn`, mouse wheel    | Scroll one page / three lines                |
| Click / drag the scrollbar      | Jump to that part of the output              |
| `g` / `G`                       | Scroll to the top / bottom                   |
| `/`                             | Search the current command's output          |
| `n` / `N`                       | Next / previous match                        |
| `i`                             | Input mode: send keystrokes to the command   |
| `h`                             | Show / hide the list of keys                 |
| `q`, `Ctrl+C`                   | Quit                                         |

Search ignores case and marks matches as you type. `Enter` keeps the search
and jumps to the most recent match; `Esc` cancels it, or clears it afterwards.

In input mode every key goes to the current command, including `Ctrl+C`.
`Esc` returns to normal mode. To send `Esc` itself to the command, press
`Ctrl+A` then `Esc` (`Ctrl+A` twice sends `Ctrl+A`).

On quit, every command's process group gets `SIGTERM` and has 3 seconds to
exit before being killed with `SIGKILL`. On Windows the process tree is
killed.

## Releasing

Bump `version` in `Cargo.toml`, commit, then push a matching tag:

```sh
git tag v0.2.0 && git push origin v0.2.0
```

The release workflow builds every target and publishes the GitHub release
(archives, `.deb`/`.rpm`/`.apk`, checksums), npm, crates.io, the AUR package,
and triggers [pkg-dist](https://github.com/protibimbok/pkg-dist) for Homebrew
and apt. npm and crates.io use trusted publishing (no tokens); the AUR and
pkg-dist steps need the `AUR_KEY` and `PKG_DIST_TOKEN` repository secrets.
