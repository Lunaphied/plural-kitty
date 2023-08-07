CREATE TABLE IF NOT EXISTS users (
    mxid                     TEXT PRIMARY KEY,
    current_ident            TEXT
);

CREATE TABLE IF NOT EXISTS identities (
    mxid            TEXT,
    name            TEXT,
    display_name    TEXT,
    avatar          TEXT,
    track_account   BOOLEAN NOT NULL DEFAULT FALSE,
    activators      TEXT[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (mxid, name)
);

CREATE TABLE IF NOT EXISTS ignored_rooms (
    mxid    TEXT,
    room_id TEXT,
    PRIMARY KEY (mxid, room_id)
);

CREATE TABLE IF NOT EXISTS read_msgs (
    room_id     TEXT PRIMARY KEY,
    event_id    TEXT NOT NULL
);
