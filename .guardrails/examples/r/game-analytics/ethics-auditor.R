# Dark Pattern Detection & Engagement Ethics Auditor
# Demonstrates: Dark pattern identification, monetization transparency, ethical scoring, Hick's Law menus

library(ggplot2)
library(dplyr)
library(jsonlite)

# Colorblind-safe palette
colorblind_palette <- list(
  sequential = c("#440154", "#443782", "#3a608b", "#318755", "#27b57e"),  # viridis
  categorical = c("#0077BD", "#E66302", "#7C87BB", "#D6588B", "#8AB68E"),  # colorblind-universal
  cividis = c("#002495", "#005CA8", "#297EA7", "#58999D", "#8AB68E")
)

# Hick's Law compliant ethics menu (5 ± 2 items)
ethics_menu_items <- c("Autonomy", "Transparency", "Wellbeing", "Fairness", "Overall")
stopifnot(length(ethics_menu_items) >= 3 && length(ethics_menu_items) <= 7)

# Dark pattern types
dark_patterns <- list(
  forced_action = "Mandatory actions for progression",
  hidden_cost = " Costs not disclosed upfront",
  infinite_loop = " Engagement loops without exit",
  false_urgency = "Artificial time pressure",
  data_harvesting = "Excessive data collection"
)

# Ethics score structure
EthicsScore <- list(
  autonomy = 0.0,      # User choice preservation
  transparency = 0.0,  # Clear information disclosure
  wellbeing = 0.0,     # User mental health consideration
  fairness = 0.0       # Balanced monetization
)

# Dark pattern detection functions
detect_dark_patterns <- function(game_features) {
  patterns_found <- list()

  # Forced action detection (resource density too low = forced purchases)
  if (game_features$resource_density < 0.1) {
    patterns_found <- append(patterns_found, list("forced_action"))
  }

  # Hidden cost detection (treasure rate imbalanced)
  if (game_features$treasure_rate < 0.05 && game_features$enemy_rate > 0.8) {
    patterns_found <- append(patterns_found, list("hidden_cost"))
  }

  # Infinite loop detection (complexity extreme)
  if (game_features$complexity > 0.95) {
    patterns_found <- append(patterns_found, list("infinite_loop"))
  }

  # False urgency detection (time limits aggressive)
  if (game_features$time_limit_avg < 60) {  # seconds
    patterns_found <- append(patterns_found, list("false_urgency"))
  }

  # Data harvesting detection (tracking excessive)
  if (game_features$data_collection_events > 100) {
    patterns_found <- append(patterns_found, list("data_harvesting"))
  }

  patterns_found
}

# Calculate ethics score
calculate_ethics_score <- function(patterns, engagement_data) {
  pattern_penalty <- length(patterns) * 0.15

  avg_engagement <- mean(engagement_data$engagement_score)

  score <- list(
    autonomy = 1.0 - pattern_penalty,
    transparency = if ("hidden_cost" %in% patterns) 0.5 else 0.9,
    wellbeing = if ("infinite_loop" %in% patterns) 0.4 else 0.8,
    fairness = avg_engagement
  )

  score$overall <- (score$autonomy + score$transparency + score$wellbeing + score$fairness) / 4.0

  score
}

# Ethics status classification
get_ethics_status <- function(score) {
  if (score$overall >= 0.8) {
    "ETHICAL"
  } else if (score$overall >= 0.5) {
    "WARNING"
  } else {
    "UNETHICAL"
  }
}

# Generate ethics report
generate_ethics_report <- function(score, patterns) {
  status <- get_ethics_status(score)
  color_index <- ((score$overall * length(colorblind_palette$sequential)).toInt()).min(length(colorblind_palette$sequential) - 1)
  color <- colorblind_palette$sequential[color_index + 1]

  report <- list(
    overall_score = score$overall,
    status = status,
    color = color,
    patterns_detected = length(patterns),
    patterns_list = patterns,
    component_scores = score,
    recommendation = get_recommendation(score)
  )

  report
}

# Recommendation generator
get_recommendation <- function(score) {
  if (score$overall >= 0.8) {
    "Maintain ethical design"
  } else if (score$overall >= 0.5) {
    "Review dark patterns and adjust"
  } else {
    "Immediate ethics audit required - consider redesign"
  }
}

# Ethics radar chart visualization
plot_ethics_radar <- function(score, palette = colorblind_palette$categorical) {
  radar_data <- data.frame(
    metric = ethics_menu_items,  # Hick's Law: 5 items
    score = c(score$autonomy, score$transparency, score$wellbeing, score$fairness, score$overall),
    color = palette
  )

  ggplot(radar_data, aes(x = metric, y = score, fill = metric)) +
    geom_col(fill = palette) +
    theme_minimal() +
    labs(
      title = "Ethics Audit Radar",
      subtitle = paste("Status:", get_ethics_status(score)),
      x = "Ethics Component",
      y = "Score"
    ) +
    scale_y_continuous(limits = c(0, 1.0))
}

# Dark pattern frequency visualization
plot_pattern_frequency <- function(patterns, palette = colorblind_palette$sequential) {
  pattern_freq <- data.frame(
    pattern = names(patterns),
    count = unlist(patterns)
  )

  ggplot(pattern_freq, aes(x = pattern, y = count, fill = pattern)) +
    geom_col(fill = palette) +
    theme_minimal() +
    labs(
      title = "Dark Pattern Frequency",
      x = "Pattern Type",
      y = "Occurrences"
    )
}

# Monetization transparency analyzer
analyze_monetization_transparency <- function(monetization_events) {
  total_events <- nrow(monetization_events)
  hidden_costs <- monetization_events |>
    filter(is_hidden_cost == TRUE) |>
    nrow()

  transparency_score <- 1.0 - (hidden_costs / total_events)

  list(
    score = transparency_score,
    hidden_count = hidden_costs,
    total = total_events
  )
}

# Engagement ethics analysis
analyze_engagement_ethics <- function(session_data) {
  # Detect addiction loops (session length extreme)
  addiction_risk <- session_data |>
    mutate(
      addiction_flag = case_when(
        session_duration > 4 * 60 ~ TRUE,  # 4+ hours
        sessions_per_day > 10 ~ TRUE,
        TRUE ~ FALSE
      )
    ) |>
    summarise(
      addiction_rate = mean(addiction_flag),
      avg_session = mean(session_duration),
      max_session = max(session_duration)
    )

  addiction_risk
}

# Main ethics audit
run_ethics_audit <- function() {
  cat("=== Game Ethics Audit ===\n\n")

  # Display Hick's Law menu
  cat("Ethics Menu (Hick's Law: 5 items):\n")
  cat(paste(ethics_menu_items, collapse = " < "), "\n\n")

  # Sample game features
  game_features <- list(
    resource_density = 0.08,      # Low = forced purchases
    treasure_rate = 0.03,         # Low
    enemy_rate = 0.85,            # High = difficulty pressure
    complexity = 0.97,            # Extreme = infinite loop
    time_limit_avg = 45,          # Seconds = false urgency
    data_collection_events = 150  # Excessive tracking
  )

  # Detect dark patterns
  patterns <- detect_dark_patterns(game_features)
  cat("Dark Patterns Detected:", length(patterns), "\n")
  cat(paste(patterns, collapse = ", "), "\n\n")

  # Sample engagement data
  engagement_data <- data.frame(
    player_id = 1:100,
    engagement_score = rnorm(100, mean = 0.6, sd = 0.2)
  )

  # Calculate ethics score
  score <- calculate_ethics_score(patterns, engagement_data)
  cat("Ethics Score: ", score$overall, "\n")
  cat("Status: ", get_ethics_status(score), "\n\n")

  # Component scores
  cat("Component Scores:\n")
  cat("  Autonomy: ", score$autonomy, "\n")
  cat("  Transparency: ", score$transparency, "\n")
  cat("  Wellbeing: ", score$wellbeing, "\n")
  cat("  Fairness: ", score$fairness, "\n")
  cat("  Overall: ", score$overall, "\n\n")

  # Generate report
  report <- generate_ethics_report(score, patterns)
  cat("Recommendation:", report$recommendation, "\n\n")

  # Visualizations
  cat("Ethics Radar Chart:\n")
  print(plot_ethics_radar(score))

  # Monetization transparency
  cat("\nMonetization Transparency Analysis:\n")
  monetization_events <- data.frame(
    event_id = 1:50,
    price = rand(50, 1, 20),
    is_hidden_cost = sample(c(TRUE, FALSE), 50, prob = c(0.1, 0.9), replace = TRUE)
  )

  transparency <- analyze_monetization_transparency(monetization_events)
  cat("Transparency Score:", transparency$score, "\n")
  cat("Hidden Costs Found:", transparency$hidden_count, "\n")

  # Engagement ethics
  cat("\nEngagement Ethics Analysis:\n")
  session_data <- data.frame(
    player_id = 1:100,
    session_duration = rand(100, 30, 300),
    sessions_per_day = rand(100, 1, 15)
  )

  addiction <- analyze_engagement_ethics(session_data)
  cat("Addiction Risk Rate:", addiction$addiction_rate * 100, "%\n")
  cat("Average Session:", addiction$avg_session, "minutes\n")
  cat("Max Session:", addiction$max_session, "minutes\n")
}

# Run ethics audit
if (TRUE) {
  run_ethics_audit()
}