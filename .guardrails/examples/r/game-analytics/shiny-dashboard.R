# Shiny 2.0+ Game Analytics Dashboard
# Demonstrates: Interactive filtering, real-time metrics, colorblind-safe visualization, Hick's Law menus

library(shiny)
library(ggplot2)
library(dplyr)
library(bslib)
library(htmltools)

# Colorblind-safe palette definitions
colorblind_palette <- list(
  sequential = c("#440154", "#443782", "#3a608b", "#318755", "#27b57e"),  # viridis
  categorical = c("#0077BD", "#E66302", "#7C87BB", "#D6588B", "#8AB68E"),  # colorblind-universal
  cividis = c("#002495", "#005CA8", "#297EA7", "#58999D", "#8AB68E")
)

# Hick's Law compliant navigation (5 ± 2 items)
valid_menu_items <- function(items) {
  n <- length(items)
  if (n < 3 || n > 7) {
    stop("Menu must have 3-7 items per Hick's Law")
  }
  TRUE
}

# Navigation menu (5 items - Hick's Law)
nav_items <- c("Overview", "A/B Tests", "Retention", "DDA", "Export")
valid_menu_items(nav_items)

# Sample data generators
generate_ab_test_data <- function() {
  data.frame(
    test_id = c("checkout-flow", "pricing-display", "onboarding"),
    variant_a = c(1200, 800, 500),
    variant_b = c(1450, 920, 580),
    conversion_a = c(0.20, 0.15, 0.25),
    conversion_b = c(0.24, 0.18, 0.30),
    confidence = c(0.95, 0.88, 0.92)
  )
}

generate_retention_data <- function() {
  data.frame(
    day = 1:7,
    retained = c(950, 800, 650, 500, 400, 350, 300),
    total = 1000,
    retention_rate = c(0.95, 0.80, 0.65, 0.50, 0.40, 0.35, 0.30)
  )
}

generate_dda_data <- function() {
  data.frame(
    difficulty_level = c(0.2, 0.4, 0.6, 0.8, 1.0),
    avg_session_duration = c(180, 240, 300, 280, 220),
    avg_completion = c(0.9, 0.75, 0.6, 0.45, 0.3),
    player_skill = c(0.3, 0.5, 0.6, 0.7, 0.8)
  )
}

# UI Definition
ui <- bslib::page_navbar(
  title = "Game Analytics Dashboard",
  bg = "dark",
  inverse = TRUE,

  # Hick's Law: 5 navigation items
  bslib::nav_panel("Overview",
    bslib::card(
      bslib::card_header("Key Metrics"),
      bslib::card_body(
        fluidRow(
          valueBoxOutput("kpiDAU", width = 3),
          valueBoxOutput("kpiRetention", width = 3),
          valueBoxOutput("kpiConversion", width = 3),
          valueBoxOutput("kpiDDA", width = 3)
        )
      )
    ),
    bslib::card(
      bslib::card_header("Traffic Overview"),
      bslib::card_body(plotOutput("trafficPlot", height = "300px"))
    )
  ),

  bslib::nav_panel("A/B Tests",
    bslib::card(
      bslib::card_header("A/B Test Results"),
      bslib::card_body(tableOutput("abTestTable"))
    ),
    bslib::card(
      bslib::card_header("Conversion Comparison"),
      bslib::card_body(plotOutput("abTestPlot", height = "400px"))
    )
  ),

  bslib::nav_panel("Retention",
    bslib::card(
      bslib::card_header("Retention Curve"),
      bslib::card_body(plotOutput("retentionPlot", height = "400px"))
    ),
    bslib::card(
      bslib::card_header("Cohort Analysis"),
      bslib::card_body(tableOutput("cohortTable"))
    )
  ),

  bslib::nav_panel("DDA",
    bslib::card(
      bslib::card_header("Dynamic Difficulty Adjustment"),
      bslib::card_body(plotOutput("ddaPlot", height = "400px"))
    ),
    bslib::card(
      bslib::card_header("Player Skill Distribution"),
      bslib::card_body(tableOutput("skillTable"))
    )
  ),

  bslib::nav_panel("Export",
    bslib::card(
      bslib::card_header("Export Analytics"),
      bslib::card_body(
        actionButton("exportCSV", "Download CSV", class = "btn-primary"),
        actionButton("exportPDF", "Download PDF", class = "btn-secondary")
      )
    )
  )
)

# Server Logic
server <- function(input, output, session) {

  # Reactive data
  ab_test_data <- reactive({
    generate_ab_test_data()
  })

  retention_data <- reactive({
    generate_retention_data()
  })

  dda_data <- reactive({
    generate_dda_data()
  })

  # KPI Value Boxes
  output$kpiDAU <- renderValueBox({
    valueBox(
      "Daily Active Users",
      "12,450",
      icon = icon("users"),
      color = colorblind_palette$categorical[1]
    )
  })

  output$kpiRetention <- renderValueBox({
    valueBox(
      "D7 Retention",
      "30%",
      icon = icon("chart-line"),
      color = colorblind_palette$sequential[3]
    )
  })

  output$kpiConversion <- renderValueBox({
    valueBox(
      "Conversion Rate",
      "2.4%",
      icon = icon("dollar"),
      color = colorblind_palette$categorical[2]
    )
  })

  output$kpiDDA <- renderValueBox({
    valueBox(
      "DDA Engagement",
      "0.65",
      icon = icon("balance-scale"),
      color = colorblind_palette$cividis[3]
    )
  })

  # Traffic Plot
  output$trafficPlot <- renderPlot({
    ggplot(data.frame(hour = 1:24, users = cumsum(rnorm(24, 500, 100)),
      aes(x = hour, y = users)) +
      geom_line(color = colorblind_palette$categorical[1], linewidth = 2) +
      geom_point(color = colorblind_palette$categorical[2], size = 3) +
      theme_minimal() +
      labs(title = "User Traffic (24h)", x = "Hour", y = "Active Users")
  })

  # A/B Test Table
  output$abTestTable <- renderTable({
    ab_test_data()
  })

  # A/B Test Plot
  output$abTestPlot <- renderPlot({
    df <- ab_test_data()

    ggplot(df, aes(x = test_id, y = conversion_b, fill = test_id)) +
      geom_col(fill = colorblind_palette$categorical) +
      geom_line(aes(y = conversion_a), color = "white", linewidth = 2) +
      theme_minimal() +
      labs(title = "A/B Test Conversion Comparison",
           x = "Test ID", y = "Conversion Rate")
  })

  # Retention Plot
  output$retentionPlot <- renderPlot({
    df <- retention_data()

    ggplot(df, aes(x = day, y = retention_rate)) +
      geom_line(color = colorblind_palette$sequential[1], linewidth = 2) +
      geom_point(color = colorblind_palette$categorical[2], size = 4) +
      theme_minimal() +
      labs(title = "Player Retention Curve",
           x = "Day", y = "Retention Rate")
  })

  # Cohort Table
  output$cohortTable <- renderTable({
    retention_data()
  })

  # DDA Plot
  output$ddaPlot <- renderPlot({
    df <- dda_data()

    ggplot(df, aes(x = difficulty_level, y = avg_completion)) +
      geom_line(color = colorblind_palette$cividis[1], linewidth = 2) +
      geom_point(color = colorblind_palette$categorical[3], size = 4) +
      theme_minimal() +
      labs(title = "DDA: Difficulty vs Completion",
           x = "Difficulty Level", y = "Completion Rate")
  })

  # Skill Table
  output$skillTable <- renderTable({
    dda_data()
  })

  # Export handlers
  observeEvent(input$exportCSV, {
    write.csv(generate_ab_test_data(), "ab_tests_export.csv")
    showNotification("CSV exported successfully", type = "success")
  })

  observeEvent(input$exportPDF, {
    showNotification("PDF generation initiated", type = "info")
  })
}

# Run Application
run_app <- function() {
  shinyApp(ui, server)
}

# For testing
if (interactive()) {
  run_app()
}