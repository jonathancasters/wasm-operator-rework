# Contributing to WASM-operator

## Reproducible Development Shell

Setting up a reproducible development environment can be tedious. To simplify
this process, this repository includes a `shell.nix` file that sets up the
required development shell automatically.

### Requirements

The only requirement is that you have the [Nix package
manager](https://nixos.org/download/) installed.

### Usage

Once Nix is installed, simply run the following command inside the cloned
repository directory:

```sh
nix-shell
```

This will launch a shell with all necessary dependencies and configurations
applied.

> [!NOTE]
> If you encounter warnings about experimental features, create a
> configuration file at `~/.config/nix/nix.conf` with this content:
>
> ```
> experimental-features = nix-command flakes
> ```
> 

### Automatic Shell Activation (Optional)

If you want the shell environment to activate automatically when you enter the
directory, you need to install
[`direnv`](https://direnv.net/docs/installation.html) and configure it in your
shell.

You can install `direnv` using the Nix package manager if you prefer:

```sh
nix profile install nixpkgs#direnv
```



#### Steps

1. Ensure that `direnv` is enabled in your shell by adding the appropriate shell
   hook:

   ```sh
   # For Bash
   echo 'eval "$(direnv hook bash)"' >> ~/.bashrc

   # For Zsh
   echo 'eval "$(direnv hook zsh)"' >> ~/.zshrc

   # For Fish
   echo 'eval (direnv hook fish)' >> ~/.config/fish/config.fish
   ```

2. Allow the `.envrc` file inside the repository directory:

   ```sh
   cp ./.envrc.sample > ./.envrc
   direnv allow
   ```

Now, every time you enter the directory, the development shell should be
activated automatically.

### Features

Upon entering the development shell, a menu will be displayed showing all
available commands and tools included in the environment. This could help you in
discovering the tools easily. If you want to promt this menu, you can enter the
`menu` command in the terminal.

## Code quality

This project employs several formatters and linters to ensure code consistency
and maintain high-quality standards.  Contributors are expected to adhere to
these practices and use the tools provided.

| Language | Formatter / Linter | Command |
| -------- | ------------------ | ------- |
| Rust     | [rustfmt](https://github.com/rust-lang/rustfmt) (F) <br> [clippy](https://github.com/rust-lang/rust-clippy) (L)  | `cargo fmt --all` <br> `cargo clippy --all` |
| Go       | [gofmt](https://pkg.go.dev/cmd/gofmt) (F)              | `go fmt` |
| Shell    | [shfmt](https://github.com/mvdan/sh#shfmt) (F) | `shfmt` |
| Python   | [Ruff](https://github.com/astral-sh/ruff) (F+L) | `ruff format` <br> `ruff check` |
| Markdown | [markdownlint](https://github.com/DavidAnson/markdownlint) (L) | `markdownlint '**/*.md'` |

> [!TIP] These can be setup using VSCode as well
>
> - Rust:
>   [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
>   with settings.json: `"rust-analyzer.check.command": "clippy"`
> - Go: [Go](https://marketplace.visualstudio.com/items?itemName=golang.Go)
> - Shell:
>   [Shell-format](https://marketplace.visualstudio.com/items?itemName=foxundermoon.shell-format)
> - Python:
>   [Ruff](https://marketplace.visualstudio.com/items?itemName=charliermarsh.ruff)
> - Markdown:
>   [markdownlint](https://marketplace.visualstudio.com/items?itemName=DavidAnson.vscode-markdownlint)
