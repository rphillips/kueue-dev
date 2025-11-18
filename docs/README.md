# kueue-dev Documentation

This directory contains the kueue-dev user guide in mdBook format.

## Building the Documentation

### Prerequisites

Install mdBook:

```bash
cargo install mdbook
```

### Serve Locally

Start a local server with live-reload:

```bash
mdbook serve
```

Then open http://localhost:3000 in your browser.

### Build Static HTML

Generate static HTML files:

```bash
mdbook build
```

Output will be in `book/` directory.

## Structure

```
docs/
├── src/                      # Markdown source files
│   ├── SUMMARY.md           # Table of contents
│   ├── introduction.md      # Introduction
│   ├── installation.md      # Installation guide
│   ├── configuration.md     # Configuration
│   ├── quick-start.md       # Quick start guide
│   ├── workflows/           # Workflow guides
│   ├── commands/            # Command reference
│   ├── advanced/            # Advanced features
│   ├── troubleshooting/     # Troubleshooting guides
│   ├── faq.md              # FAQ
│   └── contributing.md      # Contributing guide
├── book/                    # Generated output (gitignored)
└── README.md               # This file

book.toml                    # mdBook configuration (in project root)
```

## Contributing

When adding new pages:

1. Create the markdown file in appropriate directory under `src/`
2. Add entry to `src/SUMMARY.md`
3. Test locally with `mdbook serve`
4. Commit both the new file and updated SUMMARY.md

### Markdown Tips

- Use relative links: `[text](./file.md)` or `[text](../other/file.md)`
- Add code blocks with syntax highlighting: ` ```bash ` or ` ```rust `
- Use admonitions sparingly
- Keep line length reasonable for readability

## Publishing

The documentation can be published to GitHub Pages or any static hosting service.

See mdBook's [deployment guide](https://rust-lang.github.io/mdBook/continuous-integration.html) for more information.
