-- Create function for automatic partition management

-- Function to create future partitions
CREATE OR REPLACE FUNCTION create_monthly_partition(
    p_table_name TEXT,
    p_year INT,
    p_month INT
) RETURNS TEXT
LANGUAGE plpgsql
AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
    create_sql TEXT;
BEGIN
    partition_name := p_table_name || '_y' || p_year || 'm' || LPAD(p_month::TEXT, 2, '0');
    start_date := make_date(p_year, p_month, 1);
    end_date := start_date + INTERVAL '1 month';

    -- Check if partition already exists
    IF EXISTS (
        SELECT 1 FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relname = partition_name
    ) THEN
        RETURN 'Partition ' || partition_name || ' already exists';
    END IF;

    create_sql := format(
        'CREATE TABLE %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)',
        partition_name,
        p_table_name,
        start_date,
        end_date
    );

    EXECUTE create_sql;

    RETURN 'Created partition ' || partition_name;
END;
$$;

-- Function to create partitions for upcoming months
CREATE OR REPLACE FUNCTION ensure_future_partitions(
    months_ahead INT DEFAULT 3
) RETURNS TABLE(result TEXT)
LANGUAGE plpgsql
AS $$
DECLARE
    current_date_val DATE := CURRENT_DATE;
    target_date DATE;
    i INT;
BEGIN
    -- Create partitions for failure_registry
    FOR i IN 0..months_ahead LOOP
        target_date := current_date_val + (i || ' months')::INTERVAL;
        result := create_monthly_partition('failure_registry', EXTRACT(YEAR FROM target_date)::INT, EXTRACT(MONTH FROM target_date)::INT);
        RETURN NEXT;
    END LOOP;

    -- Create partitions for audit_log
    FOR i IN 0..months_ahead LOOP
        target_date := current_date_val + (i || ' months')::INTERVAL;
        result := create_monthly_partition('audit_log', EXTRACT(YEAR FROM target_date)::INT, EXTRACT(MONTH FROM target_date)::INT);
        RETURN NEXT;
    END LOOP;
END;
$$;

-- Create initial partitions for next 3 months
SELECT * FROM ensure_future_partitions(3);

-- Add comments
COMMENT ON FUNCTION create_monthly_partition IS 'Creates a monthly partition for a partitioned table';
COMMENT ON FUNCTION ensure_future_partitions IS 'Ensures partitions exist for upcoming months';
