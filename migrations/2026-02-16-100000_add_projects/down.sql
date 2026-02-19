-- Reverse: add organisation_id back to tours, drop project_id, drop projects table
ALTER TABLE tours ADD COLUMN organisation_id UUID REFERENCES organisations(id) ON DELETE CASCADE;

UPDATE tours SET organisation_id = (
    SELECT p.organisation_id FROM projects p WHERE p.id = tours.project_id LIMIT 1
);

ALTER TABLE tours ALTER COLUMN organisation_id SET NOT NULL;
ALTER TABLE tours DROP COLUMN project_id;

DROP TABLE IF EXISTS projects;
