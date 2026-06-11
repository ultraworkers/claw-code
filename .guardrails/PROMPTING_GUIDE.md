# Master Prompting Guide

> How to write prompts that work beautifully with Agent Guardrails

**TL;DR:** Be explicit, provide context, define scope, and the guardrails will keep your AI on track.

---

## Table of Contents

1. [The Golden Rules](#the-golden-rules)
2. [Prompt Templates](#prompt-templates)
3. [Common Patterns](#common-patterns)
4. [Advanced Techniques](#advanced-techniques)
5. [Examples by Use Case](#examples-by-use-case)
6. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
7. [Troubleshooting](#troubleshooting)

---

## The Golden Rules

### Rule 1: Start with Context

❌ **Bad:**
```
Fix the bug
```

✅ **Good:**
```
There's a bug in the authentication system where users can't log in with valid credentials. 

Context:
- Repository: myapp/backend
- File: src/auth/login.js
- Error: "Invalid credentials" even with correct password
- Database: PostgreSQL
- Framework: Express.js

Task: Find and fix the login bug. The issue is likely in the password comparison logic.
```

### Rule 2: Define Scope Explicitly

❌ **Bad:**
```
Update the API
```

✅ **Good:**
```
Update the user API endpoints to add email validation.

Scope:
- File: src/routes/users.js
- Only modify POST /api/users and PUT /api/users/:id
- Do NOT touch authentication or other routes
- Add validation using Joi schema
- Return 400 if email is invalid
```

### Rule 3: Provide Constraints

❌ **Bad:**
```
Refactor the code
```

✅ **Good:**
```
Refactor the data processing module to improve readability.

Constraints:
- Keep all existing functionality
- Maintain backward compatibility
- Don't change function signatures
- Add unit tests for new helper functions
- Use existing patterns from src/utils/helpers.js
```

### Rule 4: Include Examples

❌ **Bad:**
```
Add error handling
```

✅ **Good:**
```
Add error handling to the file upload endpoint.

Current code (src/routes/upload.js):
```javascript
app.post('/upload', (req, res) => {
  const file = req.files.file;
  fs.writeFileSync('/uploads/' + file.name, file.data);
  res.json({ success: true });
});
```

Expected behavior:
- Handle missing file: return 400 with error "No file provided"
- Handle file too large (>10MB): return 413 with error "File too large"
- Handle disk full: return 500 with error "Storage error"
- Always return JSON: { success: boolean, error?: string }

Example error response:
```json
{ "success": false, "error": "No file provided" }
```
```

> **Why explicit context enables speed:** Every detail you provide upfront is a clarification your AI agent doesn't need to ask for. Explicit context eliminates round-trips, reducing a 5-prompt conversation to a single generation. The most productive vibe coding sessions start with the richest prompts.

---

## Prompt Templates

### Template 1: Feature Implementation

```markdown
## Feature: [Feature Name]

### Context
[Background information about the feature]

### Requirements
- [ ] Requirement 1
- [ ] Requirement 2
- [ ] Requirement 3

### Scope
- Files to modify: [list files]
- Files to NOT touch: [list files]
- New files to create: [list files]

### Technical Details
- Framework: [framework]
- Language: [language]
- Patterns to follow: [reference existing code]

### Acceptance Criteria
1. [Criteria 1]
2. [Criteria 2]
3. [Criteria 3]

### Testing
- [ ] Unit tests written
- [ ] Integration tests pass
- [ ] Manual testing completed

### Additional Notes
[Any special considerations]
```

### Template 2: Bug Fix

```markdown
## Bug Fix: [Bug Title]

### Problem
[Clear description of the bug]

### Steps to Reproduce
1. Step 1
2. Step 2
3. Step 3

### Expected Behavior
[What should happen]

### Actual Behavior
[What actually happens]

### Context
- File(s) involved: [list]
- Error message: [if any]
- Environment: [dev/staging/prod]

### Root Cause (if known)
[Your analysis]

### Proposed Solution
[Your suggestion, or leave blank]

### Testing After Fix
- [ ] Reproduction steps no longer trigger bug
- [ ] Related functionality still works
- [ ] Edge cases handled
```

### Template 3: Code Review

```markdown
## Code Review Request

### PR/MR Information
- Branch: [branch name]
- Changes: [files modified]
- Lines changed: [+X, -Y]

### Focus Areas
- [ ] Logic correctness
- [ ] Edge cases
- [ ] Performance
- [ ] Security
- [ ] Style/consistency

### Specific Questions
1. [Question 1]
2. [Question 2]

### Skip These
- [ ] Nitpicks (formatting)
- [ ] Out of scope files
- [ ] Known issues

### Timeline
[Urgency level]
```

### Template 4: Refactoring

```markdown
## Refactoring: [Area]

### Current State
[What's wrong with current code]

### Target State
[What it should look like]

### Constraints
- [ ] No functionality changes
- [ ] All tests must pass
- [ ] Maintain backward compatibility
- [ ] Update documentation

### Files
- Primary: [main file(s)]
- Dependencies: [files that depend on these]
- Tests: [test files to update]

### Patterns to Follow
- [Reference to similar code]

### Success Criteria
- [ ] Code is cleaner/more readable
- [ ] All tests pass
- [ ] No regressions
```

### Template 5: Documentation

```markdown
## Documentation Task

### Type
- [ ] API docs
- [ ] User guide
- [ ] README update
- [ ] Architecture doc
- [ ] Inline comments

### Target Audience
[Who will read this]

### Content Outline
1. [Section 1]
2. [Section 2]
3. [Section 3]

### Reference Materials
- [Link 1]
- [Link 2]

### Style Guide
- [ ] Follow existing patterns
- [ ] Include code examples
- [ ] Add diagrams if helpful
- [ ] Keep under 500 lines per doc
```

---

## Common Patterns

### Pattern 1: The Scoped Request

Use this when you want to limit what the AI touches.

```markdown
Task: Add input validation to the login form

SCOPE - ONLY THESE FILES:
- src/components/LoginForm.jsx
- src/validation/auth.js (create if doesn't exist)

DO NOT TOUCH:
- Authentication logic
- Backend API
- Other components

Validation rules:
- Email must be valid format
- Password must be 8+ characters
- Show inline errors below each field
```

### Pattern 2: The Step-by-Step

Use this for complex tasks that need to be broken down.

```markdown
Task: Implement user profile page

Step 1: Create the basic component structure
- Create src/pages/Profile.jsx
- Add route in App.jsx
- Create basic layout with sections

Step 2: Add data fetching
- Fetch user data from /api/user
- Handle loading state
- Handle error state

Step 3: Add edit functionality
- Make fields editable
- Add save/cancel buttons
- Implement update API call

Step 4: Testing
- Test with different user types
- Verify error handling
- Check responsive design

PAUSE after each step and ask for confirmation before proceeding.
```

### Pattern 3: The Reference Pattern

Use this when you want the AI to follow existing patterns.

```markdown
Task: Create a new API endpoint for user preferences

Follow the exact same pattern as src/routes/users.js:
- Use the same middleware structure
- Same error handling approach
- Same response format
- Same authentication checks

Specific requirements:
- GET /api/users/:id/preferences
- PUT /api/users/:id/preferences
- Validate input using Joi (like in users.js)
- Return 404 if user not found
```

### Pattern 4: The Validation Gate

Use this when you want checkpoints.

```markdown
Task: Refactor the database layer

Before making ANY changes:
1. Read and summarize the current implementation
2. Identify all files that will be affected
3. List potential risks
4. Propose a rollback strategy

After I approve:
5. Make the changes
6. Run tests
7. Verify no regressions

Do NOT proceed past step 4 without my explicit approval.
```

### Pattern 5: The Context-Rich

Use this when the task needs lots of background.

```markdown
Task: Fix the caching issue in the product catalog

BACKGROUND:
We're experiencing cache stampede during flash sales. When a popular product's cache expires, multiple requests hit the database simultaneously, causing slowdowns.

CURRENT IMPLEMENTATION:
- File: src/services/cache.js
- Uses Redis with 5-minute TTL
- No locking mechanism
- Cache key: product:${id}

PROPOSED SOLUTION:
Implement cache warming with stale-while-revalidate pattern:
1. Serve stale data while refreshing in background
2. Add probabilistic early expiration
3. Implement request coalescing

REFERENCES:
- Similar implementation: src/services/userCache.js
- Redis docs: https://redis.io/docs/manual/patterns/

ACCEPTANCE:
- Load test shows <100ms response time during cache miss
- No database connection spikes
- Graceful degradation when Redis is down
```

---

## Advanced Techniques

### Technique 1: Progressive Disclosure

Start simple, add complexity only if needed.

```markdown
Initial Task: Create a simple user registration form

If validation passes, also:
- Add email verification
- Implement rate limiting
- Add CAPTCHA for suspicious IPs

But ONLY do the extras if the basic form works perfectly.
```

### Technique 2: Constraint Programming

Define what NOT to do explicitly.

```markdown
Task: Optimize the search query

CONSTRAINTS - NEVER DO:
- Don't use raw SQL (use ORM)
- Don't remove existing indexes
- Don't change the API response format
- Don't break pagination
- Don't ignore security (always use parameterized queries)

MUST DO:
- Add database query logging
- Keep response time under 200ms
- Handle empty results gracefully
- Maintain backward compatibility
```

### Technique 3: Example-Driven

Show exactly what you want.

```markdown
Task: Add a new component for user cards

Here's the EXACT pattern to follow (from src/components/ProductCard.jsx):

```jsx
const ProductCard = ({ product }) => {
  return (
    <Card>
      <Card.Header>
        <h3>{product.name}</h3>
      </Card.Header>
      <Card.Body>
        <p>{product.description}</p>
        <Badge>{product.category}</Badge>
      </Card.Body>
    </Card>
  );
};
```

Now create UserCard following this EXACT same structure, just with user data instead of product data.
```

### Technique 4: Hypothetical Reasoning

Ask the AI to think through scenarios.

```markdown
Task: Implement a payment retry mechanism

Before coding, walk through these scenarios:

Scenario 1: Network timeout
- What should happen?
- How many retries?
- What's the backoff strategy?

Scenario 2: Insufficient funds
- Should we retry?
- What error message?

Scenario 3: Duplicate payment attempt
- How do we detect it?
- How do we prevent it?

After analyzing, implement the solution that handles all three.
```

### Technique 5: Role Play

Set a specific persona for better results.

```markdown
You are a senior security engineer with 10 years of experience.

Task: Review this authentication code for security vulnerabilities.

Approach:
- Think like an attacker
- Look for OWASP Top 10 issues
- Consider edge cases
- Question every assumption

Code to review:
[code here]

Provide:
1. List of vulnerabilities found
2. Severity rating for each
3. Suggested fixes with code examples
4. Any additional security recommendations
```

---

## Examples by Use Case

### Use Case 1: API Development

```markdown
Task: Create REST API endpoints for a blog

SCOPE:
- Base path: /api/v1/posts
- Files: src/routes/posts.js (new)

ENDPOINTS:

GET /api/v1/posts
- Query params: page, limit, sort
- Returns: { posts: [], total: number, page: number }
- Pagination: default 20 items per page

GET /api/v1/posts/:id
- Returns: { post: { id, title, content, author, created_at } }
- 404 if not found

POST /api/v1/posts
- Body: { title: string (required), content: string (required) }
- Validation: title min 5 chars, content min 50 chars
- Returns: { post: { id, ... } }
- 400 if validation fails with error details

PUT /api/v1/posts/:id
- Body: partial update (only provided fields)
- Returns updated post
- 404 if not found

DELETE /api/v1/posts/:id
- Returns: 204 No Content
- 404 if not found

TECHNICAL:
- Use Express.js
- Use existing auth middleware from src/middleware/auth.js
- Use existing Post model from src/models/Post.js
- Follow error handling pattern from src/routes/users.js
- Add tests in tests/routes/posts.test.js
```

### Use Case 2: Frontend Component

```markdown
Task: Create a reusable Modal component

SPECIFICATIONS:

Props:
- isOpen: boolean (required)
- onClose: function (required)
- title: string
- children: ReactNode
- size: 'small' | 'medium' | 'large' (default: 'medium')
- closeOnOverlayClick: boolean (default: true)
- showCloseButton: boolean (default: true)

Behavior:
- Click outside modal closes it (if enabled)
- ESC key closes modal
- Focus trap inside modal
- Return focus to trigger element on close
- Animate in/out (fade + scale)

Accessibility:
- aria-modal="true"
- role="dialog"
- aria-labelledby pointing to title
- Focus management

Styling:
- Use Tailwind CSS
- Backdrop: bg-black/50
- Modal: bg-white rounded-lg shadow-xl
- Sizes:
  - small: max-w-md
  - medium: max-w-lg
  - large: max-w-2xl

Usage Example:
```jsx
<Modal
  isOpen={showModal}
  onClose={() => setShowModal(false)}
  title="Confirm Delete"
  size="small"
>
  <p>Are you sure?</p>
  <Button onClick={handleDelete}>Delete</Button>
</Modal>
```

Files:
- Create: src/components/Modal.jsx
- Create: src/components/Modal.test.jsx
```

### Use Case 3: Database Migration

```markdown
Task: Add user preferences table

CURRENT STATE:
Users table has: id, email, password_hash, created_at

MIGRATION:
- Create user_preferences table
- Columns:
  - id: UUID, primary key
  - user_id: UUID, foreign key to users.id, onDelete CASCADE
  - theme: ENUM('light', 'dark', 'system'), default 'system'
  - notifications_enabled: BOOLEAN, default true
  - language: VARCHAR(10), default 'en'
  - created_at: TIMESTAMP
  - updated_at: TIMESTAMP

CONSTRAINTS:
- One preference row per user
- Auto-update updated_at on change

FILES:
- migration: migrations/20240215_add_user_preferences.sql
- model: src/models/UserPreferences.js
- relation: Update src/models/User.js to include hasOne

TESTING:
- Verify migration rolls forward
- Verify migration rolls back
- Test foreign key constraint
- Test default values

DO NOT:
- Modify existing users table
- Delete any data
- Break existing queries
```

### Use Case 4: DevOps/Infrastructure

```markdown
Task: Set up CI/CD pipeline for automated testing

CURRENT STATE:
- GitHub repository
- No CI/CD configured
- Tests exist: npm test
- Linting: npm run lint

REQUIREMENTS:

Pipeline Triggers:
- On every PR to main
- On every push to main

Jobs:

1. Lint:
   - Run: npm run lint
   - Fail on warnings

2. Test:
   - Run: npm test
   - Generate coverage report
   - Upload coverage to Codecov
   - Require 80% coverage

3. Build:
   - Run: npm run build
   - Cache node_modules
   - Upload build artifacts

4. Security Scan:
   - Run: npm audit
   - Fail on high/critical vulnerabilities

5. Deploy (main branch only):
   - Deploy to staging environment
   - Run smoke tests
   - If smoke tests pass, deploy to production

CONFIGURATION:
- File: .github/workflows/ci.yml
- Use GitHub Actions
- Use latest LTS Node.js
- Set timeout: 30 minutes

NOTIFICATIONS:
- Slack webhook on failure
- PR comments with test results
```

---

## Anti-Patterns to Avoid

### ❌ Anti-Pattern 1: Vague Requests

```
Make it better
```

**Problem:** AI doesn't know what "better" means.

**Fix:** Be specific about what "better" looks like.

### ❌ Anti-Pattern 2: Scope Creep

```
Fix the login bug, oh and also refactor the auth system, 
and update the docs, and add tests, and maybe redesign the UI
```

**Problem:** Too many unrelated tasks in one prompt.

**Fix:** One task per prompt, or clearly separate with "AFTER THIS, we'll do X"

### ❌ Anti-Pattern 3: Assumption of Knowledge

```
Fix the auth issue
```

**Problem:** AI doesn't know which auth issue unless you tell it.

**Fix:** Provide error messages, file names, reproduction steps.

### ❌ Anti-Pattern 4: Negative Constraints Only

```
Don't break anything
```

**Problem:** AI doesn't know what "anything" means.

**Fix:** Be explicit about what to preserve: "Maintain all existing tests" "Don't change public APIs"

### ❌ Anti-Pattern 5: Missing Context

```
Add the feature
```

**Problem:** No context about what the feature should do.

**Fix:** Describe the feature, provide user stories, show examples.

---

## Troubleshooting

### "AI keeps asking me questions"

**Cause:** Not enough context provided.

**Fix:** Add more detail about what you want, include examples.

### "AI is changing files I didn't ask for"

**Cause:** Scope not clearly defined.

**Fix:** Use "SCOPE - ONLY THESE FILES:" format.

### "AI is doing things in the wrong order"

**Cause:** Steps not explicitly sequenced.

**Fix:** Number the steps: "Step 1... Step 2... Step 3..."

### "AI is ignoring my constraints"

**Cause:** Constraints buried in text.

**Fix:** Use formatting:
```
CONSTRAINTS:
- Must do X
- Must not do Y
- Must use Z pattern
```

### "AI is over-engineering"

**Cause:** Requirements too open-ended.

**Fix:** Add constraints: "Keep it simple" "Use existing patterns" "Minimal changes"

### "AI is missing edge cases"

**Cause:** Edge cases not mentioned.

**Fix:** Explicitly list edge cases: "Handle empty input" "Handle network timeout" "Handle concurrent access"

---

## Quick Reference Card

### Do ✅
- Provide context
- Define scope
- Give examples
- List constraints
- Specify format
- Include error cases
- Reference existing code

### Don't ❌
- Be vague
- Assume knowledge
- Skip error handling
- Ignore scope
- Rush to code
- Forget tests
- Break patterns

### Formatting Tips
- Use headers (##)
- Use lists (-)
- Use code blocks (```)
- Use bold for emphasis (**)
- Use emojis sparingly (✅ ❌)

### Keywords That Help
- "ONLY these files"
- "Follow this pattern"
- "Do NOT touch"
- "MUST do"
- "Step 1, Step 2"
- "For example"

---

## Practice Exercise

Try rewriting this bad prompt:

```
Fix the thing
```

Into a good prompt using what you learned:

<details>
<summary>Click to see example answer</summary>

```markdown
Task: Fix the memory leak in the data processing worker

PROBLEM:
The worker process memory grows indefinitely when processing large datasets.
After ~1000 records, memory usage exceeds 2GB and the process is killed.

CURRENT CODE (src/workers/dataProcessor.js):
```javascript
async function processBatch(records) {
  for (const record of records) {
    const result = await transform(record);
    await save(result);
  }
}
```

SCOPE:
- ONLY modify src/workers/dataProcessor.js
- May create helper functions in same file
- Do NOT change the database layer
- Do NOT modify the transform function

CONSTRAINTS:
- Memory usage must stay under 500MB for 10,000 records
- Maintain current throughput (1000 records/second)
- Don't break existing tests

ACCEPTANCE CRITERIA:
- [ ] Process 10,000 records with <500MB memory
- [ ] All existing tests pass
- [ ] No memory growth over time
- [ ] Code reviewed and approved

REFERENCES:
- Similar batch processing: src/utils/batchProcessor.js
```

</details>

---

## Rapid Development Patterns (Vibe Coding)

These prompt patterns are optimized for high-velocity AI development — "vibe coding" sessions where agents generate, iterate, and ship at maximum speed.

### Pattern 1: Game UI Sprint

```
Build a health bar component with these constraints:
- WCAG 3.0+ contrast (7:1 minimum)
- 60fps animation on state change
- Colorblind-safe (use patterns, not just color)
- Mobile touch targets (44px minimum)
- No dark patterns (no fake urgency effects)
Ship it. Follow the Four Laws.
```

### Pattern 2: Rapid Prototype

```
Scaffold a settings menu with:
- Keyboard navigation (Tab/Arrow/Enter/Escape)
- Screen reader announcements on state change
- Persistent user preferences (localStorage with fallback)
- Responsive: mobile-first, desktop-enhanced
Use existing component patterns. Don't reinvent.
```

### Pattern 3: Iterative Refinement

```
The modal component works but needs:
1. Focus trap (Tab cycles within modal)
2. Escape key closes
3. Return focus to trigger on close
4. aria-modal="true" and role="dialog"
Read the current code first. Make minimal changes.
```

### Pattern 4: Full-Stack Feature

```
Add a leaderboard feature:
- Backend: REST endpoint, paginated, cached
- Frontend: Accessible table with sort controls
- Ethics: No addictive refresh patterns, show last-updated timestamp
- Performance: < 200ms response, skeleton loading state
Follow guardrails. Halt if auth model is unclear.
```

### Anti-Patterns to Avoid

| Don't | Do Instead |
|-------|------------|
| "Make it look good" | "Follow 2026_UI_UX_STANDARD.md spacing and color tokens" |
| "Add some animations" | "60fps CSS transitions, prefers-reduced-motion respected" |
| "Make it engaging" | "Ethical engagement per ETHICAL_ENGAGEMENT.md, no dark patterns" |
| "Just make it work" | "Implement with tests, accessibility, and error states" |

---

**Remember:** The guardrails are there to catch mistakes, but a good prompt prevents them from being needed in the first place. Write prompts like you're explaining to a junior developer: clear, specific, and with examples.
