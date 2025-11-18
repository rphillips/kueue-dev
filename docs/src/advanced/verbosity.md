# Verbosity Levels

Control output detail with multiple `-v` flags.

## Levels

- **Default** (no flag): Warnings and errors only
- **`-v`**: Info level - standard operational messages
- **`-vv`**: Debug level - detailed debugging information
- **`-vvv`**: Trace level - extremely verbose output

## Examples

```bash
kueue-dev deploy kind --name test                # Minimal output
kueue-dev -v deploy kind --name test             # Info level
kueue-dev -vv deploy kind --name test            # Debug level
kueue-dev -vvv deploy kind --name test           # Trace level
```

## Use Cases

- **Default**: Production use, CI/CD
- **`-v`**: Normal development
- **`-vv`**: Debugging issues
- **`-vvv`**: Troubleshooting deep problems
