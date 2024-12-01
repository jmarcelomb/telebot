CREATE TABLE IF NOT EXISTS services (
    id integer PRIMARY KEY AUTOINCREMENT,
    name text NOT NULL UNIQUE,
    enable boolean,
    creation_time DATETIME DEFAULT CURRENT_TIMESTAMP
);