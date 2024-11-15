CREATE TABLE block_data (
    id SERIAL PRIMARY KEY,
    block_height INT NOT NULL,
    block_timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE price_data (
    id SERIAL PRIMARY KEY,
    price_usd DOUBLE PRECISION NOT NULL,
    price_timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
