// Scala 3.4+ Functional UI Composition with Type-Safe CSS
// Demonstrates: Functional widget builders, type-safe styling, Hick's Law menus

import scala.scalajs.js
import scala.scalajs.js.annotation.*

// Colorblind-safe palette definitions
object ColorblindPalette {
  // Viridis (sequential, perceptually uniform)
  val Viridis: Vector[String] = Vector("#440154", "#443782", "#3a608b", "#318755", "#27b57e")

  // Colorblind-Universal (categorical, deuteranopia-safe)
  val Universal: Vector[String] = Vector("#0077BD", "#E66302", "#7C87BB", "#D6588B", "#8AB68E")

  // Cividis (colorvision deficiency optimized)
  val Cividis: Vector[String] = Vector("#002495", "#005CA8", "#297EA7", "#58999D", "#8AB68E")

  def validatePalette(name: String, colors: Vector[String]): Boolean = {
    colors.length >= 3 && colors.length <= 7 // Hick's Law: 5 ± 2
  }
}

// Type-safe CSS style system
trait CSSStyle {
  def backgroundColor: String
  def color: String
  def fontSize: String
  def padding: String
  def hoverEffect: Option[String]
}

case class ButtonStyle(
  backgroundColor: String,
  color: String,
  fontSize: String = "14px",
  padding: String = "8px 16px",
  hoverEffect: Option[String] = Some("brightness(1.1)")
) extends CSSStyle

case class MenuStyle(
  backgroundColor: String,
  color: String,
  fontSize: String = "16px",
  padding: String = "12px 24px",
  hoverEffect: Option[String] = Some("underline")
) extends CSSStyle

// Hick's Law compliant menu (5 ± 2 items)
case class Menu(items: Vector[String]) {
  require(items.length >= 3 && items.length <= 7,
    "Menu must have 3-7 items per Hick's Law")

  def render: String = items.mkString(" | ")
}

// Functional UI builder
object FunctionalUI {

  def button(text: String, style: ButtonStyle): String = {
    s"""<button class="btn"
         style="background-color: ${style.backgroundColor};
                color: ${style.color};
                font-size: ${style.fontSize};
                padding: ${style.padding}">
         ${text}
         </button>"""
  }

  def menu(title: String, items: Vector[String], style: MenuStyle): String = {
    val menu = Menu(items) // Validates Hick's Law
    s"""<div class="menu"
         style="background-color: ${style.backgroundColor};
                color: ${style.color}">
         <h3>${title}</h3>
         ${menu.render}
         </div>"""
  }

  def analyticsCard(
    title: String,
    metric: Double,
    palette: Vector[String],
    index: Int
  ): String = {
    val color = palette(index % palette.length)
    s"""<div class="card" style="border-left: 4px ${color} solid">
         <h4>${title}</h4>
         <p class="metric">${metric.toString}</p>
         </div>"""
  }
}

// Game Analytics Dashboard Example
object GameAnalyticsDashboard {

  // A/B Test data structure
  case class ABTestResult(
    testId: String,
    variantA: Int,
    variantB: Int,
    conversionRate: Double,
    confidence: Double
  )

  // Retention curve data
  case class RetentionData(
    day: Int,
    retainedPlayers: Int,
    totalPlayers: Int,
    retentionRate: Double
  )

  // DDA (Dynamic Difficulty Adjustment) telemetry
  case class DDA telemetry(
    playerId: String,
    challengeId: String,
    difficultyLevel: Double,
    playerSkill: Double,
    successRate: Double,
    timestamp: Long
  )

  def renderABTestDashboard(tests: Vector[ABTestResult]): String = {
    val palette = ColorblindPalette.Universal

    tests.map { test =>
      FunctionalUI.analyticsCard(
        title = s"A/B Test: ${test.testId}",
        metric = test.conversionRate,
        palette = palette,
        index = 0
      )
    }.mkString("\n")
  }

  def renderRetentionCurve(retentionData: Vector[RetentionData]): String = {
    val palette = ColorblindPalette.Viridis

    retentionData.map { data =>
      FunctionalUI.analyticsCard(
        title = s"Day ${data.day} Retention",
        metric = data.retentionRate,
        palette = palette,
        index = data.day
      )
    }.mkString("\n")
  }

  def renderDDA Telemetry(ddaData: Vector[DDATelemetry]): String = {
    val palette = ColorblindPalette.Cividis

    ddaData.map { telemetry =>
      val difficultyDelta = telemetry.difficultyLevel - telemetry.playerSkill
      FunctionalUI.analyticsCard(
        title = s"DDA: ${telemetry.challengeId}",
        metric = difficultyDelta,
        palette = palette,
        index = telemetry.playerId.hashCode % palette.length
      )
    }.mkString("\n")
  }
}

// Main application entry point
@main def uiExample(): Unit = {
  val buttonStyle = ButtonStyle(
    backgroundColor = ColorblindPalette.Universal(0),
    color = "#FFFFFF"
  )

  val menuStyle = MenuStyle(
    backgroundColor = "#1a1a2e",
    color = "#eeeef2"
  )

  val menuItems = Vector(
    "Dashboard",
    "Analytics",
    "Reports",
    "Settings",
    "Help"
  ) // Hick's Law: 5 items

  println(FunctionalUI.button("Start Game", buttonStyle))
  println(FunctionalUI.menu("Main Menu", menuItems, menuStyle))

  // Analytics dashboard
  val abTests = Vector(
    ABTestResult("checkout-flow", 1200, 1450, 0.24, 0.95),
    ABTestResult("pricing-display", 800, 920, 0.18, 0.88)
  )

  println(GameAnalyticsDashboard.renderABTestDashboard(abTests))
}