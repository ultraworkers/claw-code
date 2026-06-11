// Procedural Generation Parameter Tools with DDA Telemetry and Ethics Analysis
// Demonstrates: Procedural generation, DDA telemetry, dark pattern detection, ethical analysis

import scala.util.Random

// Colorblind-safe palette for procedural visualization
object ProceduralPalette {
  val Terrain: Vector[String] = Vector("#440154", "#443782", "#3a608b", "#318755", "#27b57e")
  val Difficulty: Vector[String] = Vector("#0077BD", "#E66302", "#7C87BB", "#D6588B", "#8AB68E")

  def getTerrainColor(height: Double): String = {
    val index = ((height * Terrain.length).toInt).min(Terrain.length - 1)
    Terrain(index)
  }

  def getDifficultyColor(level: Double): String = {
    val index = ((level * Difficulty.length).toInt).min(Difficulty.length - 1)
    Difficulty(index)
  }
}

// Hick's Law compliant parameter menu (5 ± 2 options)
case class ParameterMenu(params: Vector[String]) {
  require(params.length >= 3 && params.length <= 7,
    "Parameter menu must have 3-7 options per Hick's Law")

  def select(index: Int): String = params(index)
  def render: String = params.mkString(" < ")
}

// Procedural generation parameters
case class GenParameters(
  seed: Long,
  complexity: Double,
  biomeType: String,
  resourceDensity: Double,
  enemySpawnRate: Double,
  treasureSpawnRate: Double
) {
  require(complexity >= 0.0 && complexity <= 1.0, "Complexity must be 0.0-1.0")
  require(resourceDensity >= 0.0 && resourceDensity <= 1.0, "Resource density must be 0.0-1.0")

  def validate: Boolean = {
    complexity >= 0.0 && complexity <= 1.0 &&
    resourceDensity >= 0.0 && resourceDensity <= 1.0 &&
    enemySpawnRate >= 0.0 && enemySpawnRate <= 1.0 &&
    treasureSpawnRate >= 0.0 && treasureSpawnRate <= 1.0
  }
}

// Dynamic Difficulty Adjustment (DDA) telemetry
case class DDATelemetry(
  playerId: String,
  currentDifficulty: Double,
  playerSkill: Double,
  successRate: Double,
  failureRate: Double,
  engagementScore: Double,
  timestamp: Long
) {
  require(successRate + failureRate <= 1.0, "Success + failure rate must be <= 1.0")

  def difficultyDelta: Double = currentDifficulty - playerSkill

  def recommendation: String = {
    if (difficultyDelta > 0.3) "REDUCE_DIFFICULTY"
    else if (difficultyDelta < -0.3) "INCREASE_DIFFICULTY"
    else "MAINTAIN_DIFFICULTY"
  }
}

// Dark pattern detection
object DarkPatternDetector {

  // Dark pattern types
  sealed trait DarkPattern
  case object ForcedAction extends DarkPattern     // Mandatory actions for progression
  case object HiddenCost extends DarkPattern       // Costs not disclosed upfront
  case object InfiniteLoop extends DarkPattern     // Engagement loops without exit
  case object False urgency extends DarkPattern     // Artificial time pressure
  case object DataHarvesting extends DarkPattern   // Excessive data collection

  // Ethical analysis scores
  case class EthicsScore(
    autonomy: Double,      // User choice preservation
    transparency: Double,  // Clear information disclosure
    wellbeing: Double,     // User mental health consideration
    fairness: Double       // Balanced monetization
  ) {
    def overall: Double = (autonomy + transparency + wellbeing + fairness) / 4.0

    def status: String = {
      if (overall >= 0.8) "ETHICAL"
      else if (overall >= 0.5) "WARNING"
      else "UNETHICAL"
    }
  }

  // Detect dark patterns in procedural generation
  def analyzePatterns(parameters: GenParameters): Vector[DarkPattern] = {
    val patterns = Vector.newBuilder[DarkPattern]

    // Forced action detection (resource density too low = forced purchases)
    if (parameters.resourceDensity < 0.1) {
      patterns += ForcedAction
    }

    // Hidden cost detection (treasure rate imbalanced)
    if (parameters.treasureSpawnRate < 0.05 && parameters.enemySpawnRate > 0.8) {
      patterns += HiddenCost
    }

    // Infinite loop detection (complexity extreme)
    if (parameters.complexity > 0.95) {
      patterns += InfiniteLoop
    }

    patterns.toVector
  }

  // Calculate ethics score
  def calculateEthicsScore(
    patterns: Vector[DarkPattern],
    ddaData: Vector[DDATelemetry]
  ): EthicsScore = {
    val patternPenalty = patterns.length * 0.15

    val avgEngagement = ddaData.map(_.engagementScore).sum / ddaData.length

    EthicsScore(
      autonomy = 1.0 - patternPenalty,
      transparency = if (patterns.contains(HiddenCost)) 0.5 else 0.9,
      wellbeing = if (patterns.contains(InfiniteLoop)) 0.4 else 0.8,
      fairness = avgEngagement
    )
  }

  def renderEthicsReport(score: EthicsScore, patterns: Vector[DarkPattern]): String = {
    val color = ProceduralPalette.getDifficultyColor(score.overall)

    s"""=== Ethics Audit Report ===
         |  Overall Score: ${score.overall * 100}% [${color}]
         |  Status: ${score.status}
         |
         |  Component Scores:
         |    Autonomy: ${score.autonomy * 100}%
         |    Transparency: ${score.transparency * 100}%
         |    Wellbeing: ${score.wellbeing * 100}%
         |    Fairness: ${score.fairness * 100}%
         |
         |  Dark Patterns Detected: ${patterns.length}
         |  ${patterns.map(_.toString).mkString(", ")}
         |
         |  Recommendation: ${recommendation(score)}""".stripMargin
  }

  private def recommendation(score: EthicsScore): String = {
    if (score.overall >= 0.8) "Maintain ethical design"
    else if (score.overall >= 0.5) "Review dark patterns and adjust"
    else "Immediate ethics audit required - consider redesign"
  }
}

// Procedural world generator
class ProceduralWorldGenerator(seed: Long) {

  def generateBiome(params: GenParameters): String = {
    val random = new Random(seed)

    val terrainHeight = random.nextDouble() * params.complexity
    val resourceCount = (random.nextDouble() * params.resourceDensity * 100).toInt
    val enemyCount = (random.nextDouble() * params.enemySpawnRate * 50).toInt

    s"""Biome Generated:
         |  Seed: ${seed}
         |  Type: ${params.biomeType}
         |  Terrain Height: ${terrainHeight} [${ProceduralPalette.getTerrainColor(terrainHeight)}]
         |  Resources: ${resourceCount}
         |  Enemies: ${enemyCount}
         |  Difficulty Color: ${ProceduralPalette.getDifficultyColor(params.complexity)}""".stripMargin
  }

  // DDA integration - adjust parameters based on player skill
  def adjustForPlayer(ddaData: DDATelemetry, baseParams: GenParameters): GenParameters = {
    val adjustment = ddaData.recommendation

    val newDifficulty = adjustment match {
      case "REDUCE_DIFFICULTY" => baseParams.complexity * 0.8
      case "INCREASE_DIFFICULTY" => baseParams.complexity * 1.2
      case _ => baseParams.complexity
    }

    GenParameters(
      seed = baseParams.seed,
      complexity = newDifficulty.min(1.0),
      biomeType = baseParams.biomeType,
      resourceDensity = if (adjustment == "REDUCE_DIFFICULTY") baseParams.resourceDensity * 1.2 else baseParams.resourceDensity,
      enemySpawnRate = if (adjustment == "INCREASE_DIFFICULTY") baseParams.enemySpawnRate * 1.1 else baseParams.enemySpawnRate,
      treasureSpawnRate = baseParams.treasureSpawnRate
    )
  }
}

// Main entry point
@main def proceduralGenExample(): Unit = {
  val params = GenParameters(
    seed = 123456789L,
    complexity = 0.6,
    biomeType = "forest",
    resourceDensity = 0.4,
    enemySpawnRate = 0.3,
    treasureSpawnRate = 0.2
  )

  val generator = new ProceduralWorldGenerator(params.seed)

  // Parameter menu (Hick's Law: 5 items)
  val paramMenu = ParameterMenu(Vector(
    "Complexity",
    "Biome",
    "Resources",
    "Enemies",
    "Treasures"
  ))

  println(s"Parameter Menu: ${paramMenu.render}")
  println(generator.generateBiome(params))

  // DDA telemetry simulation
  val ddaData = DDATelemetry(
    playerId = "player-001",
    currentDifficulty = 0.7,
    playerSkill = 0.5,
    successRate = 0.4,
    failureRate = 0.3,
    engagementScore = 0.65,
    timestamp = 1234567890L
  )

  println(s"\nDDA Recommendation: ${ddaData.recommendation}")

  // Adjust parameters for player
  val adjustedParams = generator.adjustForPlayer(ddaData, params)
  println(s"\nAdjusted Parameters: complexity=${adjustedParams.complexity}")

  // Ethics audit
  val patterns = DarkPatternDetector.analyzePatterns(params)
  val ethicsScore = DarkPatternDetector.calculateEthicsScore(patterns, Vector(ddaData))

  println("\n" + DarkPatternDetector.renderEthicsReport(ethicsScore, patterns))
}