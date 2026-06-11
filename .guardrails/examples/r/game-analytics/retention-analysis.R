# Player Retention Curve Analysis
# Demonstrates: Survival analysis, cohort tracking, colorblind-safe visualization, Hick's Law menus

library(ggplot2)
library(dplyr)
library(survival)
library(lubricate)

# Colorblind-safe palette
colorblind_palette <- list(
  sequential = c("#440154", "#443782", "#3a608b", "#318755", "#27b57e"),  # viridis
  categorical = c("#0077BD", "#E66302", "#7C87BB", "#D6588B", "#8AB68E"),  # colorblind-universal
  cividis = c("#002495", "#005CA8", "#297EA7", "#58999D", "#8AB68E")
)

# Hick's Law compliant menu (5 ± 2 items)
retention_menu_items <- c("D1", "D3", "D7", "D14", "D30")
stopifnot(length(retention_menu_items) >= 3 && length(retention_menu_items) <= 7)

# Sample retention data
generate_retention_data <- function(n_players = 1000, seed = 42) {
  set.seed(seed)

  players <- data.frame(
    player_id = paste0("P", 1:n_players),
    install_date = sample(seq(as.Date('2025-01-01'), as.Date('2025-12-31'), by='day'), n_players, replace = TRUE),
    first_purchase = sample(c(0, 1), n_players, prob = c(0.7, 0.3), replace = TRUE),
    platform = sample(c("mobile", "desktop", "console"), n_players, prob = c(0.5, 0.3, 0.2), replace = TRUE)
  )

  # Simulate retention decay (exponential)
  retention_rates <- c(
    day_1 = 0.95,
    day_3 = 0.80,
    day_7 = 0.65,
    day_14 = 0.50,
    day_30 = 0.30
  )

  players$retention_d1 <- sample(c(TRUE, FALSE), n_players, prob = c(retention_rates["day_1"], 1 - retention_rates["day_1"]), replace = TRUE)
  players$retention_d3 <- sample(c(TRUE, FALSE), n_players, prob = c(retention_rates["day_3"], 1 - retention_rates["day_3"]), replace = TRUE)
  players$retention_d7 <- sample(c(TRUE, FALSE), n_players, prob = c(retention_rates["day_7"], 1 - retention_rates["day_7"]), replace = TRUE)
  players$retention_d14 <- sample(c(TRUE, FALSE), n_players, prob = c(retention_rates["day_14"], 1 - retention_rates["day_14"]), replace = TRUE)
  players$retention_d30 <- sample(c(TRUE, FALSE), n_players, prob = c(retention_rates["day_30"], 1 - retention_rates["day_30"]), replace = TRUE)

  players
}

# Cohort analysis
analyze_cohorts <- function(data, cohort_var = "platform") {
  cohorts <- data |>
    group_by(.data[[cohort_var]]) |>
    summarise(
      n = n(),
      d1_retention = mean(retention_d1),
      d3_retention = mean(retention_d3),
      d7_retention = mean(retention_d7),
      d14_retention = mean(retention_d14),
      d30_retention = mean(retention_d30)
    )

  cohorts
}

# Survival analysis
fit_survival_model <- function(data) {
  # Create survival object (time = days, event = lost player)
  survival_time <- data |>
    mutate(
      days_active = case_when(
        retention_d30 == TRUE ~ 30,
        retention_d14 == TRUE ~ 14,
        retention_d7 == TRUE ~ 7,
        retention_d3 == TRUE ~ 3,
        retention_d1 == TRUE ~ 1,
        TRUE ~ 0
      ),
      event = case_when(
        retention_d30 == FALSE ~ 1,
        TRUE ~ 0
      )
    )

  # Cox proportional hazards model
  cox_model <- coxph(Surv(days_active, event) ~ first_purchase + platform, data = survival_time)

  cox_model
}

# Retention curve visualization
plot_retention_curve <- function(data, palette = colorblind_palette$sequential) {
  retention_summary <- data |>
    summarise(
      d1 = mean(retention_d1),
      d3 = mean(retention_d3),
      d7 = mean(retention_d7),
      d14 = mean(retention_d14),
      d30 = mean(retention_d30)
    ) |>
    pivot_longer(cols = everything(), names_to = "day", values_to = "rate")

  ggplot(retention_summary, aes(x = day, y = rate)) +
    geom_line(color = palette[1], linewidth = 2) +
    geom_point(color = palette[2], size = 4, fill = "white") +
    geom_smooth(method = "loess", color = palette[3], se = TRUE, fill = palette[4]) +
    theme_minimal() +
    labs(
      title = "Player Retention Curve",
      subtitle = "N = 1,000 players",
      x = "Day",
      y = "Retention Rate"
    ) +
    scale_y_continuous(labels = c(0, 0.25, 0.5, 0.75, 1.0))
}

# Cohort comparison visualization
plot_cohort_comparison <- function(cohort_data, palette = colorblind_palette$categorical) {
  cohort_long <- cohort_data |>
    pivot_longer(
      cols = c(d1_retention, d3_retention, d7_retention, d14_retention, d30_retention),
      names_to = "day",
      values_to = "retention"
    ) |>
    mutate(day = substr(day, 2, 4))  # Remove "d_" prefix

  ggplot(cohort_long, aes(x = day, y = retention, fill = cohort)) +
    geom_col(position = "dodge", fill = palette) +
    theme_minimal() +
    labs(
      title = "Retention by Platform Cohort",
      x = "Day",
      y = "Retention Rate"
    ) +
    scale_y_continuous(labels = c(0, 0.25, 0.5, 0.75, 1.0))
}

# Hick's Law menu display
display_retention_menu <- function() {
  menu <- paste(retention_menu_items, collapse = " < ")
  cat("Retention Menu (Hick's Law: 5 items):\n")
  cat(menu, "\n\n")
}

# Main analysis
run_retention_analysis <- function() {
  set.seed(42)

  # Generate data
  players <- generate_retention_data(n_players = 1000)

  cat("=== Player Retention Analysis ===\n\n")
  cat("N =", nrow(players), "players\n\n")

  # Display Hick's Law menu
  display_retention_menu()

  # Overall retention curve
  cat("Retention Curve:\n")
  print(plot_retention_curve(players))

  # Cohort analysis
  cat("\nCohort Analysis (by Platform):\n")
  cohorts <- analyze_cohorts(players, "platform")
  print(cohorts)

  # Cohort visualization
  cat("\nCohort Comparison:\n")
  print(plot_cohort_comparison(cohorts))

  # Survival model
  cat("\nSurvival Analysis (Cox Model):\n")
  cox_model <- fit_survival_model(players)
  print(summary(cox_model))

  # A/B Test: First Purchase Impact
  cat("\nFirst Purchase Impact on Retention:\n")
  purchase_impact <- players |>
    group_by(first_purchase) |>
    summarise(
      d7_retention = mean(retention_d7),
      d30_retention = mean(retention_d30)
    )
  print(purchase_impact)

  # Retention heatmap (colorblind-safe)
  cat("\nRetention Heatmap:\n")
  heatmap_data <- players |>
    group_by(platform) |>
    summarise(
      d1 = mean(retention_d1),
      d3 = mean(retention_d3),
      d7 = mean(retention_d7),
      d14 = mean(retention_d14),
      d30 = mean(retention_d30)
    )

  heatmap_long <- heatmap_data |>
    pivot_longer(cols = c(d1, d3, d7, d14, d30), names_to = "day", values_to = "rate") |>
    mutate(day = substr(day, 2, 4))

  ggplot(heatmap_long, aes(x = day, y = platform, fill = rate)) +
    geom_tile(fill = colorblind_palette$sequential) +
    theme_minimal() +
    labs(
      title = "Retention Heatmap by Platform",
      x = "Day",
      y = "Platform"
    ) +
    scale_fill_gradient(low = colorblind_palette$sequential[1], high = colorblind_palette$sequential[5])
}

# Run analysis
if (TRUE) {
  run_retention_analysis()
}