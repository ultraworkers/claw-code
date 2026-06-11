package adapters

import (
	"context"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/domain"
)

// RuleStoreAdapter wraps the concrete database.RuleStore behind
// the domain RuleRepository interface
type RuleStoreAdapter struct {
	store *database.RuleStore
}

// NewRuleStoreAdapter creates an adapter from a RuleStore
func NewRuleStoreAdapter(store *database.RuleStore) *RuleStoreAdapter {
	return &RuleStoreAdapter{store: store}
}

// Ensure RuleStoreAdapter implements domain.RuleRepository
var _ domain.RuleRepository = (*RuleStoreAdapter)(nil)

func (a *RuleStoreAdapter) GetByID(ctx context.Context, id uuid.UUID) (*domain.PreventionRule, error) {
	rule, err := a.store.GetByID(ctx, id)
	if err != nil {
		return nil, err
	}
	return toDomainRule(rule), nil
}

func (a *RuleStoreAdapter) GetByRuleID(ctx context.Context, ruleID string) (*domain.PreventionRule, error) {
	rule, err := a.store.GetByRuleID(ctx, ruleID)
	if err != nil {
		return nil, err
	}
	return toDomainRule(rule), nil
}

func (a *RuleStoreAdapter) List(ctx context.Context, enabled *bool, category string, limit, offset int) ([]domain.PreventionRule, error) {
	rules, err := a.store.List(ctx, enabled, category, limit, offset)
	if err != nil {
		return nil, err
	}
	return toDomainRules(rules), nil
}

func (a *RuleStoreAdapter) GetActiveRules(ctx context.Context) ([]domain.PreventionRule, error) {
	rules, err := a.store.GetActiveRules(ctx)
	if err != nil {
		return nil, err
	}
	return toDomainRules(rules), nil
}

func (a *RuleStoreAdapter) Create(ctx context.Context, rule *domain.PreventionRule) error {
	dbRule := toDBRule(rule)
	return a.store.Create(ctx, dbRule)
}

func (a *RuleStoreAdapter) Update(ctx context.Context, rule *domain.PreventionRule) error {
	dbRule := toDBRule(rule)
	return a.store.Update(ctx, dbRule)
}

func (a *RuleStoreAdapter) Delete(ctx context.Context, id uuid.UUID) error {
	return a.store.Delete(ctx, id)
}

func (a *RuleStoreAdapter) Count(ctx context.Context, enabled *bool, category string) (int, error) {
	return a.store.Count(ctx, enabled, category)
}

func (a *RuleStoreAdapter) Toggle(ctx context.Context, id uuid.UUID, enabled bool) error {
	return a.store.Toggle(ctx, id, enabled)
}

// toDomainRule converts database.PreventionRule to domain.PreventionRule
func toDomainRule(rule *database.PreventionRule) *domain.PreventionRule {
	if rule == nil {
		return nil
	}
	return &domain.PreventionRule{
		ID:          rule.ID,
		RuleID:      rule.RuleID,
		Name:        rule.Name,
		Pattern:     rule.Pattern,
		PatternHash: rule.PatternHash,
		Message:     rule.Message,
		Severity:    domain.Severity(rule.Severity),
		Enabled:     rule.Enabled,
		DocumentID:  rule.DocumentID,
		Category:    rule.Category,
		CreatedAt:   rule.CreatedAt,
		UpdatedAt:   rule.UpdatedAt,
	}
}

func toDomainRules(rules []database.PreventionRule) []domain.PreventionRule {
	result := make([]domain.PreventionRule, len(rules))
	for i, r := range rules {
		result[i] = *toDomainRule(&r)
	}
	return result
}

func toDBRule(rule *domain.PreventionRule) *database.PreventionRule {
	return &database.PreventionRule{
		ID:          rule.ID,
		RuleID:      rule.RuleID,
		Name:        rule.Name,
		Pattern:     rule.Pattern,
		PatternHash: rule.PatternHash,
		Message:     rule.Message,
		Severity:    database.Severity(rule.Severity),
		Enabled:     rule.Enabled,
		DocumentID:  rule.DocumentID,
		Category:    rule.Category,
		CreatedAt:   rule.CreatedAt,
		UpdatedAt:   rule.UpdatedAt,
	}
}

// CacheAdapter wraps the cache client behind the domain.CachePort interface
type CacheAdapter struct {
	client  *database.Client
	cache   *database.Cache
}

func NewCacheAdapter(client *database.Client, cache *database.Cache) *CacheAdapter {
	return &CacheAdapter{client: client, cache: cache}
}

// Ensure CacheAdapter implements domain.CachePort
var _ domain.CachePort = (*CacheAdapter)(nil)

func (a *CacheAdapter) GetActiveRules(ctx context.Context) ([]domain.PreventionRule, error) {
	// Read from cache
	result, err := a.cache.GetActiveRules(ctx)
	if err != nil || result != nil {
		return toDomainRules(result), err
	}
	return nil, nil // Cache miss
}

func (a *CacheAdapter) SetActiveRules(ctx context.Context, rules []domain.PreventionRule, ttl time.Duration) error {
	dbRules := toDBRules(rules)
	return a.cache.SetActiveRules(ctx, dbRules, ttl)
}

func (a *CacheAdapter) InvalidateRules(ctx context.Context) error {
	return a.cache.InvalidateRules(ctx)
}

func toDBRules(rules []domain.PreventionRule) []database.PreventionRule {
	result := make([]database.PreventionRule, len(rules))
	for i, r := range rules {
		result[i] = database.PreventionRule{
			ID:          r.ID,
			RuleID:      r.RuleID,
			Name:        r.Name,
			Pattern:     r.Pattern,
			PatternHash: r.PatternHash,
			Message:     r.Message,
			Severity:    database.Severity(r.Severity),
			Enabled:     r.Enabled,
			DocumentID:  r.DocumentID,
			Category:    r.Category,
			CreatedAt:   r.CreatedAt,
			UpdatedAt:   r.UpdatedAt,
		}
	}
	return result
}
