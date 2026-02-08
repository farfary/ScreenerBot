# Contributing to ScreenerBot

Thank you for your interest in contributing to ScreenerBot! This document provides guidelines and information for contributors.

## Ways to Contribute

- **Bug Reports**: Open an issue describing the bug, steps to reproduce, and expected behavior
- **Feature Requests**: Open an issue describing the feature and its use case
- **Code Contributions**: Submit a pull request with your changes
- **Documentation**: Improve README, code comments, or add guides
- **DEX Decoders**: Add support for new Solana DEXs

## Development Setup

### Prerequisites

- Rust 1.75+ (`rustup update stable`)
- Node.js 18+ (for frontend tools)
- Platform-specific build tools (see README)

### Building

```bash
# Clone the repository
git clone https://github.com/screenerbotio/ScreenerBot.git
cd ScreenerBot

# Build (debug mode)
cargo build

# Build (release mode)
cargo build --release

# Run checks
cargo check --lib
cargo clippy
```

## Pull Request Process

1. **Fork** the repository
2. **Create a branch** for your feature (`git checkout -b feature/my-feature`)
3. **Make your changes** following the code style below
4. **Test** your changes locally
5. **Commit** with clear messages (see commit guidelines)
6. **Push** to your fork
7. **Open a PR** against `main`

## Code Style

- Follow existing patterns in the codebase
- Use `rustfmt` for formatting (`cargo fmt`)
- Run `cargo clippy` and address warnings
- Keep functions focused and reasonably sized
- Add comments for complex logic

### Naming Conventions

- **Files**: `snake_case.rs`
- **Modules**: `snake_case`
- **Types**: `PascalCase`
- **Functions**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`

## Commit Messages

Use clear, descriptive commit messages:

```
type: short description

Longer explanation if needed.
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `test`: Test additions/changes
- `chore`: Maintenance tasks

## Areas for Contribution

### High Priority

- **DEX Decoders**: Add support for new Solana DEXs
- **Strategy Conditions**: New trading strategy conditions
- **Documentation**: Guides, tutorials, API docs

### Medium Priority

- **Dashboard**: UI improvements, new visualizations
- **Tests**: Unit tests, integration tests
- **Performance**: Optimization opportunities

### Good First Issues

Look for issues labeled `good first issue` - these are suitable for newcomers.

## Architecture Notes

### Services

The bot is organized into independent services managed by `ServiceManager`:

- Each service implements the `Service` trait
- Services declare their dependencies
- ServiceManager handles startup order and health monitoring

### Adding a New DEX Decoder

1. Add decoder in `src/pools/decoders/`
2. Implement `PoolDecoder` trait
3. Register in `src/pools/analyzer.rs`
4. Add program ID to pool discovery

### Adding a Strategy Condition

1. Add condition type in `src/strategies/conditions/`
2. Implement `Condition` trait
3. Register in condition factory
4. Add UI support in webserver

## Questions?

- Open a [GitHub Discussion](https://github.com/screenerbotio/ScreenerBot/discussions)
- Join our [Telegram](https://t.me/screenerbotio)
- Check existing issues and PRs

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
