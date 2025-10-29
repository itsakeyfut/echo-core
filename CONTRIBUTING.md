# Contributing to echo-core

Thank you for your interest in contributing to echo-core! This document provides guidelines and instructions for contributing to the project.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Set up your development environment** following the instructions in [docs/05-development/setup.md](docs/05-development/setup.md)

## Development Workflow

### Before You Start

1. Check existing [issues](https://github.com/itsakeyfut/echo-core/issues) to see if your feature/bug is already being worked on
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
   - Ensure all tests pass: `cargo test`

4. **Run checks**:
   ```bash
   # Format code
   cargo fmt

   # Run clippy
   cargo clippy --all-targets --all-features -- -D warnings

   # Run tests
   cargo test

   # Build
   cargo build --release
   ```

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
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code is formatted (`cargo fmt`)
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

By contributing to echo-core, you agree that your contributions will be licensed under the Apache License 2.0.

## Questions?

Feel free to open an issue with the `question` label if you have any questions about contributing!

Thank you for contributing to echo-core! 🎮
