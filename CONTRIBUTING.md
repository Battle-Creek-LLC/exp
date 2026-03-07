# Contributing

Thanks for your interest in contributing to `exp`!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/exp.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Submit a pull request

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run the CLI locally
cargo run -- create "test-experiment"
```

## Guidelines

- Keep changes focused — one feature or fix per PR
- Add tests for new functionality
- Run `cargo clippy` and `cargo fmt` before submitting
- Follow existing code style and patterns

## Reporting Issues

Please open an issue on GitHub with:
- A description of the problem or feature request
- Steps to reproduce (for bugs)
- Expected vs actual behavior
