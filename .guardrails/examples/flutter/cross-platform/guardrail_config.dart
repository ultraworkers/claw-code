/// Agent Guardrails Configuration for Flutter
///
/// Central configuration for AI-generated Flutter code.
/// AI agents: import this file and check risk levels before
/// performing any guarded operation.
library;

/// Risk levels for AI agent decision matrix
enum RiskLevel { low, medium, high }

/// Actions agents can take at each risk level
enum AgentAction { proceed, askUser, halt }

/// Central guardrail configuration
class GuardrailConfig {
  /// Decision matrix: determines agent behavior by risk level
  static AgentAction actionFor(RiskLevel risk) {
    return switch (risk) {
      RiskLevel.low => AgentAction.proceed,
      RiskLevel.medium => AgentAction.askUser,
      RiskLevel.high => AgentAction.halt,
    };
  }

  /// Operations and their risk classifications
  static final Map<String, RiskLevel> operationRisks = {
    // LOW — agents decide autonomously
    'ui_component': RiskLevel.low,
    'styling': RiskLevel.low,
    'test_writing': RiskLevel.low,
    'documentation': RiskLevel.low,
    // MEDIUM — ask before proceeding
    'new_dependency': RiskLevel.medium,
    'api_change': RiskLevel.medium,
    'config_change': RiskLevel.medium,
    // HIGH — halt and confirm
    'auth_change': RiskLevel.high,
    'payment': RiskLevel.high,
    'data_model': RiskLevel.high,
    'infrastructure': RiskLevel.high,
    'deletion': RiskLevel.high,
  };

  /// Check if an operation requires human approval
  static bool requiresApproval(String operation) {
    final risk = operationRisks[operation] ?? RiskLevel.high;
    return actionFor(risk) != AgentAction.proceed;
  }

  /// Performance budgets
  static const int targetFps = 60;
  static const Duration maxFrameTime = Duration(microseconds: 16667); // 60fps
  static const Duration maxBuildTime = Duration(milliseconds: 100);

  /// Accessibility minimums
  static const double minContrastRatio = 7.0; // WCAG AAA
  static const double minTouchTarget = 44.0;  // 44x44 logical pixels
  static const double minFocusIndicator = 3.0; // 3px outline
}
