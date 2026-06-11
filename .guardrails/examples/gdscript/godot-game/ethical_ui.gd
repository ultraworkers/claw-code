## Ethical UI Patterns for Games
##
## Prevents dark patterns in game UI per ETHICAL_ENGAGEMENT.md.
## AI agents: use these patterns instead of raw Control nodes
## for any engagement or monetization UI.
class_name EthicalUI
extends Node

## Prohibited UI patterns that agents must never generate
const PROHIBITED_PATTERNS: Array[String] = [
	"countdown_purchase",     # Fake urgency timers on purchases
	"disguised_ad",           # Native ads without disclosure
	"hidden_cost",            # Fees revealed only at checkout
	"preselected_purchase",   # Pre-checked paid options
	"guilt_trip_cancel",      # Emotional manipulation on cancel
	"infinite_scroll",        # No natural content boundaries
	"forced_continuity",      # No clear cancellation path
]


## Create an ethical choice dialog with equal button prominence
## Both options have equal visual weight — no dark pattern emphasis
static func create_fair_choice(
	title: String,
	description: String,
	accept_text: String,
	decline_text: String,
	accept_callback: Callable,
	decline_callback: Callable,
) -> ConfirmationDialog:
	var dialog := ConfirmationDialog.new()
	dialog.title = title
	dialog.dialog_text = description
	dialog.ok_button_text = accept_text
	dialog.cancel_button_text = decline_text
	# Both buttons same size — enforced
	dialog.get_ok_button().custom_minimum_size = Vector2(120, 44)
	dialog.get_cancel_button().custom_minimum_size = Vector2(120, 44)
	dialog.confirmed.connect(accept_callback)
	dialog.canceled.connect(decline_callback)
	return dialog


## Validate that a UI element is not a dark pattern
static func validate_element(element_type: String) -> bool:
	if element_type in PROHIBITED_PATTERNS:
		push_error("EthicalUI: BLOCKED — '%s' is a prohibited dark pattern" % element_type)
		return false
	return true


## Create a purchase confirmation with mandatory price display
## Shows real currency amount, requires explicit confirmation
static func create_purchase_confirm(
	item_name: String,
	price_display: String,
	on_confirm: Callable,
	on_cancel: Callable,
) -> ConfirmationDialog:
	return create_fair_choice(
		"Confirm Purchase",
		"Buy '%s' for %s?\n\nThis is a real-money purchase." % [item_name, price_display],
		"Buy for %s" % price_display,
		"Cancel",
		on_confirm,
		on_cancel,
	)


## Display loot box odds — mandatory before any randomized purchase
static func display_drop_rates(drop_table: Dictionary) -> String:
	var display := "Drop Rates:\n"
	for tier in drop_table:
		display += "  %s: %.1f%%\n" % [tier, drop_table[tier] * 100.0]
	return display
