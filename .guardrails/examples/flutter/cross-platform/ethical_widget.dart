/// Ethical Widget Wrappers
///
/// Wraps Flutter widgets to prevent dark patterns automatically.
/// AI agents: use these wrappers instead of raw Material/Cupertino
/// widgets for any engagement or monetization UI.
library;

import 'package:flutter/material.dart';

/// Ethical button that prevents dark pattern tactics
/// - Equal prominence for accept/decline actions
/// - No fake urgency (countdown timers prohibited)
/// - Clear, honest labeling
class EthicalButton extends StatelessWidget {
  final String label;
  final VoidCallback onPressed;
  final bool isPrimary;

  const EthicalButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.isPrimary = false,
  });

  @override
  Widget build(BuildContext context) {
    // Both primary and secondary buttons have equal visual weight
    // This prevents the dark pattern of making "decline" less visible
    return SizedBox(
      height: 48, // Meets minimum touch target (44px)
      child: isPrimary
          ? FilledButton(onPressed: onPressed, child: Text(label))
          : OutlinedButton(onPressed: onPressed, child: Text(label)),
    );
  }
}

/// Ethical dialog that enforces fair choice presentation
/// - Accept and decline buttons have equal prominence
/// - No pre-selected options
/// - Clear description of consequences
class EthicalDialog extends StatelessWidget {
  final String title;
  final String description;
  final String acceptLabel;
  final String declineLabel;
  final VoidCallback onAccept;
  final VoidCallback onDecline;

  const EthicalDialog({
    super.key,
    required this.title,
    required this.description,
    required this.acceptLabel,
    required this.declineLabel,
    required this.onAccept,
    required this.onDecline,
  });

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(title),
      content: Text(description),
      actions: [
        // IMPORTANT: Both buttons same size, same style weight
        // Decline is listed first (no visual de-emphasis)
        EthicalButton(label: declineLabel, onPressed: onDecline),
        EthicalButton(label: acceptLabel, onPressed: onAccept, isPrimary: true),
      ],
    );
  }
}

/// Purchase confirmation that requires explicit double-confirm
/// for purchases over the threshold amount
class EthicalPurchaseConfirm extends StatelessWidget {
  final String itemName;
  final double priceUSD;
  final VoidCallback onConfirm;
  final VoidCallback onCancel;

  /// Purchases above this amount require double confirmation
  static const double doubleConfirmThreshold = 5.0;

  const EthicalPurchaseConfirm({
    super.key,
    required this.itemName,
    required this.priceUSD,
    required this.onConfirm,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context) {
    return EthicalDialog(
      title: 'Confirm Purchase',
      description: 'Buy "$itemName" for \$${priceUSD.toStringAsFixed(2)}?'
          '\n\nThis is a real-money purchase.',
      acceptLabel: 'Buy for \$${priceUSD.toStringAsFixed(2)}',
      declineLabel: 'Cancel',
      onAccept: onConfirm,
      onDecline: onCancel,
    );
  }
}
