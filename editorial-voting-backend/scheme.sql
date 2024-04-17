CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- atcoder_id =~ /^[0-9A-Za-z]{3,13}$/
    atcoder_id TEXT UNIQUE NOT NULL,
    -- rating in 0 .. 9999
    rating INTEGER,
    -- rating_last_change = unix time
    rating_last_update INTEGER
);

CREATE TABLE editorials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    editorial TEXT UNIQUE NOT NULL
);

CREATE TABLE votes (
    user_id INTEGER NOT NULL,
    editorial_id INTEGER NOT NULL,
    score INTEGER NOT NULL,
    rating INTEGER NOT NULL,
    PRIMARY KEY (user_id, editorial_id)
);

CREATE TABLE vote_temp (
    editorial_id INTEGER NOT NULL,
    rating_level INTEGER NOT NULL,
    score INTEGER NOT NULL,
    PRIMARY KEY (editorial_id, rating_level)
);