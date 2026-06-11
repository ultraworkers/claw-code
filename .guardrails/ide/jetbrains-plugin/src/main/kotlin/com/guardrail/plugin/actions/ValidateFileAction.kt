package com.guardrail.plugin.actions

import com.guardrail.plugin.GuardrailService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages

class ValidateFileAction : AnAction() {
    private val logger = Logger.getInstance(ValidateFileAction::class.java)

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
        val service = project.getService(GuardrailService::class.java)

        if (!service.isEnabled()) {
            Messages.showWarningDialog(project, "Guardrail is disabled", "Validation")
            return
        }

        val document = FileDocumentManager.getInstance().getDocument(file) ?: return
        val content = document.text
        val language = file.extension ?: ""
        val filePath = file.path

        Messages.showInfoMessage(project, "Validating...", "Guardrail")

        val result = service.validateFile(filePath, content, language)

        if (result.error != null) {
            Messages.showErrorDialog(project, "Validation failed: ${result.error}", "Guardrail Error")
        } else if (result.violations.isEmpty()) {
            Messages.showInfoMessage(project, "No violations found!", "Guardrail")
        } else {
            val count = result.violations.size
            Messages.showWarningDialog(
                project,
                "Found $count violation(s). Check inspection results for details.",
                "Guardrail"
            )
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE)
        e.presentation.isEnabled = project != null && file != null && !file.isDirectory
    }
}
