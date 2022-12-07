# Using `cargo auditable` as a drop-in replacement for `cargo`

**Note:** This document describes Unix-like systems, but similar approaches can be applied to Windows as well. Pull requests adding recipes for Windows are welcome.

The recommended way is to use a shell alias:
```bash
alias cargo="cargo auditable"
```
When entered into the shell, it will only persist for the duration of the session. To make the change permanent, add it to your shell's configuration file (`.bashrc` for bash, `.zshrc` for zsh, `.config/fish/config.fish` for fish).

## When `alias` is not an option

In some cases using shell aliases is not an option, e.g. in certain restricted build environments. In this case you can use a different approach:

1. Run `which cargo` to locate the Cargo binary
2. Copy the snippet provided below and replace '/path/to/cargo' with the path you got at step 1
3. Save it to a file named `cargo`
4. Run `chmod +x cargo` to make the script executable
5. Prepend the path to the directory where you saved the script to your `PATH` environment variable. For example, if you saved the script as `$HOME/.bin/cargo`, you need to add `$HOME/.bin/` to your `PATH`. The exact way to do this varies depending on the shell; in bash it's `export PATH="$HOME/.bin/:$PATH"`

```bash
#!/bin/sh
export CARGO='/path/to/real/cargo' # replace this with your path
cargo-auditable auditable "$@"
```