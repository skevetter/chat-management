# chat-management

A Rust CLI and MCP server for managing chat channels, messages, and @mentions with SQLite persistence.

## Installation

```bash
cargo build --release
```

The binary is at `target/release/chat-management`.

## CLI Usage

### Global Flags

| Flag | Description |
|------|-------------|
| `--db <path>` | SQLite database path (see [Database Location](#database-location)) |
| `--json` | Output as JSON |
| `-n, --namespace <ns>` | Scope operations to a namespace |

### Channels

```bash
# Create a channel
chat-management channel create --name general --purpose "Team discussion"

# List channels
chat-management channel list

# Show channel details
chat-management channel show general

# Delete a channel (cascades to messages and mentions)
chat-management channel delete general
```

### Posting Messages

```bash
# Post a message
chat-management post general --sender alice --content "Hello team!"

# Reply to a message
chat-management post general --sender bob --content "Hi!" --reply-to <message-uuid>

# Idempotent post (safe to retry)
chat-management post general --sender alice --content "Deploy done" --idempotency-key "alice:deploy:2026-04-29"
```

### Reading Messages

```bash
# Read latest messages
chat-management read general

# Filter by time
chat-management read general --since 2026-04-29T10:00:00Z

# Filter by sender
chat-management read general --sender alice
```

### Inspect

```bash
# Lightweight metadata (no message loading)
chat-management inspect general
```

### Mentions

```bash
# All mentions of an agent
chat-management mentions --agent bob

# Mentions in a specific channel
chat-management mentions --channel general

# Combined filter
chat-management mentions --agent bob --channel general
```

### MCP Server

```bash
# Start as MCP server (stdio transport)
chat-management serve
```

## MCP Mode

The `serve` command starts a Model Context Protocol server on stdio, exposing all operations as MCP tools: `channel_create`, `channel_list`, `channel_show`, `channel_delete`, `post_message`, `read_messages`, `inspect_channel`, and `list_mentions`.

To configure in Claude Code `settings.json`:

```json
{
  "mcpServers": {
    "chat-management": {
      "command": "/path/to/chat-management",
      "args": ["serve", "--namespace", "my-project"]
    }
  }
}
```

## Namespace Scoping

The `--namespace` / `-n` flag isolates channels into independent scopes. Channel names are unique within a namespace, so different projects can use the same channel names without conflict.

```bash
# Create channels in different namespaces
chat-management -n project-a channel create --name alerts
chat-management -n project-b channel create --name alerts

# Operations are scoped
chat-management -n project-a channel list  # only project-a channels
```

In MCP mode, `serve --namespace <ns>` sets the server-wide default. Individual tool calls can override via the `namespace` parameter.

## Database Location

By default, the database is stored at:

```
$XDG_DATA_HOME/chat-management/chat.db
```

If `XDG_DATA_HOME` is not set, it falls back to `~/.local/share/chat-management/chat.db`.

Override with `--db`:

```bash
chat-management --db /tmp/test.db channel list
```
