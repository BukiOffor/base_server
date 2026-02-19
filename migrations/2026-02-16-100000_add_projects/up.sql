-- Create projects table
CREATE TABLE projects (
    id UUID PRIMARY KEY,
    organisation_id UUID NOT NULL REFERENCES organisations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
SELECT diesel_manage_updated_at('projects');

-- Migrate existing tours: create a default project per org, then reassign
INSERT INTO projects (id, organisation_id, name, description)
SELECT gen_random_uuid(), id, name || ' - Default', 'Auto-migrated project' FROM organisations;

-- Add project_id column to tours
ALTER TABLE tours ADD COLUMN project_id UUID REFERENCES projects(id) ON DELETE CASCADE;

-- Backfill project_id from the default project created above
UPDATE tours SET project_id = (
    SELECT p.id FROM projects p WHERE p.organisation_id = tours.organisation_id LIMIT 1
);

-- Make project_id NOT NULL and drop organisation_id
ALTER TABLE tours ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE tours DROP COLUMN organisation_id;
