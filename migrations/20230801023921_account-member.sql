ALTER TABLE identities
ADD COLUMN track_account boolean;

UPDATE identities SET track_account = false;

ALTER TABLE identities
ALTER COLUMN track_account SET NOT NULL;
