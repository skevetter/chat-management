# Communication Management Tool — Design Document

A Rust CLI + MCP server for managing chat channels, messages, and @mentions across namespaced communication spaces.

## 1. SQLite Schema

### Pragmas

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
```

### channels

```sql
CREATE TABLE IF NOT EXISTS channels (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT    NOT NULL,
    namespace     TEXT    NOT NULL DEFAULT 'default',
    purpose       TEXT,
    created_at    TEXT    NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE (name, namespace)
);
```

### messages

```sql
CREATE TABLE IF NOT EXISTS messages (
    id              TEXT PRIMARY KEY,  -- UUID v4
    channel_id      INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender          TEXT    NOT NULL,
    content         TEXT    NOT NULL,
    timestamp       TEXT    NOT NULL,
    reply_to        TEXT,              -- nullable, references messages.id
    idempotency_key TEXT               -- nullable, UNIQUE
);

CREATE INDEX IF NOT EXISTS idx_messages_channel_ts ON messages (channel_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_sender     ON messages (sender);
CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_idempotency
    ON messages (idempotency_key) WHERE idempotency_key IS NOT NULL;
```

### mentions

```sql
CREATE TABLE IF NOT EXISTS mentions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id      TEXT    NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    channel_id      INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    mentioned_agent TEXT    NOT NULL,
    created_at      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mentions_agent_channel ON mentions (mentioned_agent, channel_id);
```

### schema_versions

```sql
CREATE TABLE IF NOT EXISTS schema_versions (
    version    INTEGER PRIMARY KEY,
    applied_at TEXT    NOT NULL
);
```

### message_count trigger

The `message_count` column on `channels` is denormalized. It is maintained by triggers rather than computed on read:

```sql
CREATE TRIGGER IF NOT EXISTS trg_inc_message_count
AFTER INSERT ON messages
BEGIN
    UPDATE channels SET message_count = message_count + 1 WHERE id = NEW.channel_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_dec_message_count
AFTER DELETE ON messages
BEGIN
    UPDATE channels SET message_count = message_count - 1 WHERE id = OLD.channel_id;
END;
```

## 2. CLI Interface

Uses clap 4 derive pattern (matching `task-management/src/main.rs`).

### Global flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--db <path>` | `String` | `$XDG_DATA_HOME/chat-management/chat.db` | SQLite database path |
| `--json` | `bool` | `false` | Output as JSON |
| `--namespace <ns>` | `String` | — | Scope all operations to this namespace |

### Commands

#### `channel create`

Create a new channel.

```
chat-management channel create --name <name> [--purpose <text>]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--name` | yes | Channel name (unique within namespace) |
| `--purpose` | no | Human-readable purpose |

#### `channel list`

List channels in the current namespace.

```
chat-management channel list [--limit N] [--offset N]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--limit` | 50 | Max results |
| `--offset` | 0 | Pagination offset |

#### `channel show`

Show full details for a channel.

```
chat-management channel show <name-or-id>
```

Positional argument: channel name or integer ID.

#### `channel delete`

Delete a channel and cascade-delete all its messages and mentions.

```
chat-management channel delete <name-or-id>
```

#### `post`

Post a message to a channel. Parses @mentions from content automatically.

```
chat-management post <channel> --sender <id> --content <text> [--reply-to <msg-id>] [--idempotency-key <key>]
```

| Flag | Required | Description |
|------|----------|-------------|
| `--sender` | yes | Sender agent/user ID |
| `--content` | yes | Message body (supports @agent-id mentions) |
| `--reply-to` | no | UUID of parent message for threading |
| `--idempotency-key` | no | Dedup key — duplicate posts return existing message |

#### `read`

Read messages from a channel with optional filters.

```
chat-management read <channel> [--limit N] [--offset N] [--since <timestamp>] [--sender <id>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--limit` | 50 | Max messages |
| `--offset` | 0 | Pagination offset |
| `--since` | — | ISO 8601 timestamp — return messages after this time |
| `--sender` | — | Filter by sender ID |

#### `inspect`

Lightweight metadata query — returns channel info without loading messages.

```
chat-management inspect <channel>
```

Returns: name, namespace, purpose, message_count, created_at. No message data.

#### `mentions`

Query @mentions across channels.

```
chat-management mentions [--agent <id>] [--channel <name-or-id>] [--limit N] [--offset N]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--agent` | — | Filter by mentioned agent ID |
| `--channel` | — | Filter by channel |
| `--limit` | 50 | Max results |
| `--offset` | 0 | Pagination offset |

#### `serve`

Start as an MCP server (stdio transport).

```
chat-management serve --transport stdio [--namespace <ns>]
```

The `--namespace` flag sets the default namespace for all MCP tool calls (can be overridden per-call).

## 3. MCP Tool Definitions

All tools follow the `rmcp` pattern from `task-management/src/mcp/`. Parameter structs derive `Deserialize` + `JsonSchema`. Each tool returns `Result<CallToolResult, ErrorData>` with JSON-serialized content.

### channel_create

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChannelCreateParams {
    pub name: String,
    pub purpose: Option<String>,
    pub namespace: Option<String>,
}
```

Returns: full channel object (id, name, namespace, purpose, created_at, message_count).

### channel_list

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChannelListParams {
    pub namespace: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
```

Returns: `{ channels: [...], total, limit, offset }`.

### channel_show

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChannelShowParams {
    pub channel: String,  // name or ID
    pub namespace: Option<String>,
}
```

Returns: full channel object with message_count.

### channel_delete

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChannelDeleteParams {
    pub channel: String,  // name or ID
    pub namespace: Option<String>,
}
```

Returns: `{ deleted: true, channel_id: N }`.

### post_message

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PostMessageParams {
    pub channel: String,         // name or ID
    pub sender: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub idempotency_key: Option<String>,
    pub namespace: Option<String>,
}
```

On insert: parses @mentions from `content`, writes to `mentions` table, increments `message_count` (via trigger). If `idempotency_key` matches an existing message, returns the existing message without error.

Returns: message object (id, channel_id, sender, content, timestamp, reply_to).

### read_messages

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadMessagesParams {
    pub channel: String,          // name or ID
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub since: Option<String>,    // ISO 8601 timestamp
    pub sender: Option<String>,
    pub namespace: Option<String>,
}
```

Returns: `{ messages: [...], total, limit, offset }`.

### inspect_channel

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct InspectChannelParams {
    pub channel: String,  // name or ID
    pub namespace: Option<String>,
}
```

Lightweight metadata-only query. Never touches the messages table.

Returns: `{ id, name, namespace, purpose, message_count, created_at }`.

### list_mentions

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListMentionsParams {
    pub agent: Option<String>,
    pub channel: Option<String>,  // name or ID
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub namespace: Option<String>,
}
```

Returns: `{ mentions: [...], total, limit, offset }`. Each mention includes: id, message_id, channel_id, mentioned_agent, created_at.

## 4. Tiered Reliability Model

Operations are split into two tiers based on weight and latency expectations.

### Metadata path (fast, lightweight)

- **inspect_channel** / `inspect` CLI — reads only the `channels` row; never joins or scans `messages`.
- **channel_list** / `channel list` CLI — reads `channels` table with pagination.
- **channel_show** / `channel show` CLI — reads single `channels` row.
- **channel_create**, **channel_delete** — single-row insert/delete on `channels`.

These operations must always be fast. They are never blocked by heavy message queries because they do not touch the `messages` table.

### Message read/write path (heavier, paginated)

- **post_message** / `post` CLI — inserts into `messages` + `mentions`, fires `message_count` trigger.
- **read_messages** / `read` CLI — queries `messages` with filters and pagination.
- **list_mentions** / `mentions` CLI — queries `mentions` table with optional joins.

These operations use pagination (`limit`/`offset`) to bound result size. The `(channel_id, timestamp)` index on `messages` and `(mentioned_agent, channel_id)` index on `mentions` keep read queries efficient.

### Denormalized message_count

The `channels.message_count` column is maintained by SQLite triggers on `messages` INSERT and DELETE. This avoids `SELECT COUNT(*) FROM messages WHERE channel_id = ?` on every inspect/list call, keeping the metadata path O(1) regardless of message volume.

## 5. Namespace Scoping Model

Namespaces isolate channels (and their messages) into independent scopes.

- Every channel belongs to exactly one namespace. Default: `"default"`.
- The `UNIQUE(name, namespace)` constraint on `channels` allows the same channel name in different namespaces.
- Namespace is stored on the **channel**, not on individual messages. A message inherits its namespace from its channel.
- The `--namespace <ns>` global CLI flag filters all operations to the specified namespace:
  - `channel create` sets the channel's namespace.
  - `channel list`, `channel show`, `inspect`, `read`, `post`, `mentions` all scope queries to the namespace.
- In MCP mode, the `serve --namespace <ns>` flag sets a server-wide default. Individual tool calls can override via the `namespace` parameter.
- When no namespace is specified (CLI or MCP), operations that list/query span all namespaces. Channel resolution by name requires a namespace to be unambiguous (or falls back to `"default"`).

## 6. Idempotency Model

Message posting supports optional idempotency to handle agent retries.

- The `idempotency_key` field on `messages` is nullable. When `NULL`, no dedup occurs.
- A partial unique index (`WHERE idempotency_key IS NOT NULL`) enforces uniqueness only among non-null keys.
- On `post_message` / `post` with an `--idempotency-key`:
  1. Attempt INSERT.
  2. If the key already exists (UNIQUE constraint violation), query and return the existing message.
  3. No error is raised — the caller receives the original message as if it were just posted.
- This allows agents to safely retry failed posts without creating duplicate messages. The agent generates a deterministic key (e.g., `"{agent_id}:{channel}:{timestamp}"`) and reuses it on retry.

## 7. @Mention Parsing

@mentions are extracted from message content at post time and stored in the `mentions` table.

### Parsing

- On every `post_message` / `post`, scan the `content` field for `@<agent-id>` patterns.
- Pattern: `@([a-zA-Z0-9_-]+)` — captures one or more alphanumeric, hyphen, or underscore characters after `@`.
- Each unique match produces one row in the `mentions` table with the `message_id`, `channel_id`, and `mentioned_agent`.
- Duplicate mentions within the same message are deduplicated (only one row per agent per message).

### Querying

- `list_mentions` / `mentions` CLI supports filtering by:
  - `--agent <id>` — all mentions of a specific agent across channels.
  - `--channel <name-or-id>` — all mentions within a channel.
  - Both combined — mentions of a specific agent in a specific channel.
- The `(mentioned_agent, channel_id)` index makes these queries efficient.
- Results are paginated with `limit`/`offset` and ordered by `created_at DESC`.

## 8. Dependencies

Matches `task-management/Cargo.toml` dependency set:

```toml
[package]
name = "chat-management"
version = "0.1.0"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
rmcp = { version = "1.5", features = ["server", "transport-io"] }
tokio = { version = "1", features = ["rt", "macros", "io-std"] }
schemars = "1"
regex = "1"

[dev-dependencies]
assert_cmd = "2"
tempfile = "3"
predicates = "3"
```

`regex` is added for @mention parsing. All other dependencies mirror `task-management`.
