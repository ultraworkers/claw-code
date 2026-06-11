package adapters

import (
	"context"
	"log/slog"
	"sync"

	"github.com/thearchitectit/guardrail-mcp/internal/domain"
)

// DefaultEventBus is a simple in-memory event bus (concrete implementation)
type DefaultEventBus struct {
	subscribers map[domain.EventType][]domain.EventHandler
	mu          sync.RWMutex
}

// NewDefaultEventBus creates a new event bus
func NewDefaultEventBus() *DefaultEventBus {
	return &DefaultEventBus{
		subscribers: make(map[domain.EventType][]domain.EventHandler),
	}
}

// Ensure DefaultEventBus implements domain.EventBus
var _ domain.EventBus = (*DefaultEventBus)(nil)

func (b *DefaultEventBus) Publish(ctx context.Context, event domain.Event) {
	b.mu.RLock()
	handlers := b.subscribers[event.Type]
	// Copy slice to avoid holding lock during handler execution
	handlersCopy := make([]domain.EventHandler, len(handlers))
	copy(handlersCopy, handlers)
	b.mu.RUnlock()

	for _, handler := range handlersCopy {
		handler(ctx, event)
	}
}

func (b *DefaultEventBus) Subscribe(eventType domain.EventType, handler domain.EventHandler) {
	b.mu.Lock()
	b.subscribers[eventType] = append(b.subscribers[eventType], handler)
	b.mu.Unlock()
}

// CacheInvalidationHandler handles cache invalidation on rule changes
type CacheInvalidationHandler struct {
	cache domain.CachePort
}

// NewCacheInvalidationHandler creates a handler that invalidates cache on rule events
func NewCacheInvalidationHandler(cache domain.CachePort) *CacheInvalidationHandler {
	return &CacheInvalidationHandler{cache: cache}
}

func (h *CacheInvalidationHandler) Handle(ctx context.Context, event domain.Event) {
	if err := h.cache.InvalidateRules(ctx); err != nil {
		slog.Error("Failed to invalidate cache on rule event",
			"event_type", event.Type,
			"error", err,
		)
	}
}

// WireCacheInvalidation subscribes cache invalidation to rule change events
func WireCacheInvalidation(bus domain.EventBus, cache domain.CachePort) {
	handler := NewCacheInvalidationHandler(cache)
	bus.Subscribe(domain.EventRuleCreated, handler.Handle)
	bus.Subscribe(domain.EventRuleUpdated, handler.Handle)
	bus.Subscribe(domain.EventRuleDeleted, handler.Handle)
	bus.Subscribe(domain.EventRuleToggled, handler.Handle)
}
