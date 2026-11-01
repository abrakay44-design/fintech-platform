DROP TABLE IF EXISTS entries;
DROP TABLE IF EXISTS journals;
DROP TABLE IF EXISTS posts;
DROP TABLE IF EXISTS accounts;

CREATE TABLE IF NOT EXISTS posts (
    post_id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS entries (
    entry_id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(post_id) ON DELETE CASCADE,
    account_id UUID NOT NULL,
    asset JSONB NOT NULL,
    debit NUMERIC(38, 18),
    credit NUMERIC(38, 18),
    CHECK ((debit IS NULL) != (credit IS NULL))
);

CREATE INDEX IF NOT EXISTS idx_entries_post_id ON entries(post_id);
CREATE INDEX IF NOT EXISTS idx_entries_account_id ON entries(account_id);
