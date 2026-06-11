-- Migration to drop production code tracking table

DROP TABLE IF EXISTS production_code_tracking CASCADE;
DROP TYPE IF EXISTS code_type_enum CASCADE;
