CREATE TABLE IF NOT EXISTS users (
    id              BIGSERIAL PRIMARY KEY,
    mxid            TEXT NOT NULL,
    current_ident   BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS identies (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL,
    name            TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    avatar          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS activator (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL,
    value           TEXT NOT NULL
);
