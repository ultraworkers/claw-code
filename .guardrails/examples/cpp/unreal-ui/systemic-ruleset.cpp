/*
 * systemic-ruleset.cpp - Physics/Chemistry Interaction Ruleset
 *
 * Production-ready example demonstrating:
 * - Systemic interaction rulesets for UI
 * - Physics-based UI element behavior
 * - Chemistry metaphor for combo systems
 * - Emergent gameplay through interaction
 * - DDA 3.0 adaptation for systemic rules
 *
 * © 2026 - Agent Guardrails Template
 */

#include "CoreMinimal.h"
#include "PhysicsCore/PhysicsInteraction.h"
#include "ChemicalSystem/ChemicalReaction.h"
#include "CommonUI/CommonUserWidget.h"
#include "DDA/DDAManager.h"
#include "Ethical/EthicalEngagement.h"

namespace AgentGuardrails::UnrealUI
{

/**
 * Systemic Ruleset Manager
 * Physics/chemistry interaction rules for UI elements
 * Emergent gameplay through systemic interactions
 */
class USystemicRulesetManager : public UObject
{
    GENERATED_BODY()

private:
    PhysicsInteraction* physicsInteraction;
    ChemicalReaction* chemicalReaction;
    DDAManager* ddaManager;
    EthicalEngagement* ethicalEngagement;

    TArray<FInteractionRule> interactionRules;

public:
    void Initialize()
    {
        physicsInteraction = PhysicsInteraction::GetInstance();
        chemicalReaction = ChemicalReaction::GetInstance();
        ddaManager = DDAManager::GetInstance();
        ethicalEngagement = EthicalEngagement::GetInstance();

        // Initialize interaction rules
        InitializePhysicsRules();
        InitializeChemicalRules();
    }

    /**
     * Initializes physics-based UI rules
     * UI elements obey physics simulation
     */
    void InitializePhysicsRules()
    {
        // Rule: UI elements have mass
        FInteractionRule massRule;
        massRule.Name = "UIMass";
        massRule.Description = "UI elements have simulated mass";
        massRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->SetMass(1.0f);
            widget->EnableGravity(true);
        };
        interactionRules.Add(massRule);

        // Rule: UI elements collide
        FInteractionRule collisionRule;
        collisionRule.Name = "UICollision";
        collisionRule.Description = "UI elements collide with boundaries";
        collisionRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->EnableCollision(true);
            widget->SetCollisionResponse(ECollisionResponse::Overlap);
        };
        interactionRules.Add(collisionRule);

        // Rule: UI elements bounce
        FInteractionRule bounceRule;
        bounceRule.Name = "UIBounce";
        bounceRule.Description = "UI elements bounce on impact";
        bounceRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->SetBounciness(0.5f);
        };
        interactionRules.Add(bounceRule);

        // Rule: UI elements have friction
        FInteractionRule frictionRule;
        frictionRule.Name = "UIFriction";
        frictionRule.Description = "UI elements experience friction";
        collisionRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->SetFriction(0.3f);
        };
        interactionRules.Add(frictionRule);
    }

    /**
     * Initializes chemistry-based UI rules
     * Chemical reaction metaphors for combos
     */
    void InitializeChemicalRules()
    {
        // Rule: UI elements react (combo system)
        FInteractionRule reactionRule;
        reactionRule.Name = "UIReaction";
        reactionRule.Description = "UI elements react when combined";
        reactionRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->EnableReaction(true);
            widget->SetReactionType(EResponseType::Combo);
        };
        interactionRules.Add(reactionRule);

        // Rule: UI elements catalyze (buff system)
        FInteractionRule catalysisRule;
        catalysisRule.Name = "UI Catalysis";
        catalysisRule.Description = "UI elements catalyze neighboring effects";
        catalysisRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->EnableCatalysis(true);
            widget->SetCatalysisRadius(50.0f);
        };
        interactionRules.Add(catalysisRule);

        // Rule: UI elements bond (group system)
        FInteractionRule bondRule;
        bondRule.Name = "UIBond";
        bondRule.Description = "UI elements bond into groups";
        bondRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->EnableBonding(true);
            widget->SetBondStrength(1.0f);
        };
        interactionRules.Add(bondRule);

        // Rule: UI elements decay (cooldown system)
        FInteractionRule decayRule;
        decayRule.Name = "UIDecay";
        decayRule.Description = "UI elements decay over time";
        decayRule.ApplyFunction = [](UCommonUserWidget* widget) {
            widget->EnableDecay(true);
            widget->SetHalfLife(10.0f); // seconds
        };
        interactionRules.Add(decayRule);
    }

    /**
     * Applies physics rules to UI widget
     * Physics-based UI behavior
     */
    void ApplyPhysicsRules(UCommonUserWidget* widget)
    {
        for (auto& rule : interactionRules)
        {
            if (rule.Category == ERuleCategory::Physics)
            {
                rule.ApplyFunction(widget);
            }
        }
    }

    /**
     * Applies chemical rules to UI widget
     * Chemistry metaphor for combos
     */
    void ApplyChemicalRules(UCommonUserWidget* widget)
    {
        for (auto& rule : interactionRules)
        {
            if (rule.Category == ERuleCategory::Chemical)
            {
                rule.ApplyFunction(widget);
            }
        }
    }

    /**
     * Triggers emergent gameplay event
     * Systemic interaction creates emergent behavior
     */
    void TriggerEmergentEvent(FString eventName)
    {
        // Emergent gameplay: physics + chemistry interaction
        auto* eventWidget = CreateEventWidget(eventName);

        // Apply physics rules
        ApplyPhysicsRules(eventWidget);

        // Apply chemical rules
        ApplyChemicalRules(eventWidget);

        // Notify ethical engagement
        ethicalEngagement->OnEmergentEvent(eventName);
    }

    /**
     * Creates event widget for emergent gameplay
     */
    UCommonUserWidget* CreateEventWidget(const FString& eventName)
    {
        auto* widget = NewObject<UCommonUserWidget>();
        widget->SetName(eventName);

        // Enable physics interaction
        widget->EnablePhysics(true);

        // Enable chemical reaction
        widget->EnableReaction(true);

        return widget;
    }

    /**
     * Updates systemic rules based on DDA tier
     * Reduces complexity for stressed players
     */
    void UpdateForDDATier(DDAManager::EDifficultyTier tier)
    {
        switch (tier)
        {
            case DDAManager::EDifficultyTier::Difficult:
                // Stressed players: simplify physics
                SetPhysicsIntensity(0.5f);
                SetReactionRate(0.3f);
                break;

            case DDAManager::EDifficultyTier::Normal:
                // Standard intensity
                SetPhysicsIntensity(1.0f);
                SetReactionRate(1.0f);
                break;

            case DDAManager::EDifficultyTier::Relaxed:
                // Engaged players: enhanced systemic
                SetPhysicsIntensity(1.2f);
                SetReactionRate(1.5f);
                break;
        }
    }

    /**
     * Sets physics simulation intensity
     */
    void SetPhysicsIntensity(float intensity)
    {
        physicsInteraction->SetIntensity(intensity);
    }

    /**
     * Sets chemical reaction rate
     */
    void SetReactionRate(float rate)
    {
        chemicalReaction->SetRate(rate);
    }
};

/**
 * Interaction rule structure
 * Defines systemic behavior for UI elements
 */
USTRUCT()
struct FInteractionRule
{
    GENERATED_BODY()

    FString Name;                 // Rule name
    FString Description;          // Rule description
    ERuleCategory Category;       // Physics or Chemical
    TFunction<void(UCommonUserWidget*)> ApplyFunction; // Apply function
};

/**
 * Rule category enumeration
 */
UENUM()
enum class ERuleCategory
{
    Physics,     // Physics-based rules
    Chemical     // Chemistry-based rules
};

/**
 * Reaction type enumeration
 */
UENUM()
enum class EReactionType
{
    None,           // No reaction
    Combo,          // Combo reaction
    Buff,           | Enhancement reaction
    Debuff,         // Weakening reaction
    Transform       // Transformation reaction
};

/**
 * Physics interaction component
 */
USTRUCT()
struct FPhysicsComponent
{
    GENERATED_BODY()

    float Mass;             // Simulated mass
    float Velocity;         // Current velocity
    float Acceleration;     // Current acceleration
    bool GravityEnabled;    // Gravity enabled
    bool CollisionEnabled;  // Collision enabled
    float Bounciness;       // Bounce factor
    float Friction;         // Friction coefficient
};

/**
 * Chemical reaction component
 */
USTRUCT()
struct FChemicalComponent
{
    GENERATED_BODY()

    bool ReactionEnabled;   // Reaction enabled
    EReactionType Type;     // Reaction type
    float ReactionRate;     // Reaction speed
    float CatalysisRadius;  // Catalysis area
    bool BondingEnabled;    // Bonding enabled
    float BondStrength;     // Bond strength
    bool DecayEnabled;      // Decay enabled
    float HalfLife;         // Decay half-life (seconds)
};

/**
 * Emergent gameplay controller
 * Systemic interactions create emergent behavior
 */
class UEmergentGameplayController : public UObject
{
    GENERATED_BODY()

private:
    USystemicRulesetManager* rulesetManager;

public:
    void Initialize()
    {
        rulesetManager = USystemicRulesetManager::GetInstance();
    }

    /**
     * On systemic interaction detected
     * Triggers emergent gameplay event
     */
    void OnSystemicInteraction(UCommonUserWidget* widgetA, UCommonUserWidget* widgetB)
    {
        // Check for combo reaction
        if (widgetA->GetReactionType() == EReactionType::Combo &&
            widgetB->GetReactionType() == EReactionType::Combo)
        {
            // Emergent gameplay: combo triggered
            rulesetManager->TriggerEmergentEvent("ComboTriggered");
        }

        // Check for catalysis
        if (widgetA->IsCatalysisEnabled() &&
            widgetB->IsWithinRadius(widgetA->GetCatalysisRadius()))
        {
            // Emergent gameplay: buff applied
            rulesetManager->TriggerEmergentEvent("BuffApplied");
        }

        // Check for bonding
        if (widgetA->IsBondingEnabled() &&
            widgetB->IsBondingEnabled())
        {
            // Emergent gameplay: group formed
            rulesetManager->TriggerEmergentEvent("GroupFormed");
        }
    }
};

/**
 * Physics/chemistry utilities
 */
namespace SystemicUtils
{
    /**
     * Calculates physics force on UI element
     */
    FVector CalculatePhysicsForce(FPhysicsComponent& physics)
    {
        return FVector(
            physics.Mass * physics.Acceleration,
            physics.Mass * physics.GravityEnabled ? -9.8f : 0.0f,
            0.0f
        );
    }

    /**
     | Calculates reaction rate
     */
    float CalculateReactionRate(FChemicalComponent& chemical)
    {
        return chemical.ReactionRate * chemical.CatalysisRadius;
    }

    /**
     * Checks if two widgets can react
     */
    bool CanReact(UCommonUserWidget* widgetA, UCommonUserWidget* widgetB)
    {
        return widgetA->IsReactionEnabled() &&
               widgetB->IsReactionEnabled() &&
               widgetA->IsWithinInteractionRange(widgetB);
    }

    /**
     * Triggers systemic interaction
     */
    void TriggerInteraction(UCommonUserWidget* widgetA, UCommonUserWidget* widgetB)
    {
        if (CanReact(widgetA, widgetB))
        {
            auto* eventWidget = USystemicRulesetManager::GetInstance()->CreateEventWidget("SystemicInteraction");
            eventWidget->AddChild(widgetA);
            eventWidget->AddChild(widgetB);
        }
    }
}

} // namespace AgentGuardrails::UnrealUI