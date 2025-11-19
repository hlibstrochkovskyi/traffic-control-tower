-- Add up migration script here
-- initial_schema.up.sql

CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE TABLE IF NOT EXISTS vehicle_positions (
                                                 time TIMESTAMPTZ NOT NULL,
                                                 vehicle_id TEXT NOT NULL,
                                                 latitude DOUBLE PRECISION,
                                                 longitude DOUBLE PRECISION,
                                                 speed DOUBLE PRECISION
);

SELECT create_hypertable('vehicle_positions', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_vehicle_id ON vehicle_positions (vehicle_id, time DESC);