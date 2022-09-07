CREATE TABLE IF NOT EXISTS lists
(
    id          INTEGER PRIMARY KEY NOT NULL,
    name        TEXT                NOT NULL
);

CREATE TABLE IF NOT EXISTS todos
(
    id          INTEGER PRIMARY KEY NOT NULL,
    text        TEXT                NOT NULL,
    checked     BOOLEAN             NOT NULL DEFAULT 0,
    list_id     INTEGER             NOT NULL,
    FOREIGN KEY(list_id) REFERENCES lists(id)
);

