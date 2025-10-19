# ChadReview

High-performance GitHub PR review tool with real-time comment sync.

**⚠️ This project is under active development. See [spec/plan.md](spec/plan.md) for current status.**

## Why?

GitHub's interface doesn't auto-update comments and struggles with large PRs. ChadReview fixes this with real-time SSE updates and efficient server-side rendering.

## Planned Features

- Real-time comment synchronization (all types)
- Fast diff viewing with syntax highlighting
- Clean, focused UI
- Desktop and web support

## Documentation

See the `spec/` directory:

- [architecture.md](spec/architecture.md) - System design
- [plan.md](spec/plan.md) - Implementation plan and progress

## Built With

[HyperChad](https://github.com/MoosicBox/MoosicBox/tree/master/packages/hyperchad) from [MoosicBox](https://github.com/MoosicBox/MoosicBox)

## License

MPL-2.0
