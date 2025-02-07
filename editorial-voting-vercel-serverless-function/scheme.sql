--PostgreSQL

CREATE TABLE users (
    -- unique id
    id SERIAL PRIMARY KEY,
    -- /^[0-9A-Za-z]{3,16}$/
    atcoder_id VARCHAR(16) UNIQUE NOT NULL,
    -- 0 .. 9999
    rating SMALLINT,
    rating_last_update TIMESTAMP
);

CREATE TABLE editorials (
    id SERIAL PRIMARY KEY,
    editorial TEXT UNIQUE NOT NULL
);

CREATE TABLE votes (
    user_id INTEGER NOT NULL REFERENCES users (id),
    editorial_id INTEGER NOT NULL REFERENCES editorials (id),
    score SMALLINT NOT NULL,
    rating SMALLINT NOT NULL,
    PRIMARY KEY (user_id, editorial_id)
);

CREATE TABLE vote_temp (
    editorial_id INTEGER NOT NULL REFERENCES editorials (id),
    rating_level SMALLINT NOT NULL,
    score INTEGER NOT NULL,
    PRIMARY KEY (editorial_id, rating_level)
);