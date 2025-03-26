CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Add up migration script here
BEGIN;

CREATE TABLE shortened_urls (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    original_url TEXT NOT NULL CHECK (LENGTH(original_url) <= 2048), -- Added length constraint for practicality
    short_code VARCHAR(10) NOT NULL UNIQUE CHECK (short_code ~ '^[a-zA-Z0-9]+$'), -- Added format validation
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE, -- Nullable for no expiration
    last_accessed TIMESTAMP WITH TIME ZONE DEFAULT NOW(), -- Default to creation time, nullable still allowed
    access_count BIGINT NOT NULL DEFAULT 0,
    -- created_by uuid_generate_v4() NOT NULL DEFAULT 'anonymous', 
    is_custom_code BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    metadata JSONB
);

-- Create indices for performance optimization
CREATE INDEX idx_shortened_urls_short_code ON shortened_urls(short_code);
CREATE INDEX idx_shortened_urls_created_at ON shortened_urls(created_at);
CREATE INDEX idx_shortened_urls_expires_at ON shortened_urls(expires_at) 
    WHERE expires_at IS NOT NULL;
-- CREATE INDEX idx_shortened_urls_created_by ON shortened_urls(created_by); -- Full index since created_by is now non-nullable

-- Add table and column descriptions
COMMENT ON TABLE shortened_urls IS 'Stores shortened URLs with metadata for tracking and redirection';
COMMENT ON COLUMN shortened_urls.original_url IS 'The original long URL to redirect to (max 2048 chars)';
COMMENT ON COLUMN shortened_urls.short_code IS 'Unique alphanumeric shortcode used in the shortened URL (max 10 chars)';
COMMENT ON COLUMN shortened_urls.expires_at IS 'When the URL expires, NULL means no expiration';
COMMENT ON COLUMN shortened_urls.access_count IS 'Counter for tracking URL usage';
-- COMMENT ON COLUMN shortened_urls.created_by IS 'Identifier for who created the URL, defaults to ''anonymous''';
COMMENT ON COLUMN shortened_urls.is_custom_code IS 'Whether the short code was custom or auto-generated';
COMMENT ON COLUMN shortened_urls.is_active IS 'Whether the shortlink is active or disabled';
COMMENT ON COLUMN shortened_urls.metadata IS 'Additional metadata in JSON format (tags, tracking info, etc.)';

COMMIT;
