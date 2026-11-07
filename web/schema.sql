-- D1 schema for the Great Conversation.
--   npx wrangler d1 execute great-conversation --remote --file=schema.sql

-- The curator's edits to replies (an entry's interpretation) and contexts.
CREATE TABLE IF NOT EXISTS edits (
  entry_id   TEXT NOT NULL,                                -- "entry:<id>"
  field      TEXT NOT NULL CHECK (field IN ('reply', 'context')),
  text       TEXT NOT NULL,
  updated_at TEXT NOT NULL,                                -- ISO 8601
  updated_by TEXT NOT NULL,                                -- the curator's email
  PRIMARY KEY (entry_id, field)
);

-- Readers who have signed in, and the display name each chose.
CREATE TABLE IF NOT EXISTS users (
  email        TEXT PRIMARY KEY,                           -- verified by Cloudflare Access
  display_name TEXT,
  display_key  TEXT UNIQUE,                                -- lower-cased name, for uniqueness
  updated_at   TEXT NOT NULL
);

-- Readers' messages (in a channel, or a reply in an entry's thread), once the conversation
-- reaches the present. Only the author may edit or delete their own.
CREATE TABLE IF NOT EXISTS posts (
  id         TEXT PRIMARY KEY,
  channel    TEXT NOT NULL,
  thread     TEXT,                                       -- entry id, or NULL for the channel
  text       TEXT NOT NULL,
  author     TEXT NOT NULL,                              -- email (never sent to readers)
  at         TEXT NOT NULL,
  edited     TEXT
);
CREATE INDEX IF NOT EXISTS posts_at ON posts (at);
