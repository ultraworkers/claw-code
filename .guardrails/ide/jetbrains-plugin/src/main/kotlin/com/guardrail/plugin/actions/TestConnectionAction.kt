package com.guardrail.plugin.actions

import com.guardrail.plugin.GuardrailService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages

class TestConnectionAction : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val service = project.getService(GuardrailService::class.java)

        if (!service.isEnabled()) {
            Messages.showWarningDialog(project, "Guardrail is disabled", "Connection Test")
            return
        }

        val connected = service.testConnection()

        if (connected) {
            Messages.showInfoMessage(project, "Successfully connected to Guardrail server!", "Connection Test")
        } else {
            Messages.showErrorDialog(
                project,
                "Failed to connect to Guardrail server.\n\n" +
                "Please check:\n" +
                "- Server URL is correct\n" +
                "- API key is valid\n" +
                "- Server is running",
                "Connection Test Failed"
            )
        }
    }

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project != null
    }
}
