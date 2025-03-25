-- Add down migration script here
BEGIN;

DROP TABLE IF EXISTS shortened_urls; -- Simplified; indices are dropped automatically

COMMIT;
