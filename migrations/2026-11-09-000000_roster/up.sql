CREATE TABLE roster_events (id text PRIMARY KEY, data jsonb NOT NULL);
CREATE TABLE roster_members (id text PRIMARY KEY, data jsonb NOT NULL);
CREATE UNIQUE INDEX roster_member_phone ON roster_members ((data->>'phone'));
CREATE UNIQUE INDEX roster_member_number ON roster_members ((data->>'memberId')) WHERE data->>'memberId' IS NOT NULL;
CREATE TABLE roster_zones (id text PRIMARY KEY, event_id text NOT NULL REFERENCES roster_events ON DELETE CASCADE, data jsonb NOT NULL, UNIQUE(id,event_id));
CREATE UNIQUE INDEX roster_zone_map ON roster_zones(event_id,(data->>'mapRef')) WHERE data->>'mapRef' IS NOT NULL;
CREATE TABLE roster_positions (id text PRIMARY KEY, zone_id text NOT NULL REFERENCES roster_zones ON DELETE CASCADE, data jsonb NOT NULL, UNIQUE(id,zone_id));
CREATE UNIQUE INDEX roster_position_spot ON roster_positions(zone_id,(data->>'spot')) WHERE data->>'spot' IS NOT NULL;
CREATE TABLE roster_assignments (
 id text PRIMARY KEY,
 event_id text NOT NULL REFERENCES roster_events ON DELETE CASCADE,
 member_id text NOT NULL REFERENCES roster_members ON DELETE CASCADE,
 zone_id text NOT NULL,
 position_id text NOT NULL,
 is_spot boolean NOT NULL,
 data jsonb NOT NULL,
 FOREIGN KEY(zone_id,event_id) REFERENCES roster_zones(id,event_id) ON DELETE CASCADE,
 FOREIGN KEY(position_id,zone_id) REFERENCES roster_positions(id,zone_id) ON DELETE CASCADE,
 UNIQUE(event_id,member_id),
 CHECK (data->>'status' IN ('assigned','confirmed','unavailable')),
 CHECK (data->>'status' <> 'unavailable' OR NOT (data->>'isTeamLead')::boolean)
);
CREATE UNIQUE INDEX roster_occupied_spot ON roster_assignments(position_id) WHERE is_spot AND data->>'status' <> 'unavailable';
CREATE UNIQUE INDEX roster_zone_lead ON roster_assignments(zone_id) WHERE (data->>'isTeamLead')::boolean;
CREATE INDEX roster_assignment_event ON roster_assignments(event_id);
CREATE INDEX roster_assignment_member ON roster_assignments(member_id);
CREATE INDEX roster_position_zone ON roster_positions(zone_id);
CREATE TABLE roster_settings (id text PRIMARY KEY CHECK(id='settings'), data jsonb NOT NULL, current_event_id text GENERATED ALWAYS AS (data->>'currentEventId') STORED REFERENCES roster_events(id));
INSERT INTO roster_settings VALUES ('settings', '{"currentEventId":null,"contactName":"Department Secretariat","contactPhone":"","showTeammatePhones":false,"allowSelfConfirm":true}');
CREATE TABLE roster_activity (id text PRIMARY KEY, data jsonb NOT NULL);
CREATE INDEX roster_activity_at ON roster_activity ((data->>'at') DESC);
CREATE TABLE roster_notifications (id text PRIMARY KEY, data jsonb NOT NULL);
CREATE TABLE roster_admins (id text PRIMARY KEY, email text NOT NULL UNIQUE, password_hash text NOT NULL, data jsonb NOT NULL);
CREATE TABLE roster_tokens (id text PRIMARY KEY, subject text NOT NULL, kind text NOT NULL, expires_at timestamptz NOT NULL, secret_hash text NOT NULL);
CREATE INDEX roster_token_expiry ON roster_tokens(expires_at);
CREATE TABLE roster_imports (id text PRIMARY KEY, data jsonb NOT NULL, expires_at timestamptz NOT NULL);
CREATE TABLE roster_rate_limits (id text PRIMARY KEY, hits integer NOT NULL, expires_at timestamptz NOT NULL);
