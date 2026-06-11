-- Drop partition management functions
DROP FUNCTION IF EXISTS ensure_future_partitions(INT);
DROP FUNCTION IF EXISTS create_monthly_partition(TEXT, INT, INT);

-- Note: Partitions created by these functions are not dropped
-- to preserve data integrity
