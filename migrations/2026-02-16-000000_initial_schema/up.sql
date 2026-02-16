-- Create organisations table
CREATE TABLE organisations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    logo_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    bio TEXT,
    avatar_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMP,
    role TEXT NOT NULL DEFAULT 'user',
    organisation_id UUID REFERENCES organisations(id) ON DELETE SET NULL
);

-- Create tours table
CREATE TABLE tours (
    id UUID PRIMARY KEY,
    organisation_id UUID NOT NULL REFERENCES organisations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    panorama_url TEXT NOT NULL,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create tour_links table
CREATE TABLE tour_links (
    id UUID PRIMARY KEY,
    source_node_id UUID NOT NULL REFERENCES tours(id) ON DELETE CASCADE,
    target_node_id UUID NOT NULL REFERENCES tours(id) ON DELETE CASCADE,
    yaw DOUBLE PRECISION NOT NULL,
    pitch DOUBLE PRECISION NOT NULL,
    label TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Add updated_at triggers
SELECT diesel_manage_updated_at('organisations');
SELECT diesel_manage_updated_at('users');
SELECT diesel_manage_updated_at('tours');
