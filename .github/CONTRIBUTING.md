# Contributing to PSRX

Thank you for your interest in contributing to PSRX! This document provides guidelines and instructions for contributing to the project.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Set up your development environment** following the instructions in [docs/05-development/setup.md](docs/05-development/setup.md)

## Development Workflow

### Quick Start

```bash
# Run all CI checks locally
cargo x ci

# Quick checks before commit
cargo x check

# Auto-format code
cargo x fmt

# Fix clippy warnings
cargo x clippy --fix
```

### Available Commands

We use `x` for development automation. All commands available:

- **`cargo x ci`** - Run full CI pipeline (fmt, clippy, build, test)
- **`cargo x check`** - Quick checks (fmt, clippy)
- **`cargo x fmt [--check]`** - Format code
- **`cargo x clippy [--fix]`** - Run clippy
- **`cargo x build [--release]`** - Build project
- **`cargo x test [--doc] [--ignored]`** - Run tests
- **`cargo x bench`** - Run benchmarks
- **`cargo x bios-boot [--release] [-n <instructions>] [<bios_path>]`** - Run BIOS boot test
- **`cargo x pre-commit`** - Pre-commit checks
- **`cargo x install-hooks`** - Install git hooks

### Before You Start

1. Check existing [issues](https://github.com/itsakeyfut/psrx/issues) to see if your feature/bug is already being worked on
2. Create a new issue if one doesn't exist
3. Comment on the issue to let others know you're working on it

### Making Changes

1. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/bug-description
   ```

2. **Follow coding standards**:
   - Read [docs/02-implementation/coding-standards.md](docs/02-implementation/coding-standards.md)
   - All public APIs must have documentation comments
   - Use `thiserror` for error handling
   - Follow Rust naming conventions

3. **Write tests**:
   - Add unit tests for new functionality
   - Update integration tests if needed
   - Ensure all tests pass: `cargo x test`

4. **Run checks before committing**:
   ```bash
   # Option 1: Run full CI pipeline
   cargo x ci

   # Option 2: Run quick checks
   cargo x check

   # Option 3: Auto-format and fix issues
   cargo x fmt
   cargo x clippy --fix
   cargo x test
   ```

### Git Hooks

Install pre-commit hooks to automatically run checks:

```bash
cargo x install-hooks
```

This will automatically run fmt, clippy, and test before each commit.

### Commit Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code formatting (no functional changes)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tools

**Examples:**
```bash
git commit -m "feat(cpu): implement ADD instruction"
git commit -m "fix(memory): correct address translation for scratchpad"
git commit -m "docs: update CPU architecture documentation"
```

### Pull Request Process

1. **Update your branch** with latest `main`:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

2. **Push your changes**:
   ```bash
   git push origin feat/your-feature-name
   ```

3. **Create a Pull Request** on GitHub:
   - Use a clear, descriptive title
   - Reference related issues (e.g., "Closes #123")
   - Describe what changes you made and why
   - Include any relevant screenshots or benchmarks

4. **Address review feedback**:
   - Respond to comments
   - Make requested changes
   - Push updates to your branch

## Code Review Checklist

Before submitting a PR, ensure:

- [ ] Code follows the coding standards
- [ ] All CI checks pass (`cargo x ci`)
- [ ] All tests pass (`cargo x test`)
- [ ] No clippy warnings (`cargo x clippy`)
- [ ] Code is formatted (`cargo x fmt`)
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] PR description is clear and complete

## Testing Guidelines

### Unit Tests

- Place tests in a `tests` module within the same file
- Test edge cases and error conditions
- Use descriptive test names

### Integration Tests

- Add integration tests in `tests/` directory
- Test component interactions
- Test realistic use cases

### BIOS Boot Testing

Test your changes with actual BIOS execution:

```bash
# Quick test with default settings (100k instructions)
cargo x bios-boot --release

# Extended test with more instructions
cargo x bios-boot -n 200000 --release

# Test with specific BIOS version
cargo x bios-boot path/to/SCPH5501.BIN --release

# Run emulator directly with custom parameters
cargo run --release -- SCPH1001.BIN -n 50000
```

**Use BIOS testing for:**
- Verifying CPU instruction implementations
- Testing memory subsystem changes
- Validating system initialization
- Performance profiling specific code paths

### Benchmarks

- Add benchmarks for performance-critical code
- Place benchmarks in `benches/` directory
- Use criterion for benchmarking

## Documentation

### Code Documentation

- All `pub` items must have doc comments
- Include examples in doc comments where appropriate
- Explain complex algorithms or non-obvious behavior

### Project Documentation

- Update relevant documentation in `docs/` when making architectural changes
- Keep README.md up to date
- Add migration guides for breaking changes

## Getting Help

- Check the [docs/](docs/) directory for design documents
- Read existing code and tests for examples
- Ask questions in GitHub issues or discussions
- Join our community channels (TBD)

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the code, not the person
- Help create a welcoming environment for all contributors

## License

By contributing to PSRX, you agree that your contributions will be licensed under the Apache License 2.0.

## Questions?

Feel free to open an issue with the `question` label if you have any questions about contributing!

Thank you for contributing to PSRX! 🎮
