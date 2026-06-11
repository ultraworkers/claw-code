package vision

import (
	"database/sql"
	"fmt"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

// Storage persists reviews, iterations, and findings to SQLite.
type Storage struct {
	db *sql.DB
}

// NewStorage opens or creates the SQLite database at the given path.
func NewStorage(dbPath string) (*Storage, error) {
	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		return nil, err
	}
	if err := db.Ping(); err != nil {
		return nil, err
	}
	s := &Storage{db: db}
	if err := s.migrate(); err != nil {
		return nil, err
	}
	return s, nil
}

func (s *Storage) migrate() error {
	schema := `
CREATE TABLE IF NOT EXISTS reviews (
	id TEXT PRIMARY KEY,
	screenshot_path TEXT NOT NULL,
	status TEXT NOT NULL,
	created_at DATETIME NOT NULL,
	completed_at DATETIME
);

CREATE TABLE IF NOT EXISTS iterations (
	id TEXT PRIMARY KEY,
	review_id TEXT NOT NULL REFERENCES reviews(id),
	backend_used TEXT NOT NULL,
	model_used TEXT NOT NULL,
	prompt_type TEXT NOT NULL,
	raw_response TEXT,
	findings_json TEXT,
	confidence REAL,
	latency_ms INTEGER,
	created_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS findings (
	id TEXT PRIMARY KEY,
	review_id TEXT NOT NULL REFERENCES reviews(id),
	category TEXT NOT NULL,
	severity TEXT NOT NULL,
	description TEXT NOT NULL,
	bbox TEXT,
	accepted INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_iterations_review ON iterations(review_id);
CREATE INDEX IF NOT EXISTS idx_findings_review ON findings(review_id);
`
	_, err := s.db.Exec(schema)
	return err
}

// CreateReview inserts a new review.
func (s *Storage) CreateReview(r *Review) error {
	_, err := s.db.Exec(
		"INSERT INTO reviews (id, screenshot_path, status, created_at, completed_at) VALUES (?, ?, ?, ?, ?)",
		r.ID, r.ScreenshotPath, r.Status, r.CreatedAt, r.CompletedAt,
	)
	return err
}

// GetReview retrieves a review by ID.
func (s *Storage) GetReview(id string) (*Review, error) {
	var r Review
	var completedAt sql.NullTime
	err := s.db.QueryRow("SELECT id, screenshot_path, status, created_at, completed_at FROM reviews WHERE id = ?", id).
		Scan(&r.ID, &r.ScreenshotPath, &r.Status, &r.CreatedAt, &completedAt)
	if err != nil {
		return nil, err
	}
	if completedAt.Valid {
		r.CompletedAt = &completedAt.Time
	}
	return &r, nil
}

// UpdateReviewStatus updates the status and optionally completed_at.
func (s *Storage) UpdateReviewStatus(id string, status ReviewStatus, completed *time.Time) error {
	_, err := s.db.Exec("UPDATE reviews SET status = ?, completed_at = ? WHERE id = ?", status, completed, id)
	return err
}

// ListReviews returns all reviews ordered by created_at desc.
func (s *Storage) ListReviews(limit int) ([]Review, error) {
	if limit <= 0 {
		limit = 50
	}
	rows, err := s.db.Query("SELECT id, screenshot_path, status, created_at, completed_at FROM reviews ORDER BY created_at DESC LIMIT ?", limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []Review
	for rows.Next() {
		var r Review
		var completedAt sql.NullTime
		if err := rows.Scan(&r.ID, &r.ScreenshotPath, &r.Status, &r.CreatedAt, &completedAt); err != nil {
			return nil, err
		}
		if completedAt.Valid {
			r.CompletedAt = &completedAt.Time
		}
		out = append(out, r)
	}
	return out, rows.Err()
}

// CreateIteration inserts an iteration.
func (s *Storage) CreateIteration(it *Iteration) error {
	_, err := s.db.Exec(
		"INSERT INTO iterations (id, review_id, backend_used, model_used, prompt_type, raw_response, findings_json, confidence, latency_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
		it.ID, it.ReviewID, it.BackendUsed, it.ModelUsed, it.PromptType, it.RawResponse, it.FindingsJSON, it.Confidence, it.LatencyMs, it.CreatedAt,
	)
	return err
}

// ListIterations returns iterations for a review.
func (s *Storage) ListIterations(reviewID string) ([]Iteration, error) {
	rows, err := s.db.Query("SELECT id, review_id, backend_used, model_used, prompt_type, raw_response, findings_json, confidence, latency_ms, created_at FROM iterations WHERE review_id = ? ORDER BY created_at", reviewID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []Iteration
	for rows.Next() {
		var it Iteration
		if err := rows.Scan(&it.ID, &it.ReviewID, &it.BackendUsed, &it.ModelUsed, &it.PromptType, &it.RawResponse, &it.FindingsJSON, &it.Confidence, &it.LatencyMs, &it.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, it)
	}
	return out, rows.Err()
}

// CreateFinding inserts a finding.
func (s *Storage) CreateFinding(f *Finding) error {
	bbox := ""
	if f.Bbox != nil {
		bbox = fmt.Sprintf("%f,%f,%f,%f", f.Bbox.X, f.Bbox.Y, f.Bbox.Width, f.Bbox.Height)
	}
	_, err := s.db.Exec(
		"INSERT INTO findings (id, review_id, category, severity, description, bbox, accepted) VALUES (?, ?, ?, ?, ?, ?, ?)",
		f.ID, f.ReviewID, f.Category, f.Severity, f.Description, bbox, f.Accepted,
	)
	return err
}

// ListFindings returns findings for a review.
func (s *Storage) ListFindings(reviewID string) ([]Finding, error) {
	rows, err := s.db.Query("SELECT id, review_id, category, severity, description, bbox, accepted FROM findings WHERE review_id = ?", reviewID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []Finding
	for rows.Next() {
		var f Finding
		var bboxStr string
		if err := rows.Scan(&f.ID, &f.ReviewID, &f.Category, &f.Severity, &f.Description, &bboxStr, &f.Accepted); err != nil {
			return nil, err
		}
		if bboxStr != "" {
			var b Bbox
			fmt.Sscanf(bboxStr, "%f,%f,%f,%f", &b.X, &b.Y, &b.Width, &b.Height)
			f.Bbox = &b
		}
		out = append(out, f)
	}
	return out, rows.Err()
}

// Close closes the database.
func (s *Storage) Close() error {
	return s.db.Close()
}
