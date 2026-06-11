# State Management & Data Patterns

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL applications with client state, server state, or real-time data requirements

---

## Purpose

State management is where AI-generated applications most often break down. Without clear patterns, agents reinvent state architectures on every generation — wasting tokens and introducing inconsistency. These patterns give agents a decision tree so they can select the right approach instantly.

**Core Principle:** Pick the simplest state solution that works. Complexity is earned, not assumed.

---

## State Architecture Decision Tree

```
Is the state used by only one component?
├─ YES → Local state (useState/signals)
├─ NO → Is it server data?
│  ├─ YES → Server state (React Query/SWR/TanStack Query)
│  └─ NO → Is it shared across many components?
│     ├─ YES → Global client state (Zustand/Jotai)
│     └─ NO → Lift state to nearest common parent
```

---

## Client State Patterns

### Local State (Single Component)

Use for: Form inputs, toggle states, UI-only state

```typescript
// Preferred: Simple local state
function Toggle() {
  const [isOpen, setIsOpen] = useState(false);
  return <button onClick={() => setIsOpen(!isOpen)}>{isOpen ? 'Close' : 'Open'}</button>;
}
```

```rust
// Signals pattern (Leptos/Dioxus)
fn toggle() -> impl IntoView {
    let (is_open, set_is_open) = create_signal(false);
    view! { <button on:click=move |_| set_is_open.update(|v| *v = !*v)>
        {move || if is_open() { "Close" } else { "Open" }}
    </button> }
}
```

### Global Client State (Shared Across Components)

Use for: Theme, user preferences, UI layout state, feature flags

```typescript
// Zustand — preferred for global client state
import { create } from 'zustand';

interface AppState {
  theme: 'light' | 'dark';
  sidebarOpen: boolean;
  setTheme: (theme: 'light' | 'dark') => void;
  toggleSidebar: () => void;
}

const useAppStore = create<AppState>((set) => ({
  theme: 'light',
  sidebarOpen: true,
  setTheme: (theme) => set({ theme }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
}));
```

```go
// Go equivalent: typed state container
type AppState struct {
    mu          sync.RWMutex
    Theme       string
    SidebarOpen bool
}

func (s *AppState) SetTheme(theme string) {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.Theme = theme
}

func (s *AppState) ToggleSidebar() {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.SidebarOpen = !s.SidebarOpen
}
```

### Atomic State (Fine-Grained Reactivity)

Use for: Performance-critical UIs, large forms, independent reactive values

```typescript
// Jotai — atomic state for fine-grained updates
import { atom, useAtom } from 'jotai';

const themeAtom = atom<'light' | 'dark'>('light');
const sidebarAtom = atom(true);

// Derived atoms for computed state
const isDarkAtom = atom((get) => get(themeAtom) === 'dark');
```

---

## Server State Patterns

### Data Fetching (React Query / TanStack Query)

Use for: ALL server data. Never store server responses in client state.

```typescript
// React Query — the standard for server state
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

function usePlayer(id: string) {
  return useQuery({
    queryKey: ['player', id],
    queryFn: () => fetchPlayer(id),
    staleTime: 30_000,        // Consider fresh for 30s
    gcTime: 5 * 60_000,       // Garbage collect after 5min
  });
}

function useUpdatePlayer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: updatePlayer,
    onSuccess: (data) => {
      queryClient.setQueryData(['player', data.id], data); // Optimistic update
    },
  });
}
```

---

## Offline-First & Local Persistence

### When to Use

- Mobile apps with intermittent connectivity
- Games with local save data
- Apps that should work without internet

### Persistence Layer

```typescript
// Zustand with persistence middleware
import { persist } from 'zustand/middleware';

const useGameStore = create(
  persist<GameState>(
    (set) => ({
      level: 1,
      score: 0,
      // ...
    }),
    {
      name: 'game-save',
      storage: createJSONStorage(() => localStorage),
      // NEVER persist auth tokens in localStorage
    }
  )
);
```

---

## Real-Time & CRDT Collaboration

### When to Use CRDTs

- Multiple users editing the same document
- Collaborative whiteboards or game states
- Conflict-free offline-to-online sync

```typescript
// Y.js CRDT integration pattern
import * as Y from 'yjs';
import { WebsocketProvider } from 'y-websocket';

const ydoc = new Y.Doc();
const provider = new WebsocketProvider('wss://sync.example.com', 'room-id', ydoc);
const ymap = ydoc.getMap('shared-state');

// Observe changes from any peer
ymap.observe((event) => {
  // Update local UI reactively
});
```

---

## Forbidden Patterns

| Pattern | Why Forbidden | Use Instead |
|---------|---------------|-------------|
| Global mutable singletons | Untraceable state changes, impossible to test | Zustand/Jotai stores |
| Untyped React Context for state | Type-unsafe, re-renders entire tree | Zustand with selectors |
| localStorage for auth tokens | XSS vulnerability | httpOnly cookies or secure storage |
| Storing server data in useState | Stale data, no cache invalidation | React Query / TanStack Query |
| Prop drilling > 3 levels | Maintenance nightmare | State library or composition |
| Redux for new projects | Excessive boilerplate for most apps | Zustand (simpler API, same patterns) |

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] Auth tokens or session data need to be stored (security-critical)
- [ ] Database schema changes are required for state persistence
- [ ] Real-time sync architecture is being designed (CRDT vs OT vs custom)
- [ ] State needs to be shared across micro-frontends or services
- [ ] Offline-first strategy requires conflict resolution rules
- [ ] Performance profiling shows state updates causing frame drops

---

## Language Patterns

### TypeScript
```typescript
// Type-safe state factory
function createTypedStore<T extends Record<string, unknown>>(initialState: T) {
  return create<T & { reset: () => void }>((set) => ({
    ...initialState,
    reset: () => set(initialState),
  }));
}
```

### Rust
```rust
// Type-safe state container
use std::sync::{Arc, RwLock};

pub struct AppState<T: Clone + Send + Sync> {
    inner: Arc<RwLock<T>>,
}

impl<T: Clone + Send + Sync> AppState<T> {
    pub fn new(initial: T) -> Self {
        Self { inner: Arc::new(RwLock::new(initial)) }
    }

    pub fn read(&self) -> T {
        self.inner.read().unwrap().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let mut state = self.inner.write().unwrap();
        f(&mut state);
    }
}
```

### Go
```go
// Type-safe state with generics
type Store[T any] struct {
    mu    sync.RWMutex
    state T
}

func NewStore[T any](initial T) *Store[T] {
    return &Store[T]{state: initial}
}

func (s *Store[T]) Get() T {
    s.mu.RLock()
    defer s.mu.RUnlock()
    return s.state
}

func (s *Store[T]) Update(fn func(*T)) {
    s.mu.Lock()
    defer s.mu.Unlock()
    fn(&s.state)
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development decision matrix |
| [2026_UI_UX_STANDARD.md](../ui-ux/2026_UI_UX_STANDARD.md) | UI component patterns |
| [2026_GAME_DESIGN.md](../game-design/2026_GAME_DESIGN.md) | Game state requirements |
| [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | Core safety protocols |
