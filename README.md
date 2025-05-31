# sif

A file browser for precise code selection with repomix and yek backends.

> [!Important]
> sif is still in development and has many known bugs.

## Why sif?

Find it a struggle to select the right files with complex glob patterns or config files just to select the right files for llm code ingestion? Sif solves this with a simple visual interface where **you see exactly what gets processed**.

Instead of this mess:

```bash
repomix --include "**/*.py" --exclude "**/tests/**" --exclude "**/venv/**" --remove-comments --output-format=markdown .
```

Just do this:

```bash
sif
```

## Features

- Visual file selection, if you select it, it gets included
- Never shows `.gitignore` or `.git` in sif file tree
- Zero config, downloads tools needed automatically
- Dual backend support, repomix and yek for features and speed
- Efficient file tree navigation for large codebases
- Automatically counts tokens for each file and directories
- Copies to your clipboard, for easy copy and paste your favorite llms

## Installation

```bash
cargo install sif
```

> [!NOTE]
> sif requires Node.js to be installed for repomix backend.

macOS:

```bash
brew install node
```

Linux:

```bash
sudo apt install nodejs npm
```

Windows:

```bash
winget install -e --id OpenJS.NodeJS.LTS
```

> [!NOTE]
> For windows, can also download node from [here](https://nodejs.org/)

## Navigation

| Key            | Action                      |
| -------------- | --------------------------- |
| `↑/↓` or `j/k` | Navigate files              |
| `←/→` or `h/l` | Collapse/expand directories |
| `Space`        | Toggle selection            |
| `E`            | Expand all                  |
| `C`            | Collapse all                |
| `A`            | Select all                  |
| `U`            | Unselect all                |
| `r`            | Run processing backend      |
| `q`            | Quit                        |

> [!NOTE]
> sif does have mouse support for file selection, collapse/expand directories, and scrolling.

## Backends

### Repomix (default)

- Output formats: plain text, markdown, xml
- Supports remove comments and compression
- Slower processing than yek

### Yek

- No configuration necessary
- Very fast processing

## TODO

- [ ] Fix known bugs
  - [ ] Token counting is can be slow on large repos
  - [ ] Token counting is not accurate (race condition)
  - [ ] Error messages (stacked errors and not descriptive)
- [ ] Refactor codebase (file structure, design with token counting and processing, naming)
- [ ] Fix general speed issues in large (> 1 M token count codebases)

### Features

- [ ] Add multiple file and directory selection
- [ ] Tab switching to different backends
- [ ] User config files for repomix and yek
- [ ] Rewrite processing core in Rust with same feature support

## Contributing

Open to all feature requests and bug reports. Please submit any changes as a detailed PR or propose a new issue.

## Cache issues

To completely reset sif's cache (including repomix installations and file metadata), run these commands for your operating system.

macOS:

```bash
rm -rf ~/Library/Caches/sif/
```

Linux:

```bash
rm -rf ~/.cache/sif/
rm -rf ~/.local/share/sif/
```

Windows:

```bash
rmdir /s %LOCALAPPDATA%\sif\
```

## Inspiration

- [yek](https://github.com/bodo-run/yek) - Fast Rust-based file serialization tool
- [repomix](https://github.com/yamadashy/repomix) - Feature dense file seralization tool
- [codeselect](https://github.com/maynetee/codeselect) - File selection tool to share with llms
