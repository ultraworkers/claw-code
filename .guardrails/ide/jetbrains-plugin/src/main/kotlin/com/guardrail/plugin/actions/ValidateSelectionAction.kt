package com.guardrail.plugin.actions

import com.guardrail.plugin.GuardrailService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages

class ValidateSelectionAction : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val service = project.getService(GuardrailService::class.java)

        if (!service.isEnabled()) {
            Messages.showWarningDialog(project, "Guardrail is disabled", "Validation")
            return
        }

        val selectionModel = editor.selectionModel
        if (!selectionModel.hasSelection()) {
            Messages.showWarningDialog(project, "No text selected", "Guardrail")
            return
        }

        val selectedText = selectionModel.selectedText ?: return
        val language = editor.document.language?.id ?: ""

        val result = service.validateSelection(selectedText, language)

        if (result.error != null) {
            Messages.showErrorDialog(project, "Validation failed: ${result.error}", "Guardrail Error")
        } else if (result.violations.isEmpty()) {
            Messages.showInfoMessage(project, "Selection is valid!", "Guardrail")
        } else {
            val count = result.violations.size
            val messages = result.violations.joinToString("\n") { "- ${it.message}" }
            Messages.showWarningDialog(
                project,
                "Found $count violation(s):\n$messages",
                "Guardrail"
            )
        }
    }

    override fun update(e: AnActionEvent) {
        val editor = e.getData(CommonDataKeys.EDITOR)
        e.presentation.isEnabled = editor != null && editor.selectionModel.hasSelection()
    }
}
