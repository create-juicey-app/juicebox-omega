# juicebox-omega

Current implementation of the Juicebox Omega API. Rust-based backend service that provides an API for hosting and serving files.

Source code lives in `src/`, example configuration is provided in `.env.example`, and static/hosted files are placed in the `files/` directory.

## Prerequisites

- Rust toolchain (install via rustup)
- git

## Quick start

1. Clone the repository:
   ```
   git clone https://github.com/create-juicey-app/juicebox-omega.git
   cd juicebox-omega
   ```

2. Copy example environment file and edit values:
   ```
   cp .env.example .env
   # edit .env as needed
   ```

3. Build and run in development:
   ```
   cargo run
   ```

4. Build a release binary:
   ```
   cargo build --release
   ```

5. Run tests:
   ```
   cargo test
   ```

## Environment variables

Edit `.env` (created from `.env.example`) to configure runtime behavior. Common variables you may see in the example include:

- SERVER_ADDR or PORT — network address or port the server listens on.
- STORAGE_DIR or FILES_DIR — path used to store or serve hosted files (defaults to `files/`).
- LOG_LEVEL — log verbosity (for example: `info`, `debug`).

Adjust values to match your deployment environment. Do not commit secret values.

## Files directory

- The `files/` directory is intended to hold content that the API serves or manages. Confirm its path in your `.env`/configuration and ensure appropriate permissions for the environment where the service runs.

## Contributing

- Open issues to report bugs or request features.
- Create pull requests that describe the change, include tests when appropriate, and reference related issues.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
