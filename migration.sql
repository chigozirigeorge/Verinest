-- Down
DROP TABLE IF EXISTS users;
DROP TYPE IF EXISTS user_role;
DROP TYPE IF EXISTS verification_type;
DROP EXTENSION IF EXISTS "uuid-ossp";


CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE user_role AS ENUM (
    'super_admin', 'admin', 'moderator', 'verifier', 'lawyer',
    'agent', 'landlord', 'whistleblower', 'customer_care', 'dev', 'user'
);

CREATE TYPE verification_type AS ENUM ('national_id', 'driver_license', 'passport');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    username VARCHAR(100) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password VARCHAR(100) NOT NULL,
    trust_score INTEGER NOT NULL DEFAULT 100,
    role user_role DEFAULT 'user' NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    verification_type verification_type NOT NULL DEFAULT 'national_id',
    verification_number VARCHAR(100),
    wallet_address VARCHAR(255),
    nationality VARCHAR(100),
    dob TIMESTAMPTZ,
    lga VARCHAR(255),
    transaction_pin SMALLINT,
    next_of_kin VARCHAR(100),
    verification_token VARCHAR(255),
    token_expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX users_email_idx ON users (email);
CREATE INDEX users_username_idx ON users (username);

