-- Add is_public column to projects table
ALTER TABLE projects ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT FALSE;
