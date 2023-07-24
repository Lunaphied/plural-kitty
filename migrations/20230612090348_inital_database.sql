CREATE TABLE IF NOT EXISTS users (
    mxid            TEXT PRIMARY KEY,
    current_ident   BIGINT
);

CREATE TABLE IF NOT EXISTS identities (
    mxid            TEXT,
    name            TEXT,
    display_name    TEXT,
    avatar          TEXT,
    PRIMARY KEY (mxid, name)
);

CREATE TABLE IF NOT EXISTS activators (
    mxid            TEXT NOT NULL,
    name            TEXT NOT NULL,
    value           TEXT NOT NULL,
    PRIMARY KEY (mxid, value)
);

CREATE INDEX activator_id ON activators (mxid, name);

CREATE TABLE IF NOT EXISTS read_msgs (
    room_id     TEXT PRIMARY KEY,
    event_id    TEXT NOT NULL
);
