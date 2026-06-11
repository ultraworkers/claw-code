package adapters

import (
	"context"

	"github.com/thearchitectit/guardrail-mcp/internal/domain"
	"github.com/thearchitectit/guardrail-mcp/internal/validation"
)

// ValidationEngineAdapter wraps the concrete ValidationEngine behind
// the domain GuardrailService interface (Dependency Inversion Principle)
type ValidationEngineAdapter struct {
	engine *validation.ValidationEngine
}

// NewValidationEngineAdapter creates an adapter from a ValidationEngine
func NewValidationEngineAdapter(engine *validation.ValidationEngine) *ValidationEngineAdapter {
	return &ValidationEngineAdapter{engine: engine}
}

// Ensure ValidationEngineAdapter implements GuardrailService
var _ domain.GuardrailService = (*ValidationEngineAdapter)(nil)

func (a *ValidationEngineAdapter) EvaluateCommand(ctx context.Context, command string) ([]domain.Violation, error) {
	vs, err := a.engine.ValidateBash(ctx, command)
	if err != nil {
		return nil, err
	}
	return toDomainViolations(vs), nil
}

func (a *ValidationEngineAdapter) EvaluateGit(ctx context.Context, command string) ([]domain.Violation, error) {
	vs, err := a.engine.ValidateGit(ctx, command)
	if err != nil {
		return nil, err
	}
	return toDomainViolations(vs), nil
}

func (a *ValidationEngineAdapter) EvaluateFileEdit(ctx context.Context, filePath, content, sessionID string) ([]domain.Violation, error) {
	vs, err := a.engine.ValidateFileEdit(ctx, filePath, content, sessionID)
	if err != nil {
		return nil, err
	}
	return toDomainViolations(vs), nil
}

func (a *ValidationEngineAdapter) EvaluateInput(ctx context.Context, input string, categories []string) ([]domain.Violation, error) {
	vs, err := a.engine.ValidateInput(ctx, input, categories)
	if err != nil {
		return nil, err
	}
	return toDomainViolations(vs), nil
}

func (a *ValidationEngineAdapter) CheckFileRead(ctx context.Context, sessionID, filePath string) (*domain.FileReadVerification, error) {
	v, err := a.engine.VerifyFileRead(ctx, sessionID, filePath)
	if err != nil {
		return nil, err
	}
	return &domain.FileReadVerification{
		WasRead:       v.WasRead,
		ReadAt:        v.ReadAt,
		TimeSinceRead: v.TimeSinceRead,
	}, nil
}

// toDomainViolations converts validation.Violation to domain.Violation
func toDomainViolations(vs []validation.Violation) []domain.Violation {
	result := make([]domain.Violation, len(vs))
	for i, v := range vs {
		result[i] = domain.Violation{
			RuleID:         v.RuleID,
			RuleName:       v.RuleName,
			Severity:       domain.Severity(v.Severity),
			Message:        v.Message,
			Category:       v.Category,
			MatchedPattern: v.MatchedPattern,
			MatchedInput:   v.MatchedInput,
		}
	}
	return result
}
