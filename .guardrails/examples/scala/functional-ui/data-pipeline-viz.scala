// Apache Spark UI Integration for Game Analytics
// Demonstrates: Real-time data pipelines, colorblind-safe visualization, Hick's Law navigation

import org.apache.spark.sql.SparkSession
import org.apache.spark.sql.functions.*
import org.apache.spark.sql.types.*

// Colorblind-safe visualization palette
object VizPalette {
  // Sequential palette for heatmaps (Viridis)
  val Sequential: Vector[String] = Vector("#440154", "#443782", "#3a608b", "#318755", "#27b57e")

  // Categorical palette for charts (Colorblind-Universal)
  val Categorical: Vector[String] = Vector("#0077BD", "#E66302", "#7C87BB", "#D6588B", "#8AB68E")

  def getColor(value: Double, palette: Vector[String]): String = {
    val index = ((value * palette.length).toInt).min(palette.length - 1)
    palette(index)
  }
}

// Game analytics data schema
object GameAnalyticsSchema {

  val PlayerSessionSchema = StructType(Seq(
    StructField("sessionId", StringType, false),
    StructField("playerId", StringType, false),
    StructField("timestamp", LongType, false),
    StructField("sessionDuration", DoubleType),
    StructField("levelCompleted", IntegerType),
    StructField("difficultyLevel", DoubleType),
    StructField("monetizationEvent", BooleanType)
  ))

  val ABTestSchema = StructType(Seq(
    StructField("testId", StringType, false),
    StructField("variant", StringType, false),
    StructField("playerId", StringType, false),
    StructField("conversion", BooleanType),
    StructField("timestamp", LongType, false)
  ))

  val RetentionSchema = StructType(Seq(
    StructField("playerId", StringType, false),
    StructField("installDate", LongType, false),
    StructField("lastActiveDate", LongType),
    StructField("retentionDay", IntegerType),
    StructField("active", BooleanType)
  ))
}

// Hick's Law compliant navigation (5 ± 2 menu items)
case class NavigationMenu(items: Vector[String]) {
  require(items.length >= 3 && items.length <= 7,
    "Navigation must have 3-7 items per Hick's Law")

  def render: String = {
    items.map(item => s"[${item}]").mkString(" ")
  }
}

// Real-time analytics dashboard
class RealTimeDashboard(spark: SparkSession) {

  // A/B Test Analysis
  def analyzeABTests(testId: String): (String, Double, Double) = {
    val abData = spark.read.schema(GameAnalyticsSchema.ABTestSchema).csv("data/ab_tests")

    val results = abData
      .filter(col("testId") == testId)
      .groupBy("variant")
      .agg(
        count("playerId").as("total"),
        avg("conversion").as("conversion_rate")
      )

    val variantA = results.filter(col("variant") == "A").head()
    val variantB = results.filter(col("variant") == "B").head()

    val improvement = (variantB.getDouble(1) - variantA.getDouble(1)) / variantA.getDouble(1)
    val confidence = calculateConfidence(variantA, variantB)

    (testId, improvement, confidence)
  }

  // Retention Curve Analysis
  def analyzeRetention(days: Int = 7): Vector[(Int, Double)] = {
    val retentionData = spark.read.schema(GameAnalyticsSchema.RetentionSchema).csv("data/retention")

    val retentionByDay = retentionData
      .filter(col("retentionDay") <= days)
      .groupBy("retentionDay")
      .agg(avg("active").as("retention_rate"))
      .orderBy("retentionDay")

    retentionByDay.collect().map(row =>
      (row.getInt(0), row.getDouble(1))
    ).toVector
  }

  // DDA (Dynamic Difficulty Adjustment) Analysis
  def analyzeDDA(): Vector[(String, Double, Double)] = {
    val sessionData = spark.read.schema(GameAnalyticsSchema.PlayerSessionSchema).csv("data/sessions")

    val ddaStats = sessionData
      .groupBy("difficultyLevel")
      .agg(
        avg("sessionDuration").as("avg_duration"),
        avg("levelCompleted").as("avg_completion")
      )
      .orderBy("difficultyLevel")

    ddaStats.collect().map(row =>
      (s"Difficulty ${row.getDouble(0)}",
       row.getDouble(1),
       row.getDouble(2))
    ).toVector
  }

  private def calculateConfidence(
    variantA: org.apache.spark.sql.Row,
    variantB: org.apache.spark.sql.Row
  ): Double = {
    // Simplified confidence calculation
    val meanA = variantA.getDouble(1)
    val meanB = variantB.getDouble(1)
    val nA = variantA.getLong(0)
    val nB = variantB.getLong(0)

    // Z-score approximation
    val zScore = (meanB - meanA) / math.sqrt(meanA / nA + meanB / nB)
    math.min(0.99, math.abs(zScore) / 3.0)
  }

  // Render dashboard with colorblind-safe palette
  def renderDashboard(): String = {
    val navigation = NavigationMenu(Vector(
      "Overview",
      "A/B Tests",
      "Retention",
      "DDA",
      "Export"
    )) // Hick's Law: 5 items

    val abResults = analyzeABTests("checkout-flow")
    val retention = analyzeRetention(7)
    val ddaStats = analyzeDDA()

    val sb = new StringBuilder()
    sb.append(s"=== Game Analytics Dashboard ===\n")
    sb.append(s"Navigation: ${navigation.render}\n\n")

    sb.append(s"A/B Test: ${abResults.1}\n")
    sb.append(s"  Improvement: ${abResults.2 * 100}%\n")
    sb.append(s"  Confidence: ${abResults.3 * 100}%\n")
    sb.append(s"  Color: ${VizPalette.getColor(abResults.3, VizPalette.Categorical)}\n\n")

    sb.append("Retention Curve:\n")
    retention.foreach { (day, rate) =>
      val color = VizPalette.getColor(rate / 1.0, VizPalette.Sequential)
      sb.append(s"  Day ${day}: ${rate * 100}% [${color}]\n")
    }

    sb.append("\nDDA Analysis:\n")
    ddaStats.foreach { (difficulty, duration, completion) =>
      sb.append(s"  ${difficulty}: Duration=${duration}s, Completion=${completion}\n")
    }

    sb.toString
  }
}

// Monetization Transparency Analyzer
object MonetizationAnalyzer {

  case class MonetizationEvent(
    playerId: String,
    productId: String,
    price: Double,
    timestamp: Long,
    isHiddenCost: Boolean
  )

  def analyzeTransparency(events: Vector[MonetizationEvent]): (Double, Int) = {
    val totalEvents = events.length
    val hiddenCosts = events.filter(_.isHiddenCost).length

    val transparencyScore = 1.0 - (hiddenCosts.toDouble / totalEvents)
    (transparencyScore, hiddenCosts)
  }

  def renderTransparencyReport(score: Double, hiddenCount: Int): String = {
    val color = VizPalette.getColor(score, VizPalette.Sequential)
    val status = if (score >= 0.8) "GOOD" else if (score >= 0.5) "WARNING" else "CRITICAL"

    s"""Monetization Transparency Report
         |  Score: ${score * 100}% [${color}]
         |  Status: ${status}
         |  Hidden Costs Found: ${hiddenCount}
         |  Recommendation: ${recommendation(score)}""".stripMargin
  }

  private def recommendation(score: Double): String = {
    if (score >= 0.8) "Maintain current transparency"
    else if (score >= 0.5) "Review pricing disclosure"
    else "Immediate pricing transparency audit required"
  }
}

// Main entry point
@main def dataPipelineViz(): Unit = {
  val spark = SparkSession.builder()
    .appName("GameAnalyticsViz")
    .master("local[*]")
    .getOrCreate()

  val dashboard = new RealTimeDashboard(spark)

  println(dashboard.renderDashboard())

  // Monetization transparency check
  val sampleEvents = Vector(
    MonetizationEvent("p1", "gem-pack", 4.99, 1234567890L, false),
    MonetizationEvent("p2", "vip-pass", 9.99, 1234567891L, false),
    MonetizationEvent("p3", "hidden-fee", 2.99, 1234567892L, true)
  )

  val (score, hiddenCount) = MonetizationAnalyzer.analyzeTransparency(sampleEvents)
  println(MonetizationAnalyzer.renderTransparencyReport(score, hiddenCount))

  spark.stop()
}