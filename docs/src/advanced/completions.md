# Shell Completions

Set up command completion for faster typing.

## Supported Shells

- Bash
- Zsh
- Fish
- PowerShell
- Elvish

## Installation

See [Installation - Shell Completion](../installation.md#shell-completion) for detailed instructions.

## Usage

Once installed, press `Tab` to:

- Complete command names
- Complete subcommands
- Complete flag names
- Complete flag values (where applicable)

## Example

```bash
kueue-dev clu<Tab>       # Completes to "cluster"
kueue-dev cluster cr<Tab> # Completes to "create"
kueue-dev cluster create --cn<Tab>  # Completes to "--cni"
```
