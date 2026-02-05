# Contribution

Contribution to this project is welcome. Please follow the code of conduct for
all interactions.

Ways to Contribute
------------------

- File bug reports and feature requests in the issue tracker.
- Fork the repository and submit pull requests to improve the source code or
  documentation.

Development
-----------

### Architecture

- Review the OpenSpec requirements in `documentation/architecture/openspec`.
  Use the OpenSpec workflow when making changes that affect behavior or user
  experience.

### Hot reload (MCP development)

Use `reloaderoo` to run the MCP server with hot-reload during development:

```json
{
  "mcpServers": {
    "nb": {
      "command": "reloaderoo",
      "args": ["--", "cargo", "run", "--release", "--", "--notebook", "myproject"]
    }
  }
}
```

<!-- TODO: Add guidance and standards section once finalized. -->

Artificial Intelligence
-----------------------

- Contributions co-authored by large language models are welcome, provided they
  follow the standards above and are reviewed for correctness.
