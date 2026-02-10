# Contributing to Wave Terminal

We welcome and value contributions to Wave Terminal! Wave is an open source project, always open for contributors. There are several ways you can contribute:

- Submit issues related to bugs or new feature requests
- Fix outstanding [issues](https://github.com/wavetermdev/waveterm/issues) with the existing code
- Contribute to [documentation](./docs)
- Spread the word on social media (tag us on [LinkedIn](https://www.linkedin.com/company/wavetermdev), [Twitter/X](https://x.com/wavetermdev))
- Or simply ⭐️ the repository to show your appreciation

However you choose to contribute, please be mindful and respect our [code of conduct](./CODE_OF_CONDUCT.md).

> All contributions are highly appreciated! 🥰

## Before You Start

We accept patches in the form of github pull requests. If you are new to github, please review this [github pull request guide](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-pull-requests).

### Contributor License Agreement

Contributions to this project must be accompanied by a Contributor License Agreement (CLA). You (or your employer) retain the copyright to your contribution, this simply gives us permission to use and redistribute your contributions as part of the project.

> On submission of your first pull request you will be prompted to sign the CLA confirming your original code contribution and that you own the intellectual property.

### Style guide

The project uses American English.

We have a set of recommended Visual Studio Code extensions to enforce our style and quality standards. Please ensure you use these, especially [Prettier](https://prettier.io) and [EditorConfig](https://editorconfig.org), when contributing to our code.

## How to contribute

- For minor changes, you are welcome to [open a pull request](https://github.com/wavetermdev/waveterm/pulls).
- For major changes, please [create an issue](https://github.com/wavetermdev/waveterm/issues/new) first.
- If you are looking for a place to start take a look at [Good First Issues](https://github.com/wavetermdev/waveterm/issues?q=is:issue%20state:open%20label:%22good%20first%20issue%22).
- Join the [Discord channel](https://discord.gg/XfvZ334gwU) to collaborate with the community on your contribution.

### Development Environment

To build and run Wave locally, see instructions at [Building Wave Terminal](./BUILD.md).

### UI Component Library

We are working to document all our UI components in [Storybook](https://storybook.js.org/docs) for easy reference and testing. If you would like to help us with this, we would be very grateful!

Our Storybook site is hosted [docs.waveterm.dev/storybook](https://docs.waveterm.dev/storybook).

### Create a Pull Request

Guidelines:

- Before writing any code, please look through existing PRs or issues to make sure nobody is already working on the same thing.
- Develop features on a branch - do not work on the main branch
- For anything but minor fixes, please submit tests and documentation
- Please reference the issue in the pull request

## Project Structure

The project is broken into three main components: frontend (Tauri + React with in-process Rust backend) and wsh (Go shell integration CLI). This section is a work-in-progress as our codebase is constantly changing.

### Frontend

Our frontend can be found in the [`/frontend`](./frontend/) directory. It is written in React TypeScript. The main entrypoint is [`wave.ts`](./frontend/wave.ts) and the root for the React VDOM is [`app.tsx`](./frontend/app/app.tsx). If you are using `task dev` to run your dev instance of the app, the frontend will be loaded using Vite, which allows for Hot Module Reloading. This should work for most styling and simple component changes, but anything that affects the state of the app (the Jotai or layout code, for instance) may put the frontend into a bad state. If this happens, you can force reload the frontend using `Cmd:Shift:R` or `Ctrl:Shift:R`.

### Tauri + Rust Backend

The Tauri native shell and Rust backend can be found at [`/src-tauri`](./src-tauri/). It handles native window management, system tray, menus, crash handling, logging, and all backend services (SQLite database, terminal PTY, pub/sub, RPC, config management). The main entrypoint is [`lib.rs`](./src-tauri/src/lib.rs). IPC commands are registered in the `invoke_handler` and defined in [`src/commands/`](./src-tauri/src/commands/). Backend services are in [`src/backend/`](./src-tauri/src/backend/). Changes to Rust code are auto-rebuilt by `task dev`.

### wsh

wsh can be found at [`/cmd/wsh`](./cmd/wsh/). It serves two purposes: it functions as a CLI tool for controlling the app from the command line and it functions as a server on remote machines to facilitate multiplexing terminal sessions over a single connection and streaming files between the remote host and the local host. This process does not hot-reload — run `task build:backend` to rebuild.

Communication between the Rust backend and wsh is handled by wshrpc via local domain socket IPC or WebSocket, depending on what the remote host supports.
